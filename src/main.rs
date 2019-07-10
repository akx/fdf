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
use std::error::Error;
use std::result::Result;
use walkdir::DirEntry;

fn process_key_group(key: &GroupKey, dents: &Vec<DirEntry>, options: &Options) -> KeyGroupResult {
    let size = key.size.file_size(file_size_opts::CONVENTIONAL).unwrap();
    let identifier = key.extension.to_string();
    let mut header_printed: bool = false;
    let mut kgr = KeyGroupResult {
        size: key.size,
        identifier: identifier.to_string(),
        hash_groups: Vec::new(),
    };
    for (hash, dents) in fdf::hash::hash_key_group(&dents, &options) {
        if dents.len() <= 1 {
            continue;
        }

        if !header_printed {
            println!("### {}/{} ({} files)", size, identifier, dents.len());
            header_printed = true;
        }
        let mut files: Vec<String> = Vec::new();
        for dent in dents {
            let filename = dent.path().to_str().unwrap();
            println!("{} {}", hash, &filename);
            files.push(filename.to_string());
        }
        println!("");
        let hgr = HashGroupResult { hash, files };
        kgr.hash_groups.push(hgr);
    }
    kgr
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
        .map(|(key, dents)| process_key_group(key, dents, &options));
    let gr = GrandResult {
        find_stats,
        hash_stats,
        key_groups: key_group_results.collect(),
    };
    println!("{}", serde_json::to_string(&gr).unwrap());
}
