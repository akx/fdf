use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct FindStats {
    pub n_bytes: u64,
    pub n_dirs: u64,
    pub n_files: u64,
    pub n_precull_groups: u64,
}

#[derive(Debug, Serialize)]
pub struct HashStats {
    pub n_bytes: u64,
    pub n_files: u64,
    pub n_groups: u64,
}

#[derive(Debug, Serialize)]
pub struct HashGroupResult {
    pub hash: String,
    pub files: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct KeyGroupResult {
    pub size: u64,
    pub identifier: String,
    pub hash_groups: Vec<HashGroupResult>,
    pub n_files: u64,
}

#[derive(Debug, Serialize)]
pub struct GrandResult {
    pub find_stats: FindStats,
    pub hash_stats: HashStats,
    pub key_groups: Vec<KeyGroupResult>,
}
