use super::options::{HashAlgorithm, Options};
use crate::fdf::find::GroupKey;
use hashbrown::HashMap;
use murmur3::murmur3_x64_128;
use rayon::prelude::*;
use sha2::{Digest, Sha256};
use std::error::Error;
use std::fs::File;
use std::io::{copy, BufReader, Read};
use walkdir::DirEntry;

fn hash_file<'a>(
    key: &'a GroupKey,
    dent: &'a DirEntry,
    options: &Options,
) -> Result<(&'a DirEntry, String), Box<dyn Error>> {
    let f = File::open(dent.path())?.take(options.hash_bytes);
    let buf_cap = options.hash_bytes.min(524_288).max(8_192) as usize;
    let mut reader = BufReader::with_capacity(buf_cap, f);
    let hash: String;
    match options.hash_algorithm {
        HashAlgorithm::Sha256 => {
            let mut sha256 = Sha256::new();
            let n = copy(&mut reader, &mut sha256)?;
            assert!(n <= options.hash_bytes);
            hash = hex::encode(sha256.result());
        }
        HashAlgorithm::Murmur3 => {
            let mut murmur3_buf: [u8; 16] = [0; 16];
            let seed: u32 = (key.size % (std::u32::MAX as u64)) as u32;
            murmur3_x64_128(&mut reader, seed, &mut murmur3_buf);
            hash = format!("m{}", hex::encode(murmur3_buf));
        }
    }

    if options.verbosity >= 2 {
        println!("{} {}", dent.path().display(), hash);
    }
    Ok((dent, hash))
}

pub fn hash_key_group<'a>(
    key: &'a GroupKey,
    dents: &'a [DirEntry],
    options: &Options,
) -> HashMap<String, Vec<&'a DirEntry>> {
    let hashes: Vec<Result<(&DirEntry, String), ()>> = dents
        .par_iter()
        .map(|dent| match hash_file(key, dent, options) {
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
