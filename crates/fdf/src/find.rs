use crate::interrupt::{check_and_reset_interrupt, is_interrupted};
use crate::options::{NameGroupingOption, Options};
use crate::output::{FindStats, HashStats};
use crate::progress::{ProgressCallback, ProgressEvent};
use std::collections::HashMap;
use std::path::Path;
use string_cache::DefaultAtom as Atom;
use tracing::{debug, error};
use walkdir::{DirEntry, WalkDir};

#[derive(Clone, Debug)]
pub struct AugDirEntry {
    pub dir_entry: DirEntry,
    pub tag_index: u8,
    pub size: u64,
}

impl AugDirEntry {
    pub fn path(&self) -> &Path {
        self.dir_entry.path()
    }
}

#[derive(Clone, Eq, PartialEq, Hash, Debug)]
pub struct GroupKey {
    pub size: u64,
    pub extension: Atom,
}

fn group_key(options: &Options, dent: &AugDirEntry) -> GroupKey {
    let size = dent.size;
    let ex = dent.path().extension();
    let ng = &options.name_grouping;
    let extension = match (ng, ex) {
        (NameGroupingOption::IgnoreName, _) => Atom::from("<none>"),
        (NameGroupingOption::FullNameWhenNoExtension, None) => {
            Atom::from(dent.path().file_name().unwrap().to_str().unwrap())
        }
        (NameGroupingOption::SingleGroupWhenNoExtension, None) => Atom::from("<none>"),
        (_, Some(ps)) => Atom::from(ps.to_str().unwrap().to_lowercase()),
    };
    GroupKey { size, extension }
}

type StringToDentMap = HashMap<String, AugDirEntry>;
pub type KeyToStringToDentMap = HashMap<GroupKey, StringToDentMap>;
pub type KeyToDentsMap = HashMap<GroupKey, Vec<AugDirEntry>>;

fn calculate_hash_stats(by_key: &KeyToDentsMap) -> HashStats {
    let (n_files, n_bytes) = by_key
        .values()
        .fold((0u64, 0u64), |(n_files, total_size), dents| {
            (
                n_files + dents.len() as u64,
                total_size + dents.iter().fold(0u64, |acc, dent| acc + dent.size),
            )
        });
    HashStats {
        interrupted: false,
        n_files,
        n_bytes,
        n_groups: by_key.len() as u64,
    }
}

pub fn find_files(
    options: &Options,
    return_precull: bool,
    on_progress: ProgressCallback,
) -> (
    FindStats,
    HashStats,
    KeyToDentsMap,
    Option<KeyToStringToDentMap>,
) {
    on_progress(ProgressEvent::FindStarted);
    let mut n_dirs: u64 = 0;
    let mut n_files: u64 = 0;
    let mut n_bytes: u64 = 0;
    let mut by_key_and_path: KeyToStringToDentMap = HashMap::new();
    for ds in options.directories.iter() {
        let walker = WalkDir::new(&ds.path).into_iter();
        for er in walker.filter_entry(|entry| options.is_entry_included(entry)) {
            if is_interrupted() {
                break;
            }
            let entry = match er {
                Ok(entry) => entry,
                Err(err) => {
                    error!("[!] {}", err);
                    continue;
                }
            };
            if entry.file_type().is_dir() {
                n_dirs += 1;
                continue;
            }
            // TODO: Process symlinks gracefully
            if entry.file_type().is_symlink() {
                continue;
            }
            let size = entry.metadata().unwrap().len();
            if size == 0 || size < options.min_size || size > options.max_size {
                continue;
            }
            n_files += 1;
            n_bytes += size;
            if options.verbosity >= 3 {
                debug!("Found: {}", entry.path().display());
            }
            let path_str = entry.path().to_str().unwrap().to_string();
            let aug_entry = AugDirEntry {
                dir_entry: entry,
                size,
                tag_index: ds.tag_index,
            };
            let key = group_key(options, &aug_entry);
            let by_path = by_key_and_path.entry(key).or_default();
            by_path.insert(path_str, aug_entry);
            on_progress(ProgressEvent::FindProgress {
                n_dirs,
                n_files,
                n_bytes,
            });
        }
    }
    let mut by_key: KeyToDentsMap = HashMap::new();
    let find_stats = FindStats {
        interrupted: check_and_reset_interrupt(),
        n_bytes,
        n_dirs,
        n_files,
        n_precull_groups: by_key_and_path.len() as u64,
    };
    for (key, ent_map) in &by_key_and_path {
        if ent_map.len() > 1 {
            by_key.insert(key.clone(), ent_map.values().cloned().collect());
        }
    }
    on_progress(ProgressEvent::CalculatingStats { n_files });
    let hash_stats = calculate_hash_stats(&by_key);
    on_progress(ProgressEvent::FindFinished);
    (
        find_stats,
        hash_stats,
        by_key,
        if return_precull {
            Some(by_key_and_path)
        } else {
            None
        },
    )
}
