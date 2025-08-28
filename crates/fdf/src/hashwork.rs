use crate::hash::hash_file;
use crate::interrupt::is_interrupted;
use crate::output::TaggedPath;
use crate::{AugDirEntry, GroupKey, HashGroupResult};
use crate::{KeyGroupResult, KeyToDentsMap, Options};
use anyhow::Context;
use bit_set::BitSet;
use crossbeam::channel::{unbounded, Receiver, Sender};
use indicatif::{ProgressBar, ProgressStyle};
use std::collections::HashMap;
use std::sync::Arc;
use std::thread;

#[derive(Debug)]
pub enum Job {
    HashFile { key: GroupKey, dent: AugDirEntry },
    Shutdown,
}

#[derive(Debug)]
pub struct HashResult {
    pub key: GroupKey,
    pub dent: AugDirEntry,
    pub result: Result<String, String>,
}

pub struct WorkerParams {
    pub worker_id: usize,
    pub job_receiver: Receiver<Job>,
    pub result_sender: Sender<HashResult>,
    pub options: Arc<Options>,
}

pub fn hash_worker(params: WorkerParams) {
    let WorkerParams {
        worker_id,
        job_receiver,
        result_sender,
        options,
    } = params;

    loop {
        match job_receiver.recv() {
            Ok(Job::HashFile { key, dent }) => {
                let result = match hash_file(&key, &dent, &options) {
                    Ok((_, hash)) => Ok(hash),
                    Err(e) => {
                        tracing::error!(
                            "Worker {worker_id}: Unable to hash {:?}: {e}",
                            dent.path()
                        );
                        Err(e.to_string())
                    }
                };

                if result_sender
                    .send(HashResult { key, dent, result })
                    .is_err()
                {
                    tracing::error!("Worker {}: Unable to send result", worker_id);
                    return;
                }

                if is_interrupted() {
                    tracing::debug!("Worker {}: Interrupt noticed.", worker_id);
                    return;
                }
            }
            Ok(Job::Shutdown) => {
                break;
            }
            Err(_) => {
                tracing::debug!("Worker {}: Job channel has been closed.", worker_id);
                return;
            }
        }
    }
}

pub fn do_hash(
    core_options: Options,
    by_key: KeyToDentsMap,
) -> anyhow::Result<Vec<KeyGroupResult>> {
    let mut sorted_pairs = by_key
        .into_iter()
        .collect::<Vec<(GroupKey, Vec<AugDirEntry>)>>();
    sorted_pairs.sort_unstable_by(|(ka, _), (kb, _)| kb.size.cmp(&ka.size));

    // Count total files for progress bar
    let total_files: usize = sorted_pairs.iter().map(|(_, dents)| dents.len()).sum();
    let prog = ProgressBar::new(total_files as u64);
    prog.set_style(
        ProgressStyle::default_bar().template("{pos:>6}/{len:6} {msg} (ETA {eta}) {wide_bar}")?,
    );

    let shared_options = Arc::new(core_options);
    let (job_sender, job_receiver) = unbounded();
    let (result_sender, result_receiver) = unbounded();
    let num_workers = shared_options.file_hash_threads;

    tracing::info!("Starting {num_workers} worker threads to process {total_files} files");

    let mut handles = Vec::new();
    for worker_id in 0..num_workers {
        let worker_params = WorkerParams {
            worker_id,
            job_receiver: job_receiver.clone(),
            result_sender: result_sender.clone(),
            options: Arc::clone(&shared_options),
        };
        let handle = thread::spawn(move || hash_worker(worker_params));
        handles.push(handle);
    }
    drop(result_sender); // We don't need this one in the main thread

    let mut group_expected_counts: HashMap<GroupKey, u64> = HashMap::new();
    for (key, dents) in &sorted_pairs {
        group_expected_counts.insert((*key).clone(), dents.len() as u64);
    }
    let total_groups = group_expected_counts.len();

    // Producer: Send jobs in priority order (largest groups first)
    for (key, dents) in sorted_pairs {
        for dent in dents {
            if is_interrupted() {
                break;
            }
            job_sender
                .send(Job::HashFile {
                    key: key.clone(),
                    dent,
                })
                .context("Failed to send job")?;
        }
    }

    for _ in 0..num_workers {
        let _ = job_sender.send(Job::Shutdown);
    }

    let mut results: HashMap<GroupKey, Vec<(AugDirEntry, Result<String, String>)>> = HashMap::new();
    let mut processed_files = 0;
    let mut completed_groups = 0;

    loop {
        match result_receiver.recv() {
            Ok(hash_result) => {
                let HashResult { key, dent, result } = hash_result;
                results.entry(key.clone()).or_default().push((dent, result));
                processed_files += 1;

                // Check if this group is now complete
                let group_current_count = results.get(&key).unwrap().len() as u64;
                let group_expected_count = group_expected_counts.get(&key).unwrap();

                if group_current_count == *group_expected_count {
                    completed_groups += 1;
                }

                prog.set_position(processed_files as u64);
                prog.set_message(format!("{completed_groups}/{total_groups} groups complete",));

                if is_interrupted() {
                    break;
                }
            }
            Err(_) => {
                // All workers done (they've all dropped their senders)
                break;
            }
        }
    }

    for handle in handles {
        if let Err(e) = handle.join() {
            tracing::error!("Worker thread panicked: {:?}", e);
        }
    }

    prog.finish();

    let mut key_group_results = Vec::new();

    for (key, file_hashes) in results.into_iter() {
        // Consume results into hash groups
        let mut hash_to_group: HashMap<String, Vec<TaggedPath>> = HashMap::new();
        let mut tag_indices_seen = BitSet::new();
        let mut n_files = 0;
        let mut n_errors = 0;

        for (dent, result) in file_hashes {
            match result {
                Ok(hash) => {
                    let tagged_path = TaggedPath {
                        tag_index: dent.tag_index,
                        path: dent.dir_entry.path().to_str().unwrap().to_string(),
                    };
                    tag_indices_seen.insert(dent.tag_index as usize);
                    hash_to_group.entry(hash).or_default().push(tagged_path);
                    n_files += 1;
                }
                Err(_) => {
                    n_errors += 1;
                }
            }
        }

        let cross_tag = tag_indices_seen.len() > 1;
        let mut hash_groups = Vec::with_capacity(hash_to_group.len());

        for (hash, mut files) in hash_to_group {
            files.sort_unstable_by(|a, b| {
                if a.tag_index == b.tag_index {
                    a.path.cmp(&b.path)
                } else {
                    a.tag_index.cmp(&b.tag_index)
                }
            });

            hash_groups.push(HashGroupResult { hash, files });
        }

        let result = KeyGroupResult {
            size: key.size,
            identifier: key.extension.to_string(),
            hash_groups,
            n_files,
            n_errors,
            complete: n_files + n_errors == *group_expected_counts.get(&key).unwrap(),
            cross_tag,
        };
        key_group_results.push(result);
    }
    key_group_results.sort_unstable_by(|ka, kb| kb.size.cmp(&ka.size));

    Ok(key_group_results)
}
