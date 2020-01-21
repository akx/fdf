use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct FindStats {
    pub n_bytes: u64,
    pub n_dirs: u64,
    pub n_files: u64,
    pub n_precull_groups: u64,
}

impl FindStats {
    pub fn zero() -> FindStats {
        FindStats {
            n_files: 0,
            n_dirs: 0,
            n_bytes: 0,
            n_precull_groups: 0,
        }
    }
    pub fn file_of_size(size: u64) -> FindStats {
        FindStats {
            n_files: 1,
            n_dirs: 0,
            n_bytes: size,
            n_precull_groups: 0,
        }
    }
    pub fn accumulate(self: &mut FindStats, other: &FindStats) {
        self.n_precull_groups += other.n_precull_groups;
        self.n_files += other.n_files;
        self.n_bytes += other.n_bytes;
        self.n_dirs += other.n_dirs;
    }
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
pub struct GrandResult<'a> {
    pub find_stats: &'a FindStats,
    pub hash_stats: &'a HashStats,
    pub key_groups: &'a Vec<KeyGroupResult>,
}
