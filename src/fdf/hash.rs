use rayon::prelude::*;
use hashbrown::HashMap;
use sha2::{Digest, Sha256};
use std::error::Error;
use std::fs::{File};
use std::io::{Read};
use walkdir::{DirEntry};
use super::options::{Options};

fn hash_file<'a>(
    dent: &'a DirEntry,
    options: &Options,
) -> Result<(&'a DirEntry, String), Box<Error>> {
    let f = File::open(dent.path())?;
    let mut buf = vec![0; 524288];
    let mut hasher = Sha256::new();
    let mut flimit = f.take(options.hash_bytes);
    loop {
        let n_read = flimit.read(&mut buf)?;
        if n_read == 0 {
            break;
        }
        hasher.input(&buf[0..n_read]);
    }
    // todo: verify we only read up to options.hash_bytes bytes?
    let hash = hex::encode(hasher.result());
    if options.verbosity >= 2 {
        println!("{} {}", dent.path().display(), hash);
    }
    Ok((dent, hash))
}

pub fn hash_key_group<'a>(
    dents: &'a Vec<DirEntry>,
    options: &Options,
) -> HashMap<String, Vec<&'a DirEntry>> {
    let hashes: Vec<Result<(&DirEntry, String), ()>> =
        dents.par_iter().map(|dent| match hash_file(dent, options) {
            Ok(v) => Ok(v),
            Err(x) => {
                println!("Unable to hash {:?}: {}", dent, x);
                Err(())
            },
        }).collect();
    let mut hm: HashMap<String, Vec<&DirEntry>> = HashMap::new();
    for res in hashes {
        match res {
            Ok((dent, hash)) => hm.entry(hash).or_insert_with(|| Vec::new()).push(dent),
            _ => (),
        }
    }
    hm
}
