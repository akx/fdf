use crate::find::{AugDirEntry, GroupKey};
use crate::options::Options;
use crate::HashAlgorithm;
use rayon::prelude::*;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::error::Error;
use std::fs::File;
use std::hash::Hasher;
use std::io;
use std::io::{copy, BufReader, Read, Write};
use tracing::{debug, error};
use twox_hash::XxHash64;

enum HashResult {
    Blake3([u8; 32]),
    Sha256([u8; 32]),
    Xxh64(u64, u64), // (file_size, hash_value)
}

impl HashResult {
    fn format_as_string(&self) -> String {
        match self {
            HashResult::Blake3(bytes) => format!("blake3-{}", hex::encode(bytes)),
            HashResult::Sha256(bytes) => format!("sha256-{}", hex::encode(bytes)),
            HashResult::Xxh64(file_size, hash_value) => {
                format!("xxh64-{file_size:x}-{hash_value:x}")
            }
        }
    }
}

pub enum HasherInstance {
    Blake3,
    Sha256,
    Xxh64 { seed: u64, file_size: u64 },
}

impl HasherInstance {
    fn hash_oneshot(&self, data: &[u8]) -> HashResult {
        match self {
            HasherInstance::Blake3 => HashResult::Blake3(*blake3::hash(data).as_bytes()),
            HasherInstance::Sha256 => HashResult::Sha256(Sha256::digest(data).into()),
            HasherInstance::Xxh64 { seed, file_size } => {
                HashResult::Xxh64(*file_size, XxHash64::oneshot(*seed, data))
            }
        }
    }

    fn hash_streaming(&self, reader: &mut dyn Read) -> anyhow::Result<HashResult> {
        match self {
            HasherInstance::Blake3 => {
                let mut hasher = blake3::Hasher::new();
                copy(reader, &mut hasher)?;
                Ok(HashResult::Blake3(*hasher.finalize().as_bytes()))
            }
            HasherInstance::Sha256 => {
                let mut hasher = Sha256::new();
                copy(reader, &mut hasher)?;
                Ok(HashResult::Sha256(hasher.finalize().into()))
            }
            HasherInstance::Xxh64 { seed, file_size } => {
                let hasher = XxHash64::with_seed(*seed);
                let mut hw = HashWriter(hasher);
                copy(reader, &mut hw)?;
                let hash_u = hw.0.finish();
                Ok(HashResult::Xxh64(*file_size, hash_u))
            }
        }
    }
}

// via https://stackoverflow.com/questions/48533445/proper-way-to-hash-a-reader-in-rust
struct HashWriter<T: Hasher>(T);
impl<T: Hasher> Write for HashWriter<T> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.0.write(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }

    fn write_all(&mut self, buf: &[u8]) -> io::Result<()> {
        self.write(buf).map(|_| ())
    }
}

fn create_hasher(algo: &HashAlgorithm, seed: u64, file_size: u64) -> HasherInstance {
    match algo {
        HashAlgorithm::Blake3 => HasherInstance::Blake3,
        HashAlgorithm::Sha256 => HasherInstance::Sha256,
        HashAlgorithm::Xxh64 => HasherInstance::Xxh64 { seed, file_size },
    }
}

fn hash_file<'a>(
    key: &'a GroupKey,
    dent: &'a AugDirEntry,
    options: &Options,
) -> Result<(&'a AugDirEntry, String), Box<dyn Error>> {
    let file_size = dent.size;
    let bytes_to_hash = file_size.min(options.hash_bytes);

    let seed = key.size % (u32::MAX as u64);
    let hasher = create_hasher(&options.hash_algorithm, seed, key.size);

    let hash_result = if bytes_to_hash <= options.hash_oneshot_size {
        let mut buffer = vec![0u8; bytes_to_hash as usize];
        let mut f = File::open(dent.dir_entry.path())?;
        f.read_exact(&mut buffer)?;
        hasher.hash_oneshot(&buffer)
    } else {
        // Use streaming hashing for larger files
        let f = File::open(dent.dir_entry.path())?.take(options.hash_bytes);
        let buf_cap = options.hash_bytes.clamp(8_192, 524_288) as usize;
        let mut reader = BufReader::with_capacity(buf_cap, f);
        hasher.hash_streaming(&mut reader)?
    };

    let hash = hash_result.format_as_string();

    if options.verbosity >= 2 {
        debug!("Hashed: {} {}", dent.path().display(), hash);
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
                error!("Unable to hash {:?}: {}", dent, x);
                Err(())
            }
        })
        .collect();
    let mut hm: HashMap<String, Vec<&AugDirEntry>> = HashMap::new();
    for (dent, hash) in hashes.into_iter().flatten() {
        hm.entry(hash).or_default().push(dent)
    }
    hm
}
