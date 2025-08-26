mod cli;
mod cli_options;
mod parse_size;

use crate::cli::parse_args;
use crate::cli_options::{CliOptions, ReportOption};
use fdf::{
    find::KeyToStringToDentMap,
    interrupt::{check_and_reset_interrupt, is_interrupted, set_interrupted},
    AugDirEntry, GrandResult, GroupKey, HashGroupResult, HashStats, KeyGroupResult, KeyToDentsMap,
    Options,
};
use indicatif::{HumanBytes, ProgressBar, ProgressStyle};
use rayon::prelude::*;
use std::error::Error;
use std::fs::File;
use std::io::{stdout, Write};
use std::process::exit;
use std::time::{Duration, Instant};
use termcolor::{Color, ColorChoice, ColorSpec, NoColor, StandardStream, WriteColor};

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
    stream: &mut dyn WriteColor,
    kgr: &KeyGroupResult,
) -> Result<(), Box<dyn Error>> {
    if !kgr.hash_groups.iter().any(|hg| hg.files.len() > 1) {
        return Ok(());
    }

    let size = HumanBytes(kgr.size).to_string();

    for hg in &kgr.hash_groups {
        let n_files = hg.files.len();
        if n_files <= 1 {
            continue;
        }
        stream.set_color(
            ColorSpec::new()
                .set_intense(true)
                .set_underline(true)
                .set_fg(Some(Color::Yellow)),
        )?;
        writeln!(
            stream,
            "### {} / {} / {} ({} files)",
            size, kgr.identifier, hg.hash, n_files
        )?;
        stream.reset()?;
        // Find common prefix for paths
        let common_prefix = hg
            .files
            .iter()
            .map(|path| path.as_str())
            .reduce(|a, b| {
                let min_len = a.len().min(b.len());
                let mut i = 0;
                while i < min_len && a.as_bytes()[i] == b.as_bytes()[i] {
                    i += 1;
                }
                &a[..i]
            })
            .unwrap_or("");

        for path in &hg.files {
            if common_prefix.len() > 1 {
                stream.set_color(ColorSpec::new().set_dimmed(true))?;
                write!(stream, "{common_prefix}")?;
                stream.reset()?;
                writeln!(stream, "{}", &path[common_prefix.len()..])?;
            } else {
                writeln!(stream, "{path}")?;
            }
        }
        writeln!(stream)?;
    }
    Ok(())
}

fn do_hash(core_options: &Options, by_key: KeyToDentsMap) -> Vec<KeyGroupResult> {
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
            Some(process_key_group(key, dents, core_options))
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
        HumanBytes(bytes_per_sec as u64),
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
            HumanBytes(n_bytes_wasted),
        );
    } else {
        eprintln!("No duplicates.");
    }
}

fn print_file_list(writer: &mut dyn Write, ksdmap: &KeyToStringToDentMap) {
    for (_key, path_to_dent_map) in ksdmap.iter() {
        for key in path_to_dent_map.keys() {
            writeln!(writer, "{key}").unwrap();
        }
    }
}

fn maybe_write_report<W>(report_option: &ReportOption, writer: W)
where
    W: Fn(&mut dyn WriteColor),
{
    use std::io::IsTerminal;
    let stream_box_opt: Option<Box<dyn WriteColor>> = match report_option {
        ReportOption::None => None,
        ReportOption::Stdout => Some(Box::new(StandardStream::stdout(
            if stdout().is_terminal() {
                ColorChoice::Auto
            } else {
                ColorChoice::Never
            },
        ))),
        ReportOption::File(name) => Some(Box::new(NoColor::new(File::create(name).unwrap()))),
    };
    match stream_box_opt {
        None => {}
        Some(mut stream_box) => {
            writer(&mut *stream_box);
        }
    };
}

fn configure_interrupt() {
    ctrlc::set_handler(move || {
        eprintln!("received Ctrl+C!");
        set_interrupted();
    })
    .unwrap_or_else(|e| eprintln!("Error setting Ctrl-C handler: {}", e));
}

fn main() {
    let (options, cli_options) = parse_args().unwrap_or_else(|err| {
        eprintln!("{err}");
        exit(1);
    });
    let CliOptions {
        report_json,
        mut report_human,
        report_file_list,
    } = cli_options;
    if report_json == ReportOption::None && report_human == ReportOption::None {
        eprintln!("No output arguments set; assuming human output to stdout desired.");
        report_human = ReportOption::Stdout;
    }
    configure_interrupt();
    let start_time = Instant::now();
    let (find_stats, mut hash_stats, by_key, precull_files) =
        fdf::find::find_files(&options, report_file_list != ReportOption::None);
    eprintln!(
        "Found {} files in {} directories ({} groups before culling) in {:.2} s, {}.",
        find_stats.n_files,
        find_stats.n_dirs,
        find_stats.n_precull_groups,
        start_time.elapsed().as_secs_f32(),
        HumanBytes(find_stats.n_bytes),
    );
    if let Some(precull_files) = precull_files {
        maybe_write_report(&report_file_list, |stream| {
            print_file_list(stream, &precull_files);
        });
    }
    eprintln!(
        "Hashing {} groups, {} files, {}.",
        hash_stats.n_groups,
        hash_stats.n_files,
        HumanBytes(hash_stats.n_bytes),
    );
    let hash_start_time = Instant::now();
    let key_group_results = do_hash(&options, by_key);
    hash_stats.interrupted = check_and_reset_interrupt();
    print_stage_duration("Hashing", &hash_stats, hash_start_time.elapsed());
    let output_start_time = Instant::now();
    maybe_write_report(&report_human, |stream| {
        for kgr in key_group_results.iter() {
            print_key_group_result(stream, kgr).unwrap();
        }
    });
    maybe_write_report(&report_json, |stream| {
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
