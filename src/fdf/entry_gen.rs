use super::options::Options;
use std::fs;
use walkdir;

pub type EntryPair = (walkdir::DirEntry, Option<fs::Metadata>);

pub trait EntryPairGenerator {
    fn next_entry_pair(&mut self, options: &Options) -> Option<EntryPair>;
}
