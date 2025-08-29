use serde::Serialize;
use std::time::Duration;

#[derive(Debug, Serialize)]
pub struct FindStats {
    pub interrupted: bool,
    pub n_bytes: u64,
    pub n_dirs: u64,
    pub n_files: u64,
    pub n_precull_groups: u64,
    pub duration: Option<Duration>,
}

#[derive(Debug, Serialize)]
pub struct HashStats {
    pub interrupted: bool,
    pub n_bytes: u64,
    pub n_files: u64,
    pub n_groups: u64,
    pub duration: Option<Duration>,
}

#[derive(Debug, Serialize)]
pub struct TaggedPath {
    pub tag_index: u8,
    pub path: String,
}

#[derive(Debug, Serialize)]
pub struct HashGroupResult {
    pub hash: String,
    pub files: Vec<TaggedPath>,
}

#[derive(Debug, Serialize)]
pub struct KeyGroupResult {
    pub size: u64,
    pub identifier: String,
    pub hash_groups: Vec<HashGroupResult>,
    pub n_files: u64,
    pub n_errors: u64,
    pub complete: bool,
    pub cross_tag: bool,
}

#[derive(Debug, Serialize)]
pub struct GrandResult<'a> {
    pub tag_names: Vec<String>,
    pub find_stats: &'a FindStats,
    pub hash_stats: &'a HashStats,
    pub key_groups: &'a Vec<KeyGroupResult>,
}
