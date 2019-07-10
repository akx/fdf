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
use humansize::{file_size_opts, FileSize};
use walkdir::DirEntry;

fn process_key_group(key: &GroupKey, dents: &Vec<DirEntry>, options: &Options) {
    let size = key.size.file_size(file_size_opts::CONVENTIONAL).unwrap();
    let mut header_printed: bool = false;
    for (hash, dents) in fdf::hash::hash_key_group(&dents, &options) {
        if dents.len() <= 1 {
            continue;
        }
        if !header_printed {
            println!(
                "### {} {:?}s ({} files)",
                size,
                &*key.extension,
                dents.len()
            );
            header_printed = true;
        }
        for dent in dents {
            println!("{} {}", hash, dent.path().to_str().unwrap());
        }
        println!("");
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
    sorted_pairs
        .iter()
        .for_each(|(key, dents)| process_key_group(key, dents, &options));
}
