use anyhow::{Result, anyhow};
use std::fs;
use std::io::Read;
use std::path::Path;
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum HashAlgo {
    Md5,
    Sha1,
    Sha256,
    Sha512,
    Sha3_256,
    Blake2b,
    Blake3,
}

impl HashAlgo {
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "md5" => Some(Self::Md5),
            "sha1" => Some(Self::Sha1),
            "sha256" => Some(Self::Sha256),
            "sha512" => Some(Self::Sha512),
            "sha3-256" | "sha3_256" => Some(Self::Sha3_256),
            "blake2b" => Some(Self::Blake2b),
            "blake3" => Some(Self::Blake3),
            _ => None,
        }
    }
}

/// Calculate multiple hashes for a file in a single pass.
pub fn calculate_hashes(
    path: &Path,
    algos: &[HashAlgo],
) -> Result<HashMap<HashAlgo, String>> {
    use blake2::Blake2b512;
    use md5::Md5;
    use sha1::Sha1;
    use sha2::{Digest, Sha256, Sha512};
    use sha3::Sha3_256;

    let mut file = fs::File::open(path)?;
    let mut buffer = [0u8; 128 * 1024]; // 128KB buffer

    // Initialize hasher instances
    let mut h_md5 = Md5::new();
    let mut h_sha1 = Sha1::new();
    let mut h_sha256 = Sha256::new();
    let mut h_sha512 = Sha512::new();
    let mut h_sha3_256 = Sha3_256::new();
    let mut h_blake2b = Blake2b512::new();
    let mut h_blake3 = blake3::Hasher::new();

    loop {
        let n = file.read(&mut buffer)?;
        if n == 0 {
            break;
        }
        let chunk = &buffer[..n];

        if algos.contains(&HashAlgo::Md5) {
            h_md5.update(chunk);
        }
        if algos.contains(&HashAlgo::Sha1) {
            h_sha1.update(chunk);
        }
        if algos.contains(&HashAlgo::Sha256) {
            h_sha256.update(chunk);
        }
        if algos.contains(&HashAlgo::Sha512) {
            h_sha512.update(chunk);
        }
        if algos.contains(&HashAlgo::Sha3_256) {
            h_sha3_256.update(chunk);
        }
        if algos.contains(&HashAlgo::Blake2b) {
            h_blake2b.update(chunk);
        }
        if algos.contains(&HashAlgo::Blake3) {
            h_blake3.update(chunk);
        }
    }

    let mut results = HashMap::new();
    for algo in algos {
        let hex = match algo {
            HashAlgo::Md5 => format!("{:x}", h_md5.finalize_reset()),
            HashAlgo::Sha1 => format!("{:x}", h_sha1.finalize_reset()),
            HashAlgo::Sha256 => format!("{:x}", h_sha256.finalize_reset()),
            HashAlgo::Sha512 => format!("{:x}", h_sha512.finalize_reset()),
            HashAlgo::Sha3_256 => format!("{:x}", h_sha3_256.finalize_reset()),
            HashAlgo::Blake2b => format!("{:x}", h_blake2b.finalize_reset()),
            HashAlgo::Blake3 => h_blake3.finalize().to_hex().to_string(),
        };
        results.insert(*algo, hex);
    }
    Ok(results)
}

/// Calculate SHA256 checksum of a file.
pub fn sha256_checksum(path: &Path) -> Result<String> {
    hash_file(path, HashAlgo::Sha256)
}

pub fn hash_file(path: &Path, algo: HashAlgo) -> Result<String> {
    let hashes = calculate_hashes(path, &[algo])?;
    hashes
        .get(&algo)
        .cloned()
        .ok_or_else(|| anyhow!("Hash calculation failed"))
}
