use regex::RegexSet;
use walkdir::DirEntry;

pub struct Options {
    pub file_include_regexes: RegexSet,
    pub file_exclude_regexes: RegexSet,
    pub dir_include_regexes: RegexSet,
    pub dir_exclude_regexes: RegexSet,
    pub verbosity: u64,
    pub hash_bytes: u64,
}

impl Options {
    pub fn is_file_included(&self, path_str: &str) -> bool {
        if self.file_exclude_regexes.len() > 0 && self.file_exclude_regexes.is_match(&path_str) {
            return false;
        }
        if self.file_include_regexes.len() > 0 && !self.file_include_regexes.is_match(&path_str) {
            return false;
        }
        true
    }

    pub fn is_dir_included(&self, path_str: &str) -> bool {
        if self.dir_exclude_regexes.len() > 0 && self.dir_exclude_regexes.is_match(&path_str) {
            return false;
        }
        if self.dir_include_regexes.len() > 0 && !self.dir_include_regexes.is_match(&path_str) {
            return false;
        }
        return true;
    }

    pub fn is_entry_included(&self, dent: &DirEntry) -> bool {
        match dent.file_type().is_dir() {
            true => self.is_dir_included(dent.path().to_str().unwrap()),
            false => self.is_file_included(dent.path().to_str().unwrap()),
        }
    }
}