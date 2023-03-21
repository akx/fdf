use super::find::{AugDirEntry, GroupKey};
use super::options::{HashAlgorithm, Options};
use rayon::prelude::*;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::error::Error;
use std::fs::File;
use std::hash::Hasher;
use std::io;
use std::io::{copy, BufReader, Read, Write};
use twox_hash::XxHash64;

// via https://stackoverflow.com/questions/48533445/proper-way-to-hash-a-reader-in-rust
struct HashWriter<T: Hasher>(T);
impl<T: Hasher> Write for HashWriter<T> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.0.write(buf);
        Ok(buf.len())
    }

    fn write_all(&mut self, buf: &[u8]) -> io::Result<()> {
        self.write(buf).map(|_| ())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

fn hash_file<'a>(
    key: &'a GroupKey,
    dent: &'a AugDirEntry,
    options: &Options,
) -> Result<(&'a AugDirEntry, String), Box<dyn Error>> {
    let f = File::open(dent.dir_entry.path())?.take(options.hash_bytes);
    let buf_cap = options.hash_bytes.min(524_288).max(8_192) as usize;
    let mut reader = BufReader::with_capacity(buf_cap, f);
    let hash: String = match options.hash_algorithm {
        HashAlgorithm::Blake3 => {
            let mut b3 = blake3::Hasher::new();
            let n = copy(&mut reader, &mut b3)?;
            assert!(n <= options.hash_bytes);
            format!("blake3-{}", hex::encode(b3.finalize().as_bytes()))
        }
        HashAlgorithm::Sha256 => {
            let mut sha256 = Sha256::new();
            let n = copy(&mut reader, &mut sha256)?;
            assert!(n <= options.hash_bytes);
            format!("sha256-{}", hex::encode(sha256.finalize()))
        }
        HashAlgorithm::Xxh64 => {
            let seed: u64 = key.size % (std::u32::MAX as u64);
            let hasher = XxHash64::with_seed(seed);
            let mut hw = HashWriter(hasher);
            let n = copy(&mut reader, &mut hw)?;
            assert!(n <= options.hash_bytes);
            let hash_u = hw.0.finish();
            format!("xxh64-{:x}-{:x}", key.size, hash_u)
        }
    };

    if options.verbosity >= 2 {
        println!("{} {}", dent.path().display(), hash);
    }
    Ok((dent, hash))
}

pub fn hash_key_group<'a>(
    key: &'a GroupKey,
    dents: &'a [AugDirEntry],
    options: &Options,
) -> HashMap<String, Vec<&'a AugDirEntry>> {
    let hashes: Vec<Result<(&AugDirEntry, String), ()>> = dents
        .par_iter()
        .map(|dent| match hash_file(key, dent, options) {
            Ok(v) => Ok(v),
            Err(x) => {
                println!("Unable to hash {:?}: {}", dent, x);
                Err(())
            }
        })
        .collect();
    let mut hm: HashMap<String, Vec<&AugDirEntry>> = HashMap::new();
    for (dent, hash) in hashes.into_iter().flatten() {
        hm.entry(hash).or_insert_with(Vec::new).push(dent)
    }
    hm
}
