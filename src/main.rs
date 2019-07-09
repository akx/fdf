#[macro_use]
extern crate clap;
extern crate humansize;
extern crate rayon;
extern crate regex;
extern crate walkdir;

use clap::{App, Arg};
use hashbrown::HashMap;
use humansize::{file_size_opts, FileSize};
use regex::RegexSet;
use walkdir::{DirEntry};

mod fdf;

use fdf::options::{Options};

fn parse_args() -> clap::ArgMatches<'static> {
    return App::new("fdf")
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
        .get_matches();
}

fn main() {
    let args = parse_args();
    let directories = values_t!(args, "directory", String).unwrap();
    let options = Options {
        dir_exclude_regexes: RegexSet::new(&[r"node_modules|pycache|\.git|\.tox"]).unwrap(),
        dir_include_regexes: RegexSet::new(&([] as [String; 0])).unwrap(),
        file_exclude_regexes: RegexSet::new(&([] as [String; 0])).unwrap(),
        file_include_regexes: RegexSet::new(&([] as [String; 0])).unwrap(),
        verbosity: args.occurrences_of("v"),
        hash_bytes: args.value_of("hash-bytes").unwrap().parse().unwrap(),
    };
    let by_key: HashMap<String, Vec<DirEntry>> = fdf::find::find_files(&directories, &options);

    let (n_files, total_size) =
        by_key
            .values()
            .fold((0u64, 0u64), |(n_files, total_size), dents| {
                (
                    n_files + dents.len() as u64,
                    total_size
                        + dents
                            .iter()
                            .fold(0u64, |acc, dent| acc + dent.metadata().unwrap().len()),
                )
            });
    println!(
        "{} groups, {} files, {}.",
        by_key.len(),
        n_files,
        total_size.file_size(file_size_opts::CONVENTIONAL).unwrap()
    );
    // TODO: sort key groups so largest gains get processed first

    for (key, dents) in by_key {
        println!("### {}", key);
        for (hash, dents) in fdf::hash::hash_key_group(&dents, &options) {
            if dents.len() <= 1 {
                continue;
            }
            for dent in dents {
                println!("{} {}", hash, dent.path().to_str().unwrap());
            }
        }
    }
}
