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
                .short("d")
                .multiple(true)
                .value_name("DIRECTORY")
                .takes_value(true)
                .help("Add directory to search"),
        )
        .arg(
            Arg::with_name("v")
                .short("v")
                .multiple(true)
                .help("Sets the level of verbosity"),
        )
        .arg(
            Arg::with_name("hash-bytes")
                .short("b")
                .takes_value(true)
                .help("Hash N first bytes only?")
                .default_value("1000000000"),
        )
        .arg(
            Arg::with_name("hash-algorithm")
                .short("a")
                .takes_value(true)
                .help("Hash algorithm selection")
                .possible_values(&HashAlgorithm::variants())
                .default_value("Sha256"),
        )
        .get_matches();
    let directories = values_t!(args, "directory", String)?;
    Ok(Options {
        directories,
        dir_exclude_regexes: RegexSet::new(&[r"node_modules|pycache|\.git|\.tox"])?,
        dir_include_regexes: RegexSet::new(&([] as [String; 0]))?,
        file_exclude_regexes: RegexSet::new(&([] as [String; 0]))?,
        file_include_regexes: RegexSet::new(&([] as [String; 0]))?,
        verbosity: args.occurrences_of("v"),
        hash_bytes: value_t!(args, "hash-bytes", u64).unwrap(),
        hash_algorithm: value_t!(args, "hash-algorithm", HashAlgorithm).unwrap(),
    })
}
