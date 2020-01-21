use super::options::{HashAlgorithm, Options};
use clap::{App, Arg};
use regex::RegexSet;
use std::error::Error;
use std::result::Result;

pub fn parse_args() -> Result<Options, Box<dyn Error>> {
    let args = App::new("fdf")
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
                .default_value("1000000000")
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
            Arg::with_name("json")
                .long("json")
                .short("j")
                .help("Output JSON report"),
        )
        .arg(
            Arg::with_name("human")
                .long("human")
                .short("h")
                .help("Output human-readable report"),
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
        .get_matches();
    Ok(Options {
        directories: values_t!(args, "directory", String)?,
        dir_exclude_regexes: RegexSet::new(
            &values_t!(args, "dir-exclude-re", String).unwrap_or_else(|_| Vec::new()),
        )?,
        dir_include_regexes: RegexSet::new(
            &values_t!(args, "dir-include-re", String).unwrap_or_else(|_| Vec::new()),
        )?,
        file_exclude_regexes: RegexSet::new(
            &values_t!(args, "file-exclude-re", String).unwrap_or_else(|_| Vec::new()),
        )?,
        file_include_regexes: RegexSet::new(
            &values_t!(args, "file-include-re", String).unwrap_or_else(|_| Vec::new()),
        )?,
        verbosity: args.occurrences_of("v"),
        hash_bytes: value_t!(args, "hash-bytes", u64).unwrap(),
        hash_algorithm: value_t!(args, "hash-algorithm", HashAlgorithm).unwrap(),
        report_human: args.is_present("human"),
        report_json: args.is_present("json"),
    })
}
