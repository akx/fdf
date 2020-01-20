use super::options::{Options, HashAlgorithm};
use hashbrown::HashMap;
use rayon::prelude::*;
use sha2::{Digest, Sha256};
use std::error::Error;
use std::fs::File;
use std::io::{copy, Read};
use walkdir::DirEntry;
//use super::digest_wrap::{DigestWrap, SHA256Wrap};

fn hash_file<'a>(
    dent: &'a DirEntry,
    options: &Options,
) -> Result<(&'a DirEntry, String), Box<dyn Error>> {
    let f = File::open(dent.path())?;
    let mut hasher = Sha256::new();
    let mut flimit = f.take(options.hash_bytes);
    let n = copy(&mut flimit, &mut hasher)?;
    assert!(n <= options.hash_bytes);
    let hash = hex::encode(hasher.result());
    if options.verbosity >= 2 {
        println!("{} {}", dent.path().display(), hash);
    }
    Ok((dent, hash))
}

pub fn hash_key_group<'a>(
    dents: &'a [DirEntry],
    options: &Options,
) -> HashMap<String, Vec<&'a DirEntry>> {
    let hashes: Vec<Result<(&DirEntry, String), ()>> = dents
        .par_iter()
        .map(|dent| match hash_file(dent, options) {
            Ok(v) => Ok(v),
            Err(x) => {
                println!("Unable to hash {:?}: {}", dent, x);
                Err(())
            }
        })
        .collect();
    let mut hm: HashMap<String, Vec<&DirEntry>> = HashMap::new();
    for res in hashes {
        if let Ok((dent, hash)) = res {
            hm.entry(hash).or_insert_with(Vec::new).push(dent)
        }
    }
    hm
}
