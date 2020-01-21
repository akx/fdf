use crate::fdf::options::Options;
use std::io::{BufRead, BufReader, Lines, Error};
use std::fs;
use std::path::PathBuf;
use walkdir::{DirEntry, WalkDir, IntoIter};
use std::fs::Metadata;
use std::rc::Rc;
use crate::fdf::entry_gen::{EntryPairGenerator, EntryPair};

struct WalkDirEntryPairGenerator {
    walkdir: WalkDir,
    walkdir_iter: IntoIter,
}

impl WalkDirEntryPairGenerator {
    pub fn new(
        dir: &String,
    ) -> WalkDirEntryPairGenerator {
        let walkdir = WalkDir::new(dir);
        let walkdir_iter = walkdir.into_iter();
        WalkDirEntryPairGenerator {
            walkdir,
            walkdir_iter,
        }
    }

    // Unfortunately copied from .filter_entry
    fn internal_next(self: &mut WalkDirEntryPairGenerator, options: &options) -> Option<Result<DirEntry>> {
        loop {
            let dent = match self.walkdir_iter.next() {
                None => return None,
                Some(result) => itry!(result),
            };
            if !options.is_entry_included(&dent) {
                if dent.is_dir() {
                    self.it.skip_current_dir();
                }
                continue;
            }
            return Some(Ok(dent));
        }
    }
}

impl EntryPairGenerator for WalkDirEntryPairGenerator {
    fn next_entry_pair(self: &mut WalkDirEntryPairGenerator, options: &Options) -> Option<EntryPair> {
        loop {
            let dent = match self.internal_next(options) {
                None => return None,
                Some(result) => itry!(result),
            };
            return Some((dent.unwrap(), None));
        }
    }
}


fn dir_name_to_entries(options: &Options, dir: &String) -> impl Iterator<Item=EntryPair> {
    return
        walkdir
            .into_iter()
            .filter_entry(|entry| options.is_entry_included(&entry))
            // TODO: use metadata from the entry on Windows here
            .map(|entry| (entry.unwrap(), None))
            .into_iter();
}
