#[macro_use]
extern crate clap;
extern crate regex;
extern crate rayon;
extern crate walkdir;

use clap::{App, Arg};
use regex::RegexSet;
use std::fs::{self, DirEntry};
use std::io;
use std::path::Path;
use rayon::prelude::*;
use walkdir::WalkDir;


struct Options {
    file_include_regexes: RegexSet,
    file_exclude_regexes: RegexSet,
    dir_include_regexes: RegexSet,
    dir_exclude_regexes: RegexSet,
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

fn visit_dirs(options: &Options, dir: &Path) -> io::Result<Vec<fs::DirEntry>> {
    let mut dirs: Vec<fs::DirEntry> = vec![];
    let mut files: Vec<fs::DirEntry> = vec![];
    if dir.is_dir() {
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            let path_str = path.to_str().unwrap();
            if path.is_dir() && options.is_dir_included(path_str) {
                dirs.push(entry);
            } else {
                if options.is_file_included(path_str) {
                    files.push(entry);
                    println!("{:?}", path);
                }
            }
        }
    }
    files.par_extend(dirs.into_par_iter().flat_map(|de| visit_dirs(&options, &de.path()).unwrap()));
    
    Ok(files)
}


fn main() {
    let args = parse_args();
    let directories = values_t!(args, "directory", String).unwrap();
    let options = Options {
        dir_exclude_regexes: RegexSet::new(&[r"node_modules|pycache|\.git|\.tox"]).unwrap(),
        dir_include_regexes: RegexSet::new(&([] as [String; 0])).unwrap(),
        file_exclude_regexes: RegexSet::new(&([] as [String; 0])).unwrap(),
        file_include_regexes: RegexSet::new(&([] as [String; 0])).unwrap(),
    };

    directories.par_iter().for_each(|dir| {
        println!("Traversing: {:#?}", dir);
        visit_dirs(&options, Path::new(dir)).expect("ok");
    });
}
