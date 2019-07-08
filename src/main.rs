#[macro_use]
extern crate clap;
extern crate rayon;
extern crate regex;
extern crate walkdir;
extern crate humansize;

use clap::{App, Arg};
use hashbrown::HashMap;
use rayon::prelude::*;
use regex::RegexSet;
use walkdir::{DirEntry, WalkDir};
use humansize::{file_size_opts, FileSize};
use std::fs::{File};
use std::io::{Read, Seek, SeekFrom};
use std::error::Error;
use sha2::{Sha256, Digest};


struct Options {
    file_include_regexes: RegexSet,
    file_exclude_regexes: RegexSet,
    dir_include_regexes: RegexSet,
    dir_exclude_regexes: RegexSet,
    verbosity: u64,
    hash_bytes: u64,
}

impl Options {
    fn is_file_included(&self, path_str: &str) -> bool {
        if self.file_exclude_regexes.len() > 0 && self.file_exclude_regexes.is_match(&path_str) {
            return false;
        }
        if self.file_include_regexes.len() > 0 && !self.file_include_regexes.is_match(&path_str) {
            return false;
        }
        true
    }

    fn is_dir_included(&self, path_str: &str) -> bool {
        if self.dir_exclude_regexes.len() > 0 && self.dir_exclude_regexes.is_match(&path_str) {
            return false;
        }
        if self.dir_include_regexes.len() > 0 && !self.dir_include_regexes.is_match(&path_str) {
            return false;
        }
        return true;
    }

    fn is_entry_included(&self, dent: &DirEntry) -> bool {
        match dent.file_type().is_dir() {
            true => self.is_dir_included(dent.path().to_str().unwrap()),
            false => self.is_file_included(dent.path().to_str().unwrap()),
        }
    }
}

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

fn group_key(dent: &DirEntry) -> String {
    let size = match dent.metadata() {
        Ok(s) => s.len(),
        Err(_) => 0,
    };
    let extension: &str = match dent.path().extension() {
        Some(ps) => ps.to_str().unwrap_or_default(),
        None => "",
    };
    format!("{},{}", extension, size)
}

fn find_files(directories: &Vec<String>, options: &Options) -> HashMap<String, Vec<DirEntry>> {
    let by_key_and_path = directories
        .par_iter()
        .map(|dir| {
            let mut by_key_and_path: HashMap<String, HashMap<String, DirEntry>> = HashMap::new();
            println!("Traversing: {:#?}", dir);
            let walker = WalkDir::new(dir).into_iter();
            for er in walker.filter_entry(|dent| options.is_entry_included(&dent)) {
                let entry = er.unwrap();
                if options.verbosity >= 3 {
                    println!("{}", entry.path().display());
                }
                let key = group_key(&entry);
                let by_path = by_key_and_path.entry(key).or_insert_with(|| HashMap::new());
                by_path.insert(entry.path().to_str().unwrap().to_string(), entry);
            }
            by_key_and_path
        })
        // merge per-directory maps into one
        .reduce(
            || HashMap::new(),
            |mut accmap, map| {
                for (key, ents) in map {
                    accmap
                        .entry(key)
                        .or_insert_with(|| HashMap::new())
                        .extend(ents);
                }
                accmap
            },
        );
    let mut by_key: HashMap<String, Vec<DirEntry>> = HashMap::new();
    for (key, ent_map) in by_key_and_path {
        by_key.insert(key, ent_map.values().cloned().collect());
    }
    by_key
}

fn hash_key_group<'a>(dents: &'a Vec<DirEntry>, options: &Options) -> HashMap<String, Vec<&'a DirEntry>> {
    let hashes: Vec<Result<(&DirEntry, String), Box<Error>>> = dents.iter().map(|dent| {
        let f = File::open(dent.path())?;
        let mut buf = vec![0; 524288];
        let mut hasher = Sha256::new();
        let mut flimit = f.take(options.hash_bytes);
        loop {
            let n_read = flimit.read(&mut buf)?;
            if n_read == 0 {
                break;
            }
            hasher.input(&buf[0..n_read]);
        }
        // todo: verify we only read up to options.hash_bytes bytes?
        let hash = hex::encode(hasher.result());
        Ok((dent, hash))
    }).collect();
    let mut hm: HashMap<String, Vec<&DirEntry>> = HashMap::new();
    for res in hashes {
        match res {
            Ok((dent, hash)) => hm.entry(hash).or_insert_with(|| Vec::new()).push(dent),
            _ => ()
        }
    }
    hm
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
    let by_key: HashMap<String, Vec<DirEntry>> = find_files(&directories, &options);

    let (n_files, total_size) = by_key
        .values()
        .fold((0u64, 0u64), |(n_files, total_size), dents| (
            n_files + dents.len() as u64,
            total_size + dents.iter().fold(0u64, |acc, dent| acc + dent.metadata().unwrap().len())
        ));
    println!(
        "{} groups, {} files, {}.",
        by_key.len(),
        n_files,
        total_size.file_size(file_size_opts::CONVENTIONAL).unwrap()
    );
    // TODO: sort key groups so largest gains get processed first

    for (key, dents) in by_key {
        for (hash, dents) in hash_key_group(&dents, &options) {
            if dents.len() <= 1 {
                continue;
            }
            for dent in dents {
                println!("{} {}", hash, dent.path().to_str().unwrap());
            }
        }
    }
}
