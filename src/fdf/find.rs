use super::options::{ExtensionGroupingOption, Options};
use super::output::{FindStats, HashStats};
use hashbrown::HashMap;
use humansize::{file_size_opts, FileSize};
use indicatif::ProgressBar;
use std::path::Path;
use string_cache::DefaultAtom as Atom;
use walkdir::{DirEntry, WalkDir};

#[derive(Clone, Debug)]
pub struct AugDirEntry {
    pub dir_entry: DirEntry,
    pub size: u64,
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

fn group_key(options: &Options, dent: &AugDirEntry) -> GroupKey {
    let size = dent.size;
    let extension = match dent.path().extension() {
        Some(ps) => Atom::from(ps.to_str().unwrap().to_lowercase()),
        None => match options.extension_grouping {
            ExtensionGroupingOption::FullName => {
                Atom::from(dent.path().file_name().unwrap().to_str().unwrap())
            }
            ExtensionGroupingOption::SingleGroup => Atom::from("<no extension>"),
        },
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
        n_files,
        n_bytes,
        n_groups: by_key.len() as u64,
    }
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
    let mut n_dirs: u64 = 0;
    let mut n_files: u64 = 0;
    let mut n_bytes: u64 = 0;
    prog.set_draw_delta(100);
    let by_key_and_paths = options
        .directories
        .iter()
        .map(|dir| {
            let mut by_key_and_path: KeyToStringToDentMap = HashMap::new();
            let walker = WalkDir::new(dir).into_iter();
            for er in walker.filter_entry(|entry| options.is_entry_included(&entry)) {
                let entry = match er {
                    Ok(entry) => entry,
                    Err(err) => {
                        eprintln!("[!] {}", err);
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
                if size == 0 {
                    continue;
                }
                n_files += 1;
                n_bytes += size;
                if options.verbosity >= 3 {
                    println!("{}", entry.path().display());
                }
                let path_str = entry.path().to_str().unwrap().to_string();
                let aug_entry = AugDirEntry {
                    dir_entry: entry,
                    size,
                };
                let key = group_key(&options, &aug_entry);
                let by_path = by_key_and_path.entry(key).or_insert_with(HashMap::new);
                by_path.insert(path_str, aug_entry);
                prog.set_message(
                    format!(
                        "{} dirs, {} files, {}...",
                        n_dirs,
                        n_files,
                        n_bytes.file_size(file_size_opts::CONVENTIONAL).unwrap()
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
    let find_stats = FindStats {
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
    prog.set_message("Calculating statistics...");
    let stats = calculate_hash_stats(&by_key);
    prog.finish_and_clear();
    (
        find_stats,
        stats,
        by_key,
        if return_precull {
            Some(by_key_and_path)
        } else {
            None
        },
    )
}
