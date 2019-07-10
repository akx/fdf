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
use clap::{App, Arg};
use fdf::find::GroupKey;
use fdf::options::Options;
use humansize::{file_size_opts, FileSize};
use regex::RegexSet;
use std::error::Error;
use std::result::Result;
use walkdir::DirEntry;

fn parse_args() -> Result<Options, Box<Error>> {
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
        .get_matches();
    let directories = values_t!(args, "directory", String)?;
    Ok(Options {
        directories,
        dir_exclude_regexes: RegexSet::new(&[r"node_modules|pycache|\.git|\.tox"])?,
        dir_include_regexes: RegexSet::new(&([] as [String; 0]))?,
        file_exclude_regexes: RegexSet::new(&([] as [String; 0]))?,
        file_include_regexes: RegexSet::new(&([] as [String; 0]))?,
        verbosity: args.occurrences_of("v"),
        hash_bytes: args
            .value_of("hash-bytes")
            .unwrap_or("1000000000")
            .parse()?,
    })
}

fn process_key_group(key: &GroupKey, dents: &Vec<DirEntry>, options: &Options) {
    let size = key.size.file_size(file_size_opts::CONVENTIONAL).unwrap();
    let mut header_printed: bool = false;
    for (hash, dents) in fdf::hash::hash_key_group(&dents, &options) {
        if dents.len() <= 1 {
            continue;
        }
        if !header_printed {
            println!(
                "### {} {:?}s ({} files)",
                size,
                &*key.extension,
                dents.len()
            );
            header_printed = true;
        }
        for dent in dents {
            println!("{} {}", hash, dent.path().to_str().unwrap());
        }
        println!("");
    }
}

fn main() {
    let options = parse_args().unwrap();
    let (find_stats, hash_stats, by_key) = fdf::find::find_files(&options);
    eprintln!(
        "Found {} files in {} directories ({} groups before culling), {}.",
        find_stats.n_files,
        find_stats.n_dirs,
        find_stats.n_precull_groups,
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
    let mut sorted_pairs = by_key.iter().collect::<Vec<(&GroupKey, &Vec<DirEntry>)>>();
    sorted_pairs.sort_unstable_by(|(ka, _), (kb, _)| kb.size.cmp(&ka.size));
    sorted_pairs
        .iter()
        .for_each(|(key, dents)| process_key_group(key, dents, &options));
}
