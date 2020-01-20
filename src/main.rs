#[macro_use]
extern crate clap;
extern crate humansize;
extern crate rayon;
extern crate regex;
extern crate serde;
extern crate serde_json;
extern crate string_cache;
extern crate walkdir;

mod fdf;
use fdf::cli::parse_args;
use fdf::find::GroupKey;
use fdf::options::Options;
use fdf::output::*;
use humansize::{file_size_opts, FileSize};
use walkdir::DirEntry;

fn process_key_group(key: &GroupKey, dents: &[DirEntry], options: &Options) -> KeyGroupResult {
    KeyGroupResult {
        size: key.size,
        identifier: key.extension.to_string(),
        hash_groups: fdf::hash::hash_key_group(&key, &dents, &options)
            .iter()
            .map(|(hash, dents)| HashGroupResult {
                hash: hash.to_string(),
                files: dents
                    .iter()
                    .map(|dent| dent.path().to_str().unwrap().to_string())
                    .collect(),
            })
            .collect(),
        n_files: dents.len() as u64,
    }
}

fn print_key_group_result(kgr: &KeyGroupResult) {
    if !kgr.hash_groups.iter().any(|hg| hg.files.len() > 1) {
        return;
    }
    let size = kgr.size.file_size(file_size_opts::CONVENTIONAL).unwrap();
    println!("### {}/{} ({} files)", size, kgr.identifier, kgr.n_files);
    for hg in &kgr.hash_groups {
        if hg.files.len() > 1 {
            for path in &hg.files {
                println!("{} {}", hg.hash, path);
            }
            println!();
        }
    }
}

fn main() {
    let options = parse_args().unwrap();
    let (find_stats, hash_stats, by_key) = fdf::find::find_files(&options);
    eprintln!(
        "Found {} files in {} directories ({} groups before culling), {}.",
        find_stats.n_files,
        find_stats.n_dirs,
        find_stats.n_precull_groups,
        find_stats
            .n_bytes
            .file_size(file_size_opts::CONVENTIONAL)
            .unwrap()
    );
    eprintln!(
        "Hashing {} groups, {} files, {}.",
        hash_stats.n_groups,
        hash_stats.n_files,
        hash_stats
            .n_bytes
            .file_size(file_size_opts::CONVENTIONAL)
            .unwrap()
    );
    let mut sorted_pairs = by_key.iter().collect::<Vec<(&GroupKey, &Vec<DirEntry>)>>();
    sorted_pairs.sort_unstable_by(|(ka, _), (kb, _)| kb.size.cmp(&ka.size));
    let key_group_results = sorted_pairs
        .iter()
        .map(|(key, dents)| {
            let kgr = process_key_group(key, dents, &options);
            print_key_group_result(&kgr);
            kgr
        })
        .collect();
    let gr = GrandResult {
        find_stats,
        hash_stats,
        key_groups: key_group_results,
    };
    println!("{}", serde_json::to_string(&gr).unwrap());
}
