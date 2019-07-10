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
