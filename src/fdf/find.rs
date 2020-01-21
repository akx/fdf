use super::options::Options;
use super::output::{FindStats, HashStats};
use hashbrown::HashMap;
use humansize::{file_size_opts, FileSize};
use indicatif::ProgressBar;
use std::fs;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use string_cache::DefaultAtom as Atom;
use walkdir::{DirEntry, WalkDir};

#[derive(Clone, Debug)]
pub struct AugDirEntry {
    pub dir_entry: DirEntry,
    pub metadata: fs::Metadata,
}

impl AugDirEntry {
    pub fn path(&self) -> &Path {
        &self.dir_entry.path()
    }
}

#[derive(Clone, Eq, PartialEq, Hash, Debug)]
pub struct GroupKey {
    pub size: u64,
    pub extension: Atom,
}

fn group_key(dent: &AugDirEntry) -> GroupKey {
    let size = dent.metadata.len();
    let extension = Atom::from(match dent.path().extension() {
        Some(ps) => ps.to_str().unwrap(),
        None => dent.path().file_name().unwrap().to_str().unwrap(),
    });
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
                total_size
                    + dents
                        .iter()
                        .fold(0u64, |acc, dent| acc + dent.metadata.len()),
            )
        });
    HashStats {
        n_files,
        n_bytes,
        n_groups: by_key.len() as u64,
    }
}

fn process_entry(
    options: &Options,
    by_key_and_path: &mut KeyToStringToDentMap,
    entry_pair: EntryPair,
) -> FindStats {
    let (dir_entry, pre_metadata) = entry_pair;
    // TODO: Process symlinks gracefully
    if dir_entry.file_type().is_dir() || dir_entry.file_type().is_symlink() {
        return FindStats::zero();
    }
    let metadata = match pre_metadata {
        None => dir_entry.metadata().unwrap(),
        Some(m) => m,
    };
    let size = metadata.len();
    if size == 0 {
        return FindStats::zero();
    }
    if options.verbosity >= 3 {
        println!("{}", dir_entry.path().display());
    }
    let path_str = dir_entry.path().to_str().unwrap().to_string();
    let aug_entry = AugDirEntry {
        dir_entry,
        metadata,
    };
    by_key_and_path
        .entry(group_key(&aug_entry))
        .or_insert_with(HashMap::new)
        .insert(path_str, aug_entry);
    FindStats::file_of_size(size)
}

fn dir_name_to_entries(options: &Options, dir: &String) -> impl Iterator<Item = EntryPair> {
    return WalkDir::new(dir)
        .into_iter()
        .filter_entry(|entry| options.is_entry_included(&entry))
        // TODO: use metadata from the entry on Windows here
        .map(|entry| (entry.unwrap(), None))
        .into_iter();
}

pub fn find_files(
    options: &Options,
    return_precull: bool,
) -> (
    FindStats,
    HashStats,
    KeyToDentsMap,
    Option<KeyToStringToDentMap>,
) {
    let prog = ProgressBar::new_spinner();
    let mut find_stats = FindStats::zero();
    prog.set_draw_delta(100);

    let dir_dent_iterators = options
        .directories
        .iter()
        .map(|dir| dir_name_to_entries(options, dir));

    //    let filelist_dent_iterators = options
    //        .file_lists
    //        .iter()
    //        .map(|filename: &String| file_list_to_entries(options, filename));

    let by_key_and_paths = dir_dent_iterators
        //        .chain(filelist_dent_iterators)
        .map(|entry_iter| {
            let mut by_key_and_path: KeyToStringToDentMap = HashMap::new();
            for er in entry_iter {
                let pr = process_entry(options, &mut by_key_and_path, er);
                find_stats.accumulate(&pr);
                prog.set_message(
                    format!(
                        "{} dirs, {} files, {}...",
                        find_stats.n_dirs,
                        find_stats.n_files,
                        find_stats
                            .n_bytes
                            .file_size(file_size_opts::CONVENTIONAL)
                            .unwrap()
                    )
                    .as_str(),
                );
                prog.inc(1);
            }
            by_key_and_path
        })
        .collect::<Vec<KeyToStringToDentMap>>();
    prog.set_draw_delta(0);
    prog.set_message("Merging and regrouping...");
    // merge per-directory maps into one
    let by_key_and_path: KeyToStringToDentMap =
        by_key_and_paths
            .into_iter()
            .fold(HashMap::new(), |mut accmap, map| {
                for (key, ents) in map {
                    accmap.entry(key).or_insert_with(HashMap::new).extend(ents);
                }
                accmap
            });
    let mut by_key: KeyToDentsMap = HashMap::new();
    find_stats.n_precull_groups = by_key_and_path.len() as u64;
    for (key, ent_map) in &by_key_and_path {
        if ent_map.len() > 1 {
            by_key.insert(key.clone(), ent_map.values().cloned().collect());
        }
    }
    prog.set_message("Calculating statistics...");
    let stats = calculate_hash_stats(&by_key);
    prog.finish_and_clear();
    (
        find_stats,
        stats,
        by_key,
        match return_precull {
            true => Some(by_key_and_path),
            false => None,
        },
    )
}
