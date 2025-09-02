use crate::cli_options::{CliOptions, HashAlgorithm, NameGroupingOption, ReportOption};
use crate::parse_size::parse_size_string;
use clap::Parser;
use fdf::options::DirectorySpec;
use fdf::Options as CoreOptions;
use std::collections::HashMap;
use std::result::Result;

fn read_report_option(value: &Option<String>) -> ReportOption {
    match value {
        Some(filename) => match filename.as_str() {
            "-" => ReportOption::Stdout,
            "" => ReportOption::Stdout,
            _ => ReportOption::File(filename.clone()),
        },
        None => ReportOption::None,
    }
}

fn parse_regex_set(values: Vec<String>) -> Result<regex::RegexSet, regex::Error> {
    regex::RegexSet::new(&values)
}

fn parse_size(value: &str) -> anyhow::Result<u64, String> {
    parse_size_string(value).map_err(|e| e.to_string())
}

#[derive(Parser, Debug)]
#[command(name = "fdf")]
#[command(about = "Fast file duplicate finder")]
pub struct Args {
    /// Add directory to search
    #[arg(short = 'd', long = "directory")]
    pub directory: Vec<String>,

    /// Add directory to search, with the given tag
    #[arg(long = "tagged-directory", visible_alias = "td", value_names = &["TAG", "DIR"], number_of_values = 2)]
    pub tagged_directory: Vec<String>,

    /// Sets the level of verbosity
    #[arg(short = 'v', long = "verbose", action = clap::ArgAction::Count)]
    pub verbose: u8,

    /// Hash N first bytes only
    #[arg(short = 'b', long = "hash-bytes", value_parser = parse_size)]
    pub hash_bytes: Option<u64>,

    /// File size threshold for oneshot hashing (avoids streaming for small files)
    #[arg(long = "hash-oneshot-size", value_parser = parse_size, default_value = "64k")]
    pub hash_oneshot_size: u64,

    /// Select a hash algorithm; there are speed/quality tradeoffs
    #[arg(short = 'a', long = "hash-algorithm", default_value = "sha256")]
    pub hash_algorithm: HashAlgorithm,

    /// Name grouping option
    #[arg(long = "name-grouping", default_value = "full-name-when-no-extension")]
    pub name_grouping: NameGroupingOption,

    /// Output JSON report (to stdout or the given filename)
    #[arg(long = "output-json", alias = "oj", value_name = "FILE", num_args = 0..=1, default_missing_value = "-")]
    pub report_json: Option<String>,

    /// Output human-readable report (to stdout or the given filename)
    #[arg(long = "output-human", alias = "oh", value_name = "FILE", num_args = 0..=1, default_missing_value = "-")]
    pub report_human: Option<String>,

    /// Output list of files matched (to stdout or the given filename)
    #[arg(long = "output-file-list", alias = "ol", value_name = "FILE", num_args = 0..=1, default_missing_value = "-")]
    pub report_file_list: Option<String>,

    /// Regexp to exclude directories with
    #[arg(short = 'x', long = "dir-exclude-re", visible_alias = "dx", default_values_t = vec![r"node_modules|pycache|\.git|\.tox".to_string()])]
    pub dir_exclude_re: Vec<String>,

    /// Regexp to include directories with
    #[arg(short = 'X', long = "dir-include-re", visible_alias = "di")]
    pub dir_include_re: Vec<String>,

    /// Regexp to exclude files with
    #[arg(short = 'y', long = "file-exclude-re", visible_alias = "fx")]
    pub file_exclude_re: Vec<String>,

    /// Regexp to include files with
    #[arg(short = 'Y', long = "file-include-re", visible_alias = "fi")]
    pub file_include_re: Vec<String>,

    /// Minimum file size to consider
    #[arg(long = "min-size", value_parser = parse_size, default_value = "0")]
    pub min_size: u64,

    /// Maximum file size to consider
    #[arg(long = "max-size", value_parser = parse_size, default_value = "18446744073709551615", hide_default_value = true)]
    pub max_size: u64,

    /// Number of threads for hashing files within each group (default: number of CPUs)
    #[arg(long = "file-hash-threads")]
    pub file_hash_threads: Option<usize>,

    /// Elide groups where all files have the same directory tag
    #[arg(long = "elide-same-tag-groups")]
    pub elide_same_tag_groups: bool,
}

pub fn parse_args() -> anyhow::Result<(CoreOptions, CliOptions)> {
    let args = Args::parse();

    let file_exclude_regexes = parse_regex_set(args.file_exclude_re)?;
    let file_include_regexes = parse_regex_set(args.file_include_re)?;
    let dir_exclude_regexes = parse_regex_set(args.dir_exclude_re)?;
    let dir_include_regexes = parse_regex_set(args.dir_include_re)?;

    // Use system CPU count as default for file hash threads
    let n_cpus = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(4);
    let file_hash_threads = args.file_hash_threads.unwrap_or(n_cpus);

    let mut tag_to_index: HashMap<String, u8> = HashMap::new();
    let mut tag_names: Vec<String> = Vec::new();

    let mut directories: Vec<DirectorySpec> = Vec::new();
    for dir in args.directory.into_iter() {
        directories.push(DirectorySpec {
            tag_index: None,
            path: dir,
        });
    }

    for pair in args.tagged_directory.chunks_exact(2) {
        let tag = &pair[0];
        let dir = &pair[1];
        let tag_index = if let Some(&index) = tag_to_index.get(tag) {
            index
        } else {
            let index = tag_names.len() as u8;
            tag_to_index.insert(tag.clone(), index);
            tag_names.push(tag.clone());
            index
        };
        directories.push(DirectorySpec {
            tag_index: Some(tag_index),
            path: dir.clone(),
        });
    }

    let core_options = CoreOptions {
        directories,
        tag_names,
        file_include_regexes,
        file_exclude_regexes,
        dir_include_regexes,
        dir_exclude_regexes,
        verbosity: args.verbose as u64,
        hash_bytes: args.hash_bytes,
        hash_oneshot_size: args.hash_oneshot_size,
        hash_algorithm: args.hash_algorithm.into(),
        name_grouping: args.name_grouping.into(),
        min_size: args.min_size,
        max_size: args.max_size,
        file_hash_threads,
        elide_same_tag_groups: args.elide_same_tag_groups,
        interrupt_handle: fdf::InterruptHandle::new(),
    };
    core_options.validate()?;

    let report_json = read_report_option(&args.report_json);
    let report_human = read_report_option(&args.report_human);
    let report_file_list = read_report_option(&args.report_file_list);

    let cli_options = CliOptions {
        report_json,
        report_human,
        report_file_list,
    };
    Ok((core_options, cli_options))
}
