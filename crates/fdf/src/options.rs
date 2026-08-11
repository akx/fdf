use crate::interrupt::InterruptHandle;
use bit_set::BitSet;
use regex::RegexSet;
use serde::Serialize;
use walkdir::DirEntry;

#[derive(Clone, PartialEq, Eq, Debug, Serialize)]
pub enum HashAlgorithm {
    Blake3,
    Sha256,
    Xxh64,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub enum NameGroupingOption {
    IgnoreName,
    FullNameWhenNoExtension,
    SingleGroupWhenNoExtension,
}

#[derive(Debug, Serialize)]
pub struct DirectorySpec {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tag_index: Option<u8>,
    pub path: String,
}

#[derive(Debug)]
pub struct Options {
    pub tag_names: Vec<String>,
    pub directories: Vec<DirectorySpec>,
    pub file_include_regexes: RegexSet,
    pub file_exclude_regexes: RegexSet,
    pub dir_include_regexes: RegexSet,
    pub dir_exclude_regexes: RegexSet,
    pub verbosity: u64,
    pub hash_bytes: Option<u64>,
    pub hash_oneshot_size: u64,
    pub hash_algorithm: HashAlgorithm,
    pub name_grouping: NameGroupingOption,
    pub min_size: u64,
    pub max_size: u64,
    pub file_hash_threads: usize,
    pub elide_same_tag_groups: bool,
    pub interrupt_handle: InterruptHandle,
}

impl Options {
    pub fn validate(&self) -> anyhow::Result<()> {
        let mut tag_indices: BitSet = BitSet::new();
        let mut has_none_tag_indices = false;
        for dir in &self.directories {
            match dir.tag_index {
                Some(i) => {
                    tag_indices.insert(i as usize);
                }
                None => has_none_tag_indices = true,
            };
        }
        if has_none_tag_indices && tag_indices.count() > 0 {
            anyhow::bail!("Cannot mix tagged and untagged directories");
        }
        if !tag_indices.is_empty() && self.tag_names.len() != tag_indices.count() {
            anyhow::bail!("Tag indices in directories do not match tag names");
        }
        Ok(())
    }
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
