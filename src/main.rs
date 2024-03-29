extern crate clap;
extern crate humansize;
extern crate rayon;
extern crate regex;
extern crate serde;
extern crate serde_json;
extern crate string_cache;
extern crate walkdir;

mod fdf;

use crate::fdf::find::KeyToStringToDentMap;
use crate::fdf::interrupt::{check_and_reset_interrupt, configure_interrupt, is_interrupted};
use fdf::cli::parse_args;
use fdf::find::{AugDirEntry, GroupKey, KeyToDentsMap};
use fdf::options::{Options, ReportOption};
use fdf::output::*;
use humansize::{format_size, DECIMAL};
use indicatif::{ProgressBar, ProgressStyle};
use rayon::prelude::*;
use std::error::Error;
use std::fs::File;
use std::io::{stdout, Write};
use std::process::exit;
use std::time::{Duration, Instant};

fn process_key_group(key: &GroupKey, dents: &[AugDirEntry], options: &Options) -> KeyGroupResult {
    KeyGroupResult {
        size: key.size,
        identifier: key.extension.to_string(),
        hash_groups: fdf::hash::hash_key_group(key, dents, options)
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

fn print_key_group_result(
    stream: &mut dyn Write,
    kgr: &KeyGroupResult,
) -> Result<(), Box<dyn Error>> {
    if !kgr.hash_groups.iter().any(|hg| hg.files.len() > 1) {
        return Ok(());
    }
    let size = format_size(kgr.size, DECIMAL);

    for hg in &kgr.hash_groups {
        let n_files = hg.files.len();
        if n_files <= 1 {
            continue;
        }
        writeln!(
            stream,
            "### {} / {} / {} ({} files)",
            size, kgr.identifier, hg.hash, n_files
        )?;
        for path in &hg.files {
            writeln!(stream, "{}", path)?;
        }
        writeln!(stream)?;
    }
    Ok(())
}

fn do_hash(options: &mut Options, by_key: KeyToDentsMap) -> Vec<KeyGroupResult> {
    let mut sorted_pairs = by_key
        .iter()
        .collect::<Vec<(&GroupKey, &Vec<AugDirEntry>)>>();
    sorted_pairs.sort_unstable_by(|(ka, _), (kb, _)| kb.size.cmp(&ka.size));
    let prog = ProgressBar::new(sorted_pairs.len() as u64);
    prog.set_style(
        ProgressStyle::default_bar()
            .template("{pos:>6}/{len:6} {msg} (ETA {eta}) {wide_bar}")
            .unwrap(),
    );

    let key_group_results: Vec<KeyGroupResult> = sorted_pairs
        .par_iter()
        .map(|(key, dents)| {
            if is_interrupted() {
                return None;
            }
            prog.set_message(format!("{}/{}", key.extension, key.size));
            prog.inc(1);
            Some(process_key_group(key, dents, options))
        })
        .filter_map(|x| x)
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
        format_size(bytes_per_sec, DECIMAL),
    );
}

fn print_duplicate_info(key_group_results: &[KeyGroupResult]) {
    let mut n_duplicate_files: u64 = 0;
    let mut n_bytes_wasted: u64 = 0;
    for kgr in key_group_results.iter() {
        for hg in &kgr.hash_groups {
            if hg.files.len() > 1 {
                let n = (hg.files.len() - 1) as u64;
                n_duplicate_files += n;
                n_bytes_wasted += kgr.size * n;
            }
        }
    }
    if n_duplicate_files > 0 {
        eprintln!(
            "{} duplicate files, {} wasted.",
            n_duplicate_files,
            format_size(n_bytes_wasted, DECIMAL),
        );
    } else {
        eprintln!("No duplicates.");
    }
}

fn print_file_list(writer: &mut dyn Write, ksdmap: &KeyToStringToDentMap) {
    for (_key, path_to_dent_map) in ksdmap.iter() {
        for key in path_to_dent_map.keys() {
            writeln!(writer, "{}", key).unwrap();
        }
    }
}

fn maybe_write_report<W>(report_option: &ReportOption, writer: W)
where
    W: Fn(&mut dyn Write),
{
    let stream_box_opt: Option<Box<dyn Write>> = match report_option {
        ReportOption::None => None,
        ReportOption::Stdout => Some(Box::new(stdout())),
        ReportOption::File(name) => Some(Box::new(File::create(name).unwrap())),
    };
    match stream_box_opt {
        None => {}
        Some(mut stream_box) => {
            writer(&mut *stream_box);
        }
    };
}

fn main() {
    let mut options = parse_args().unwrap_or_else(|err| {
        eprintln!("{}", err);
        exit(1);
    });
    if options.report_json == ReportOption::None && options.report_human == ReportOption::None {
        eprintln!("No output arguments set; assuming human output to stdout desired.");
        options.report_human = ReportOption::Stdout;
    }
    configure_interrupt();
    let start_time = Instant::now();
    let (find_stats, mut hash_stats, by_key, precull_files) =
        fdf::find::find_files(&options, options.report_file_list != ReportOption::None);
    eprintln!(
        "Found {} files in {} directories ({} groups before culling) in {:.2} s, {}.",
        find_stats.n_files,
        find_stats.n_dirs,
        find_stats.n_precull_groups,
        start_time.elapsed().as_secs_f32(),
        format_size(find_stats.n_bytes, DECIMAL),
    );
    if precull_files.is_some() {
        maybe_write_report(&options.report_file_list, |stream| {
            print_file_list(stream, precull_files.as_ref().unwrap());
        });
    }
    eprintln!(
        "Hashing {} groups, {} files, {}.",
        hash_stats.n_groups,
        hash_stats.n_files,
        format_size(hash_stats.n_bytes, DECIMAL),
    );
    let hash_start_time = Instant::now();
    let key_group_results = do_hash(&mut options, by_key);
    hash_stats.interrupted = check_and_reset_interrupt();
    print_stage_duration("Hashing", &hash_stats, hash_start_time.elapsed());
    let output_start_time = Instant::now();
    maybe_write_report(&options.report_human, |stream| {
        for kgr in key_group_results.iter() {
            print_key_group_result(stream, kgr).unwrap();
        }
    });
    maybe_write_report(&options.report_json, |stream| {
        let gr = GrandResult {
            find_stats: &find_stats,
            hash_stats: &hash_stats,
            key_groups: &key_group_results,
        };
        serde_json::to_writer_pretty(stream, &gr).unwrap();
    });
    print_duplicate_info(&key_group_results);
    print_stage_duration("Output", &hash_stats, output_start_time.elapsed());
    print_stage_duration("Finished", &hash_stats, start_time.elapsed());
}
