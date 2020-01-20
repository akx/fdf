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
use fdf::find::{GroupKey, KeyToDentsMap};
use fdf::options::Options;
use fdf::output::*;
use humansize::{file_size_opts, FileSize};
use indicatif::{ProgressBar, ProgressStyle};
use rayon::prelude::*;
use std::time::{Duration, Instant};
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

fn do_hash(options: &mut Options, by_key: KeyToDentsMap) -> Vec<KeyGroupResult> {
    let mut sorted_pairs = by_key.iter().collect::<Vec<(&GroupKey, &Vec<DirEntry>)>>();
    sorted_pairs.sort_unstable_by(|(ka, _), (kb, _)| kb.size.cmp(&ka.size));
    let prog = ProgressBar::new(sorted_pairs.len() as u64);
    prog.set_style(
        ProgressStyle::default_bar().template("{pos:>6}/{len:6} {msg} (ETA {eta}) {wide_bar}"),
    );

    let key_group_results: Vec<KeyGroupResult> = sorted_pairs
        .par_iter()
        .map(|(key, dents)| {
            prog.set_message(format!("{}/{}", key.extension, key.size).as_str());
            prog.inc(1);
            process_key_group(key, dents, &options)
        })
        .collect();
    prog.finish();
    key_group_results
}

fn print_stage_duration(label: &str, hash_stats: &HashStats, d: Duration) {
    let time = d.as_secs_f32();
    let files_per_sec = (hash_stats.n_files as f32 / time) as u32;
    let bytes_per_sec = ((hash_stats.n_bytes) as f32 / time) as u32;

    eprintln!(
        "{}: {} seconds ({} files/sec, {}/sec).",
        label,
        time,
        files_per_sec,
        bytes_per_sec
            .file_size(file_size_opts::CONVENTIONAL)
            .unwrap(),
    );
}

fn main() {
    let mut options = parse_args().unwrap();
    if !(options.report_json || options.report_human) {
        eprintln!("No output arguments set; assuming human output desired.");
        options.report_human = true;
    }
    let start_time = Instant::now();
    let (find_stats, hash_stats, by_key) = fdf::find::find_files(&options);
    eprintln!(
        "Found {} files in {} directories ({} groups before culling) in {:.2} s, {}.",
        find_stats.n_files,
        find_stats.n_dirs,
        find_stats.n_precull_groups,
        start_time.elapsed().as_secs_f32(),
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
    let hash_start_time = Instant::now();
    let key_group_results = do_hash(&mut options, by_key);
    print_stage_duration("Hashing", &hash_stats, hash_start_time.elapsed());
    let output_start_time = Instant::now();
    if options.report_human {
        for kgr in key_group_results.iter() {
            print_key_group_result(&kgr);
        }
    }
    if options.report_json {
        let gr = GrandResult {
            find_stats: &find_stats,
            hash_stats: &hash_stats,
            key_groups: &key_group_results,
        };
        println!("{}", serde_json::to_string(&gr).unwrap());
    }
    print_stage_duration("Output", &hash_stats, output_start_time.elapsed());
    print_stage_duration("Finished", &hash_stats, start_time.elapsed());
}
