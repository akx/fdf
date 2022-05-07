use super::options::{ExtensionGroupingOption, HashAlgorithm, Options, ReportOption};
use super::parse_size::parse_size_string;
use clap::{App, Arg, ArgMatches};
use regex;
use std::result::Result;

fn read_report_option(args: &ArgMatches, name: &str) -> ReportOption {
    if args.is_present(name) {
        let val = value_t!(args, name, String).unwrap_or_else(|_| String::new());
        if val.is_empty() || val == "-" {
            ReportOption::Stdout
        } else {
            ReportOption::File(val)
        }
    } else {
        ReportOption::None
    }
}

fn build_app<'a>() -> App<'a, 'a> {
    App::new("fdf")
        .version(crate_version!())
        .author(crate_authors!())
        .about("File duplicate finder")
        .arg(
            Arg::with_name("directory")
                .long("directory")
                .short("d")
                .multiple(true)
                .value_name("DIRECTORY")
                .takes_value(true)
                .help("Add directory to search"),
        )
        .arg(
            Arg::with_name("v")
                .short("v")
                .long("verbose")
                .multiple(true)
                .help("Sets the level of verbosity"),
        )
        .arg(
            Arg::with_name("hash-bytes")
                .short("b")
                .takes_value(true)
                .help("Hash N first bytes only")
                .default_value("18446744073709551615")
                .hide_default_value(true),
        )
        .arg(
            Arg::with_name("hash-algorithm")
                .short("a")
                .takes_value(true)
                .help("Select a hash algorithm. Murmur3 is faster, but not cryptographically safe.")
                .possible_values(&HashAlgorithm::variants())
                .default_value("Sha256"),
        )
        .arg(
            Arg::with_name("extensionless-full-name")
                .long("extensionless-full-name")
                .help(
                    "Group extensionless files by their full basename (instead of a single group)",
                )
                .required(false)
                .takes_value(false),
        )
        .arg(
            Arg::with_name("report-json")
                .long("output-json")
                .min_values(0)
                .max_values(1)
                .short("j")
                .help("Output JSON report (to stdout or the given filename)"),
        )
        .arg(
            Arg::with_name("report-human")
                .long("output-human")
                .min_values(0)
                .max_values(1)
                .short("h")
                .help("Output human-readable report (to stdout or the given filename)"),
        )
        .arg(
            Arg::with_name("report-file-list")
                .long("output-file-list")
                .min_values(0)
                .max_values(1)
                .help("Output list of files matched (to stdout or the given filename)"),
        )
        .arg(
            Arg::with_name("dir-exclude-re")
                .long("dir-exclude-re")
                .visible_alias("dx")
                .short("x")
                .takes_value(true)
                .multiple(true)
                .default_value(r"node_modules|pycache|\.git|\.tox")
                .required(false)
                .help("Regexp to exclude directories with"),
        )
        .arg(
            Arg::with_name("dir-include-re")
                .long("dir-include-re")
                .visible_alias("di")
                .short("i")
                .takes_value(true)
                .multiple(true)
                .required(false)
                .help("Regexp to include directories with"),
        )
        .arg(
            Arg::with_name("file-exclude-re")
                .long("file-exclude-re")
                .visible_alias("fx")
                .short("X")
                .takes_value(true)
                .multiple(true)
                .required(false)
                .help("Regexp to exclude files with"),
        )
        .arg(
            Arg::with_name("file-include-re")
                .long("file-include-re")
                .visible_alias("fi")
                .short("I")
                .takes_value(true)
                .multiple(true)
                .required(false)
                .help("Regexp to include files with"),
        )
        .arg(
            Arg::with_name("min-size")
                .long("min-size")
                .takes_value(true)
                .required(false)
                .help("Minimum file size to consider")
                .default_value("0")
                .hide_default_value(true),
        )
        .arg(
            Arg::with_name("max-size")
                .long("max-size")
                .takes_value(true)
                .required(false)
                .help("Maximum file size to consider")
                .default_value("18446744073709551615")
                .hide_default_value(true),
        )
}

fn parse_regex_set(args: &ArgMatches, name: &str) -> Result<regex::RegexSet, regex::Error> {
    regex::RegexSet::new(&values_t!(args, name, String).unwrap_or_else(|_| Vec::new()))
}

fn parse_size(args: &ArgMatches, name: &str) -> anyhow::Result<u64> {
    parse_size_string(&value_t!(args, name, String)?)
}

pub fn parse_args() -> anyhow::Result<Options> {
    let args = build_app().get_matches();
    Ok(Options {
        directories: values_t!(args, "directory", String)?,
        dir_exclude_regexes: parse_regex_set(&args, "dir-exclude-re")?,
        dir_include_regexes: parse_regex_set(&args, "dir-include-re")?,
        file_exclude_regexes: parse_regex_set(&args, "file-exclude-re")?,
        file_include_regexes: parse_regex_set(&args, "file-include-re")?,
        verbosity: args.occurrences_of("v"),
        hash_bytes: parse_size(&args, "hash-bytes").unwrap(),
        hash_algorithm: value_t!(args, "hash-algorithm", HashAlgorithm).unwrap(),
        report_human: read_report_option(&args, "report-human"),
        report_json: read_report_option(&args, "report-json"),
        report_file_list: read_report_option(&args, "report-file-list"),
        extension_grouping: if args.is_present("extensionless-full-name") {
            ExtensionGroupingOption::FullName
        } else {
            ExtensionGroupingOption::SingleGroup
        },
        min_size: parse_size(&args, "min-size")?,
        max_size: parse_size(&args, "max-size")?,
    })
}
