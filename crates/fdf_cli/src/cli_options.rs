use clap::ValueEnum;
use fdf::{HashAlgorithm as CoreHashAlgorithm, NameGroupingOption as CoreNameGroupingOption};

#[derive(Clone, PartialEq, Eq, Debug, ValueEnum)]
pub enum HashAlgorithm {
    Blake3,
    Sha256,
    Xxh64,
}

impl From<HashAlgorithm> for CoreHashAlgorithm {
    fn from(cli_algo: HashAlgorithm) -> Self {
        match cli_algo {
            HashAlgorithm::Blake3 => CoreHashAlgorithm::Blake3,
            HashAlgorithm::Sha256 => CoreHashAlgorithm::Sha256,
            HashAlgorithm::Xxh64 => CoreHashAlgorithm::Xxh64,
        }
    }
}

#[derive(PartialEq, Eq, Debug, Hash)]
pub enum ReportOption {
    None,
    Stdout,
    File(String),
}

#[derive(Clone, Debug, PartialEq, ValueEnum)]
pub enum NameGroupingOption {
    IgnoreName,
    FullNameWhenNoExtension,
    SingleGroupWhenNoExtension,
}

impl From<NameGroupingOption> for CoreNameGroupingOption {
    fn from(cli_grouping: NameGroupingOption) -> Self {
        match cli_grouping {
            NameGroupingOption::IgnoreName => CoreNameGroupingOption::IgnoreName,
            NameGroupingOption::FullNameWhenNoExtension => {
                CoreNameGroupingOption::FullNameWhenNoExtension
            }
            NameGroupingOption::SingleGroupWhenNoExtension => {
                CoreNameGroupingOption::SingleGroupWhenNoExtension
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, ValueEnum)]
pub enum Action {
    /// Replace duplicate files with APFS clonefile copies (macOS only).
    /// Keeps one file per duplicate set and replaces the others with
    /// copy-on-write clones, reclaiming disk space.
    Clonefile,
}

#[derive(Debug)]
pub struct CliOptions {
    pub report_json: ReportOption,
    pub report_human: ReportOption,
    pub report_file_list: ReportOption,
    pub action: Option<Action>,
    pub no_confirm: bool,
}
