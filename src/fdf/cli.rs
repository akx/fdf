use super::options::{HashAlgorithm, NameGroupingOption, Options, ReportOption};
use super::parse_size::parse_size_string;
use clap::{command, value_parser, Arg, ArgAction, ArgMatches};

use regex;
use std::result::Result;

fn read_report_option(args: &ArgMatches, name: &str) -> ReportOption {
    match args.get_one::<String>(name) {
        Some(filename) => match filename.as_str() {
            "-" => ReportOption::Stdout,
            "" => ReportOption::Stdout,
            _ => ReportOption::File(filename.clone()),
        },
        None => ReportOption::None,
    }
}

fn parse_regex_set(args: &ArgMatches, name: &str) -> Result<regex::RegexSet, regex::Error> {
    regex::RegexSet::new(args.get_many::<String>(name).unwrap_or_default())
}

fn parse_size(value: &str) -> anyhow::Result<u64, String> {
    parse_size_string(value).map_err(|e| e.to_string())
}

pub fn parse_args() -> anyhow::Result<Options> {
    let matches = command!()
        .arg(
            Arg::new("directory")
                .long("directory")
                .short('d')
                .action(ArgAction::Append)
                .value_name("DIRECTORY")
                .help("Add directory to search")
                .required(true),
        )
        .arg(
            Arg::new("v")
                .short('v')
                .long("verbose")
                .action(ArgAction::Append)
                .default_value("0")
                .value_parser(value_parser!(u64))
                .help("Sets the level of verbosity"),
        )
        .arg(
            Arg::new("hash-bytes")
                .long("hash-bytes")
                .short('b')
                .help("Hash N first bytes only")
                .value_parser(parse_size)
                .default_value("18446744073709551615")
                .hide_default_value(true),
        )
        .arg(
            Arg::new("hash-algorithm")
                .long("hash-algorithm")
                .short('a')
                .help("Select a hash algorithm; there are speed/quality tradeoffs")
                .value_parser(value_parser!(HashAlgorithm))
                .default_value("sha256"),
        )
        .arg(
            Arg::new("name-grouping")
                .long("name-grouping")
                .value_parser(value_parser!(NameGroupingOption))
                .default_value("full-name-when-no-extension"),
        )
        .arg(
            Arg::new("report-json")
                .long("output-json")
                .required(false)
                .alias("oj")
                .help("Output JSON report (to stdout or the given filename)"),
        )
        .arg(
            Arg::new("report-human")
                .long("output-human")
                .required(false)
                .alias("oh")
                .help("Output human-readable report (to stdout or the given filename)"),
        )
        .arg(
            Arg::new("report-file-list")
                .long("output-file-list")
                .required(false)
                .alias("ol")
                .help("Output list of files matched (to stdout or the given filename)"),
        )
        .arg(
            Arg::new("dir-exclude-re")
                .long("dir-exclude-re")
                .visible_alias("dx")
                .short('x')
                .action(ArgAction::Append)
                .default_value(r"node_modules|pycache|\.git|\.tox")
                .required(false)
                .help("Regexp to exclude directories with"),
        )
        .arg(
            Arg::new("dir-include-re")
                .long("dir-include-re")
                .visible_alias("di")
                .short('i')
                .action(ArgAction::Append)
                .required(false)
                .help("Regexp to include directories with"),
        )
        .arg(
            Arg::new("file-exclude-re")
                .long("file-exclude-re")
                .visible_alias("fx")
                .short('X')
                .action(ArgAction::Append)
                .required(false)
                .help("Regexp to exclude files with"),
        )
        .arg(
            Arg::new("file-include-re")
                .long("file-include-re")
                .visible_alias("fi")
                .short('I')
                .action(ArgAction::Append)
                .required(false)
                .help("Regexp to include files with"),
        )
        .arg(
            Arg::new("min-size")
                .long("min-size")
                .required(false)
                .help("Minimum file size to consider")
                .value_parser(parse_size)
                .default_value("0")
                .hide_default_value(true),
        )
        .arg(
            Arg::new("max-size")
                .long("max-size")
                .required(false)
                .help("Maximum file size to consider")
                .value_parser(parse_size)
                .default_value("18446744073709551615")
                .hide_default_value(true),
        )
        .get_matches();
    Ok(Options {
        directories: matches
            .get_many::<String>("directory")
            .unwrap_or_default()
            .cloned()
            .collect(),
        dir_exclude_regexes: parse_regex_set(&matches, "dir-exclude-re")?,
        dir_include_regexes: parse_regex_set(&matches, "dir-include-re")?,
        file_exclude_regexes: parse_regex_set(&matches, "file-exclude-re")?,
        file_include_regexes: parse_regex_set(&matches, "file-include-re")?,
        verbosity: *matches.get_one::<u64>("v").unwrap(),
        hash_bytes: *matches.get_one::<u64>("hash-bytes").unwrap(),
        hash_algorithm: matches
            .get_one::<HashAlgorithm>("hash-algorithm")
            .unwrap()
            .clone(),
        report_human: read_report_option(&matches, "report-human"),
        report_json: read_report_option(&matches, "report-json"),
        report_file_list: read_report_option(&matches, "report-file-list"),
        name_grouping: matches
            .get_one::<NameGroupingOption>("name-grouping")
            .unwrap()
            .clone(),
        min_size: *matches.get_one::<u64>("min-size").unwrap(),
        max_size: *matches.get_one::<u64>("max-size").unwrap(),
    })
}
