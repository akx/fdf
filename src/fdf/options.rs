use regex::RegexSet;
use walkdir::DirEntry;

arg_enum! {
    #[derive(Debug)]
    pub enum HashAlgorithm {
        Sha256,
        Murmur3,
    }
}

#[derive(PartialEq, Eq, Debug, Hash)]
pub enum ReportOption {
    None,
    Stdout,
    File(String),
}

#[derive(PartialEq, Eq, Debug, Hash)]
pub enum ExtensionGroupingOption {
    FullName,
    SingleGroup,
}

#[derive(Debug)]
pub struct Options {
    pub directories: Vec<String>,
    pub file_include_regexes: RegexSet,
    pub file_exclude_regexes: RegexSet,
    pub dir_include_regexes: RegexSet,
    pub dir_exclude_regexes: RegexSet,
    pub verbosity: u64,
    pub hash_bytes: u64,
    pub hash_algorithm: HashAlgorithm,
    pub report_json: ReportOption,
    pub report_human: ReportOption,
    pub report_file_list: ReportOption,
    pub extension_grouping: ExtensionGroupingOption,
}

impl Options {
    pub fn is_file_included(&self, path_str: &str) -> bool {
        if !self.file_exclude_regexes.is_empty() && self.file_exclude_regexes.is_match(path_str) {
            return false;
        }
        if !self.file_include_regexes.is_empty() && !self.file_include_regexes.is_match(path_str) {
            return false;
        }
        true
    }

    pub fn is_dir_included(&self, path_str: &str) -> bool {
        if !self.dir_exclude_regexes.is_empty() && self.dir_exclude_regexes.is_match(path_str) {
            return false;
        }
        if !self.dir_include_regexes.is_empty() && !self.dir_include_regexes.is_match(path_str) {
            return false;
        }
        true
    }

    pub fn is_entry_included(&self, dent: &DirEntry) -> bool {
        if dent.file_type().is_dir() {
            self.is_dir_included(dent.path().to_str().unwrap())
        } else {
            self.is_file_included(dent.path().to_str().unwrap())
        }
    }
}
