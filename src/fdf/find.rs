use super::options::Options;
use super::output::{FindStats, HashStats};
use hashbrown::HashMap;
use humansize::{file_size_opts, FileSize};
use indicatif::ProgressBar;
use string_cache::DefaultAtom as Atom;
use walkdir::{DirEntry, WalkDir};

#[derive(Eq, PartialEq, Hash, Debug)]
pub struct GroupKey {
    pub size: u64,
    pub extension: Atom,
}

fn group_key(dent: &DirEntry) -> GroupKey {
    let size = match dent.metadata() {
        Ok(s) => s.len(),
        Err(_) => 0,
    };
    let extension = Atom::from(match dent.path().extension() {
        Some(ps) => ps.to_str().unwrap(),
        None => dent.path().file_name().unwrap().to_str().unwrap(),
    });
    return GroupKey {
        size: size,
        extension: extension,
    };
}

type StringToDentMap = HashMap<String, DirEntry>;
type KeyToStringToDentMap = HashMap<GroupKey, StringToDentMap>;
pub type KeyToDentsMap = HashMap<GroupKey, Vec<DirEntry>>;

fn calculate_hash_stats(by_key: &KeyToDentsMap) -> HashStats {
    let (n_files, n_bytes) = by_key
        .values()
        .fold((0u64, 0u64), |(n_files, total_size), dents| {
            (
                n_files + dents.len() as u64,
                total_size
                    + dents
                        .iter()
                        .fold(0u64, |acc, dent| acc + dent.metadata().unwrap().len()),
            )
        });
    return HashStats {
        n_files,
        n_bytes,
        n_groups: by_key.len() as u64,
    };
}

pub fn find_files(options: &Options) -> (FindStats, HashStats, KeyToDentsMap) {
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
                let entry = er.unwrap();
                if entry.file_type().is_dir() {
                    n_dirs += 1;
                    continue;
                }
                n_files += 1;
                n_bytes += entry.metadata().unwrap().len();
                if options.verbosity >= 3 {
                    println!("{}", entry.path().display());
                }
                let key = group_key(&entry);
                let by_path = by_key_and_path.entry(key).or_insert_with(|| HashMap::new());
                by_path.insert(entry.path().to_str().unwrap().to_string(), entry);
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
                    accmap
                        .entry(key)
                        .or_insert_with(|| HashMap::new())
                        .extend(ents);
                }
                accmap
            });
    let mut by_key: KeyToDentsMap = HashMap::new();
    let find_stats = FindStats {
        n_bytes: n_bytes,
        n_dirs: n_dirs,
        n_files: n_files,
        n_precull_groups: by_key_and_path.len() as u64,
    };
    for (key, ent_map) in by_key_and_path {
        if ent_map.len() > 1 {
            by_key.insert(key, ent_map.values().cloned().collect());
        }
    }
    prog.finish();
    return (find_stats, calculate_hash_stats(&by_key), by_key);
}
