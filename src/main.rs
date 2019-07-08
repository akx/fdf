#[macro_use]
extern crate clap;
extern crate rayon;
extern crate regex;
extern crate walkdir;

use clap::{App, Arg};
use hashbrown::HashMap;
use rayon::prelude::*;
use regex::RegexSet;
use walkdir::{DirEntry, WalkDir};

struct Options {
    file_include_regexes: RegexSet,
    file_exclude_regexes: RegexSet,
    dir_include_regexes: RegexSet,
    dir_exclude_regexes: RegexSet,
    verbosity: u64,
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

fn main() {
    let args = parse_args();
    let directories = values_t!(args, "directory", String).unwrap();
    let options = Options {
        dir_exclude_regexes: RegexSet::new(&[r"node_modules|pycache|\.git|\.tox"]).unwrap(),
        dir_include_regexes: RegexSet::new(&([] as [String; 0])).unwrap(),
        file_exclude_regexes: RegexSet::new(&([] as [String; 0])).unwrap(),
        file_include_regexes: RegexSet::new(&([] as [String; 0])).unwrap(),
        verbosity: args.occurrences_of("v"),
    };

    let map = directories
        .par_iter()
        .map(|dir| {
            let mut map: HashMap<String, Vec<DirEntry>> = HashMap::new();
            println!("Traversing: {:#?}", dir);
            let walker = WalkDir::new(dir).into_iter();
            for er in walker.filter_entry(|dent| options.is_entry_included(&dent)) {
                let entry = er.unwrap();
                if options.verbosity >= 3 {
                    println!("{}", entry.path().display());
                }
                let key = group_key(&entry);
                match map.get_mut(&key) {
                    Some(vec) => vec.push(entry),
                    None => {
                        map.insert(key, vec![entry]);
                    }
                }
            }
            map
        })
        .reduce(
            || HashMap::new(),
            |mut acc, map| {
                acc.extend(map);
                acc
            },
        );
    let n_files = map.values().fold(0u32, |acc, lst| acc + lst.len() as u32);
    println!("{} groups, {} files.", map.len(), n_files);
}
