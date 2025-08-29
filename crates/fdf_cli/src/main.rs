mod cli;
mod cli_options;
mod parse_size;

use crate::cli::parse_args;
use crate::cli_options::{CliOptions, ReportOption};
use fdf::find::{calculate_hash_stats, FindFilesResult};
use fdf::{
    find::KeyToStringToDentMap, hashwork, GrandResult, HashStats, InterruptHandle, KeyGroupResult,
};
use indicatif::HumanBytes;
use std::error::Error;
use std::fs::File;
use std::io::{stdout, Write};
use std::process::exit;
use std::time::{Duration, Instant};
use termcolor::{Color, ColorChoice, ColorSpec, NoColor, StandardStream, WriteColor};
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

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
            .map(|tp| tp.path.as_str())
            .reduce(|a, b| {
                let min_len = a.len().min(b.len());
                let mut i = 0;
                while i < min_len && a.as_bytes()[i] == b.as_bytes()[i] {
                    i += 1;
                }
                &a[..i]
            })
            .unwrap_or("");

        for tp in &hg.files {
            if common_prefix.len() > 1 {
                stream.set_color(ColorSpec::new().set_dimmed(true))?;
                write!(stream, "{common_prefix}")?;
                stream.reset()?;
                writeln!(stream, "{}", &tp.path[common_prefix.len()..])?;
            } else {
                writeln!(stream, "{}", tp.path)?;
            }
        }
        writeln!(stream)?;
    }
    Ok(())
}

fn print_stage_duration(label: &str, hash_stats: &HashStats, d: Duration) {
    let time = d.as_secs_f32();
    let files_per_sec = hash_stats.n_files as f32 / time;
    let bytes_per_sec = hash_stats.n_bytes as f32 / time;

    eprintln!(
        "{}: {} seconds ({} files/sec, {}/sec).",
        label,
        time,
        files_per_sec,
        HumanBytes(bytes_per_sec as u64),
    );
}

fn print_duplicate_info(key_group_results: &[KeyGroupResult], elide_same_tag_groups: bool) {
    let mut n_duplicate_files: u64 = 0;
    let mut n_bytes_wasted: u64 = 0;
    for kgr in key_group_results.iter() {
        if elide_same_tag_groups && !kgr.cross_tag {
            continue;
        }
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

fn configure_interrupt(interrupt_handle: InterruptHandle) {
    ctrlc::set_handler(move || {
        eprintln!("received Ctrl+C!");
        interrupt_handle.set_interrupted();
    })
    .unwrap_or_else(|e| eprintln!("Error setting Ctrl-C handler: {e}"));
}

fn init_tracing(verbosity: u8) {
    let filter = match verbosity {
        0 => "warn",
        1 => "info",
        2 => "debug",
        _ => "trace",
    };

    let env_filter = EnvFilter::try_from_default_env()
        .or_else(|_| EnvFilter::try_new(filter))
        .unwrap();

    tracing_subscriber::registry()
        .with(env_filter)
        .with(
            fmt::layer()
                .with_ansi(true)
                .with_target(false)
                .with_level(true)
                .compact(),
        )
        .init();
}

fn main() -> anyhow::Result<()> {
    let (options, cli_options) = parse_args().unwrap_or_else(|err| {
        eprintln!("{err}");
        exit(1);
    });

    init_tracing(options.verbosity as u8);

    let CliOptions {
        report_json,
        mut report_human,
        report_file_list,
    } = cli_options;
    if report_json == ReportOption::None && report_human == ReportOption::None {
        eprintln!("No output arguments set; assuming human output to stdout desired.");
        report_human = ReportOption::Stdout;
    }
    configure_interrupt(options.interrupt_handle.clone());
    let start_time = Instant::now();
    let find_progress = indicatif::ProgressBar::new_spinner();
    find_progress.enable_steady_tick(Duration::from_millis(500));
    let FindFilesResult {
        find_stats,
        by_key,
        precull_files,
    } = fdf::find::find_files(
        &options,
        report_file_list != ReportOption::None,
        &mut |event| {
            if let fdf::ProgressEvent::FindProgress {
                n_dirs,
                n_files,
                n_bytes,
            } = event
            {
                find_progress.set_message(format!(
                    "Finding files... (dirs: {}, files: {}, {})",
                    n_dirs,
                    n_files,
                    HumanBytes(n_bytes)
                ));
            }
        },
    )?;
    find_progress.finish_and_clear();
    eprintln!(
        "Found {} files in {} directories ({} groups before culling) in {:.2} s, {}.",
        find_stats.n_files,
        find_stats.n_dirs,
        find_stats.n_precull_groups,
        find_stats.duration.unwrap().as_secs_f32(),
        HumanBytes(find_stats.n_bytes),
    );
    if let Some(precull_files) = precull_files {
        maybe_write_report(&report_file_list, |stream| {
            print_file_list(stream, &precull_files);
        });
    }
    let mut hash_stats = calculate_hash_stats(&by_key);
    eprintln!(
        "Hashing {} groups, {} files, {}.",
        hash_stats.n_groups,
        hash_stats.n_files,
        HumanBytes(hash_stats.n_bytes),
    );
    let tag_names = options.tag_names.clone();
    let elide_same_tag_groups = options.elide_same_tag_groups;

    let hash_progress = indicatif::ProgressBar::new(hash_stats.n_groups);
    hash_progress.set_style(
        indicatif::ProgressStyle::with_template(
            "{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {msg}",
        )?
        .progress_chars("#>-"),
    );
    let hash_result = hashwork::do_hash(options, by_key, &mut |event| match event {
        fdf::ProgressEvent::HashStarted { total_files } => {
            hash_progress.set_length(total_files);
            hash_progress.set_position(0);
        }
        fdf::ProgressEvent::HashProgress {
            processed_files,
            total_files,
            completed_groups,
            total_groups,
        } => {
            hash_progress.set_message(format!(
                "{processed_files}/{total_files} files scanned, {completed_groups}/{total_groups} groups complete"
            ));
            hash_progress.set_position(processed_files);
        }
        _ => {}
    })?;
    hash_stats.duration = hash_result.duration;
    hash_stats.interrupted = hash_result.interrupted;
    hash_progress.finish_and_clear();
    print_stage_duration("Hashing", &hash_stats, hash_stats.duration.unwrap());
    let key_group_results = hash_result.key_group_results;
    let output_start_time = Instant::now();
    maybe_write_report(&report_human, |stream| {
        for kgr in key_group_results.iter() {
            if elide_same_tag_groups && !kgr.cross_tag {
                continue;
            }
            print_key_group_result(stream, kgr).unwrap();
        }
    });
    maybe_write_report(&report_json, |stream| {
        let gr = GrandResult {
            tag_names: tag_names.clone(),
            find_stats: &find_stats,
            hash_stats: &hash_stats,
            key_groups: &key_group_results,
        };
        serde_json::to_writer_pretty(stream, &gr).unwrap();
    });
    print_duplicate_info(&key_group_results, elide_same_tag_groups);
    print_stage_duration("Output", &hash_stats, output_start_time.elapsed());
    print_stage_duration("Finished", &hash_stats, start_time.elapsed());
    Ok(())
}
