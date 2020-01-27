use crate::fdf::options::Options;
use std::io::{BufRead, BufReader, Lines, Error};
use std::fs;
use std::path::PathBuf;
use walkdir::DirEntry;
use std::fs::Metadata;
use std::rc::Rc;
use crate::fdf::entry_gen::{EntryPairGenerator, EntryPair};

pub struct FileListEntryPairGenerator {
    lines: Lines<BufReader<fs::File>>,
}

impl FileListEntryPairGenerator {
    pub fn new(
        file_list_name: &String,
    ) -> FileListEntryPairGenerator {
        FileListEntryPairGenerator {
            lines: BufReader::new(fs::File::open(file_list_name).unwrap()).lines(),
        }
    }
}

impl EntryPairGenerator for FileListEntryPairGenerator {
    fn next_entry_pair(self: &mut FileListEntryPairGenerator, options: &Options) -> Option<EntryPair> {
        loop {
            let line = self.lines.next();
            match self.lines.next() {
                None => { return None; } // out of lines
                Some(filename_res) => {
                    let filename = filename_res.unwrap();
                    match filename_to_entry_pair(options, &filename) {
                        None => { continue; }
                        Some(ep) => { return Some(ep); }
                    }
                }
            }
        }
    }
}

fn filename_to_entry_pair(options: &Options, filename: &String) -> Option<EntryPair> {
    let path = PathBuf::from(filename);
    return match fs::metadata(&path) {
        Ok(md) => {
            let dent = DirEntry {
                path,
                ty: md.file_type(),
                follow_link: false,
                depth: 0,
                ino: 0,
            };
            if !options.is_entry_included(&dent) {
                None
            }
            Some((dent, Some(md)))
        }
        Err(err) => None,
    };
}
