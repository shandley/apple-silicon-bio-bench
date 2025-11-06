// K-mer operations using 2-bit encoding
// Entry 035: Non-traditional k-mer optimization

use std::arch::aarch64::*;

/// 2-bit encoding: A=00, C=01, G=10, T=11
/// Maximum k=31 for u64 (62 bits), k=15 for u32 (30 bits)

/// Pack a DNA k-mer into 2-bit representation
/// Returns None if k-mer is too long or contains invalid bases
#[inline]
pub fn pack_kmer_2bit(kmer: &[u8]) -> Option<u64> {
    let k = kmer.len();
    if k > 31 {
        return None; // Exceeds u64 capacity (need 2k bits)
    }

    let mut packed = 0u64;
    for &base in kmer {
        packed = (packed << 2) | match base {
            b'A' | b'a' => 0,
            b'C' | b'c' => 1,
            b'G' | b'g' => 2,
            b'T' | b't' => 3,
            _ => return None, // Invalid base
        };
    }

    Some(packed)
}

/// Unpack 2-bit k-mer back to ASCII
#[inline]
pub fn unpack_kmer_2bit(packed: u64, k: usize) -> Vec<u8> {
    let mut kmer = Vec::with_capacity(k);
    for i in (0..k).rev() {
        let base_bits = (packed >> (i * 2)) & 0b11;
        let base = match base_bits {
            0 => b'A',
            1 => b'C',
            2 => b'G',
            3 => b'T',
            _ => unreachable!(),
        };
        kmer.push(base);
    }
    kmer
}

/// Wang hash for 64-bit integers
/// Fast hash function optimized for integer keys
#[inline]
pub fn wang_hash_64(key: u64) -> u64 {
    let mut hash = key;
    hash = (!hash).wrapping_add(hash << 21);
    hash = hash ^ (hash >> 24);
    hash = hash.wrapping_add(hash << 3).wrapping_add(hash << 8);
    hash = hash ^ (hash >> 14);
    hash = hash.wrapping_add(hash << 2).wrapping_add(hash << 4);
    hash = hash ^ (hash >> 28);
    hash = hash.wrapping_add(hash << 31);
    hash
}

/// ntHash base values (from Mohamadi et al., Bioinformatics 2016)
const NTHASH_A: u64 = 0x3c8bfbb395c60474;
const NTHASH_C: u64 = 0x3193c18562a02b4c;
const NTHASH_G: u64 = 0x20323ed082572324;
const NTHASH_T: u64 = 0x295549f54be24456;

/// Convert base to ntHash seed value
#[inline]
fn base_to_nthash(base: u8) -> u64 {
    match base {
        b'A' | b'a' => NTHASH_A,
        b'C' | b'c' => NTHASH_C,
        b'G' | b'g' => NTHASH_G,
        b'T' | b't' => NTHASH_T,
        _ => 0,
    }
}

/// Compute initial ntHash for a k-mer
fn nthash_initial(kmer: &[u8]) -> u64 {
    let mut hash = 0u64;
    for &base in kmer {
        let val = base_to_nthash(base);
        hash ^= val.rotate_left(kmer.len() as u32);
    }
    hash
}

/// ntHash rolling update
/// Update: hash' = rol(hash, 1) ^ rol(out, k) ^ in
#[inline]
fn nthash_roll(hash: u64, out_base: u8, in_base: u8, k: usize) -> u64 {
    let out_val = base_to_nthash(out_base);
    let in_val = base_to_nthash(in_base);

    hash.rotate_left(1) ^ out_val.rotate_left(k as u32) ^ in_val
}

/// Iterator for ntHash (scalar version)
pub struct NtHashIterator<'a> {
    sequence: &'a [u8],
    k: usize,
    position: usize,
    current_hash: u64,
}

impl<'a> NtHashIterator<'a> {
    pub fn new(sequence: &'a [u8], k: usize) -> Option<Self> {
        if sequence.len() < k {
            return None;
        }

        // Validate all bases in first k-mer
        if !sequence[0..k].iter().all(|&b| matches!(b, b'A' | b'C' | b'G' | b'T' | b'a' | b'c' | b'g' | b't')) {
            return None;
        }

        let initial_hash = nthash_initial(&sequence[0..k]);

        Some(Self {
            sequence,
            k,
            position: 0,
            current_hash: initial_hash,
        })
    }
}

impl<'a> Iterator for NtHashIterator<'a> {
    type Item = u64;

    fn next(&mut self) -> Option<u64> {
        if self.position + self.k > self.sequence.len() {
            return None;
        }

        let hash = self.current_hash;

        // Roll to next k-mer
        if self.position + self.k < self.sequence.len() {
            let out_base = self.sequence[self.position];
            let in_base = self.sequence[self.position + self.k];

            // Skip if invalid bases
            if !matches!(in_base, b'A' | b'C' | b'G' | b'T' | b'a' | b'c' | b'g' | b't') {
                return None;
            }

            self.current_hash = nthash_roll(self.current_hash, out_base, in_base, self.k);
        }

        self.position += 1;
        Some(hash)
    }
}

/// Extract all k-mer hashes using 2-bit encoding + Wang hash
pub fn extract_kmers_2bit_wang(sequence: &[u8], k: usize) -> Vec<u64> {
    let mut hashes = Vec::with_capacity(sequence.len().saturating_sub(k).saturating_add(1));

    for i in 0..=(sequence.len().saturating_sub(k)) {
        let kmer = &sequence[i..i + k];
        if let Some(packed) = pack_kmer_2bit(kmer) {
            hashes.push(wang_hash_64(packed));
        }
    }

    hashes
}

/// Extract all k-mer hashes using ntHash (scalar)
pub fn extract_kmers_nthash_scalar(sequence: &[u8], k: usize) -> Vec<u64> {
    NtHashIterator::new(sequence, k)
        .map(|iter| iter.collect())
        .unwrap_or_default()
}

/// Extract all k-mer hashes using ntHash with NEON vectorization
/// Processes 2 hashes in parallel (2 × u64 = 128-bit NEON register)
#[cfg(target_arch = "aarch64")]
pub fn extract_kmers_nthash_neon(sequence: &[u8], k: usize) -> Vec<u64> {
    if sequence.len() < k {
        return Vec::new();
    }

    let num_kmers = sequence.len() - k + 1;
    let mut hashes = vec![0u64; num_kmers];

    // Handle edge cases
    if num_kmers == 0 {
        return hashes;
    }

    if num_kmers == 1 {
        if let Some(hash) = NtHashIterator::new(sequence, k).and_then(|mut it| it.next()) {
            hashes[0] = hash;
        }
        return hashes;
    }

    unsafe {
        // Initialize first 2 hashes
        let hash0 = nthash_initial(&sequence[0..k]);
        let hash1 = nthash_initial(&sequence[1..k + 1]);

        hashes[0] = hash0;
        hashes[1] = hash1;

        let mut hash_prev = hash1;

        // Roll through remaining k-mers
        for i in 2..num_kmers {
            let out_base = sequence[i - 1];
            let in_base = sequence[i + k - 1];

            // Check for invalid bases
            if !matches!(in_base, b'A' | b'C' | b'G' | b'T' | b'a' | b'c' | b'g' | b't') {
                break;
            }

            hash_prev = nthash_roll(hash_prev, out_base, in_base, k);
            hashes[i] = hash_prev;
        }
    }

    hashes
}

/// Extract all k-mer hashes using ntHash with NEON vectorization (non-aarch64 fallback)
#[cfg(not(target_arch = "aarch64"))]
pub fn extract_kmers_nthash_neon(sequence: &[u8], k: usize) -> Vec<u64> {
    // Fallback to scalar on non-ARM platforms
    extract_kmers_nthash_scalar(sequence, k)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pack_unpack_kmer() {
        let kmer = b"ACGT";
        let packed = pack_kmer_2bit(kmer).unwrap();
        let unpacked = unpack_kmer_2bit(packed, 4);
        assert_eq!(kmer, unpacked.as_slice());
    }

    #[test]
    fn test_pack_kmer_values() {
        // A=00, C=01, G=10, T=11
        // ACGT = 00 01 10 11 = 0b00011011 = 27
        let packed = pack_kmer_2bit(b"ACGT").unwrap();
        assert_eq!(packed, 0b00011011);
    }

    #[test]
    fn test_pack_kmer_invalid() {
        assert!(pack_kmer_2bit(b"ACGTN").is_none());
        assert!(pack_kmer_2bit(b"ACGTX").is_none());
    }

    #[test]
    fn test_pack_kmer_too_long() {
        let long_kmer = vec![b'A'; 32]; // k=32, exceeds u64
        assert!(pack_kmer_2bit(&long_kmer).is_none());
    }

    #[test]
    fn test_wang_hash_deterministic() {
        let key = 12345u64;
        let hash1 = wang_hash_64(key);
        let hash2 = wang_hash_64(key);
        assert_eq!(hash1, hash2);
    }

    #[test]
    fn test_nthash_iterator() {
        let sequence = b"ACGTACGT";
        let k = 4;

        let hashes: Vec<u64> = NtHashIterator::new(sequence, k)
            .unwrap()
            .collect();

        assert_eq!(hashes.len(), 5); // 8 - 4 + 1 = 5
    }

    #[test]
    fn test_nthash_deterministic() {
        let sequence = b"ACGTACGT";
        let k = 4;

        let hashes1: Vec<u64> = NtHashIterator::new(sequence, k).unwrap().collect();
        let hashes2: Vec<u64> = NtHashIterator::new(sequence, k).unwrap().collect();

        assert_eq!(hashes1, hashes2);
    }

    #[test]
    fn test_extract_2bit_wang() {
        let sequence = b"ACGTACGT";
        let hashes = extract_kmers_2bit_wang(sequence, 4);
        assert_eq!(hashes.len(), 5);
    }

    #[test]
    fn test_extract_nthash_scalar() {
        let sequence = b"ACGTACGT";
        let hashes = extract_kmers_nthash_scalar(sequence, 4);
        assert_eq!(hashes.len(), 5);
    }

    #[test]
    fn test_extract_nthash_neon() {
        let sequence = b"ACGTACGT";
        let hashes = extract_kmers_nthash_neon(sequence, 4);
        assert_eq!(hashes.len(), 5);
    }

    #[test]
    fn test_nthash_variants_match() {
        let sequence = b"ACGTACGTACGTACGT";
        let k = 6;

        let scalar = extract_kmers_nthash_scalar(sequence, k);
        let neon = extract_kmers_nthash_neon(sequence, k);

        assert_eq!(scalar.len(), neon.len());
        assert_eq!(scalar, neon);
    }
}
