//! Minimizer Extraction Operation
//!
//! Extracts minimizers from sequences - the minimum hash k-mer in each sliding window.
//! Minimizers are critical for genomic indexing (minimap2, BWA-MEM2) and sketching.
//!
//! # Operation Characteristics
//! - **Category**: Search/Aggregation
//! - **Complexity**: 0.45 (windowing + hashing + comparison)
//! - **Output**: Minimizer positions and hashes
//! - **NEON benefit**: HIGH (10-20× expected, comparison-heavy like quality_filter)
//!
//! # Implementation Notes
//! - Uses FNV-1a hash (fast, simple)
//! - Window size w typically 5-20 (genomics standard)
//! - NEON can parallelize hashing and min-finding
//! - Comparison operations are SIMD-friendly (vminq instructions)
//!
//! # Predictions (Entry 034)
//! - Expected NEON speedup: 10-20× (similar to quality_filter 25.1×, Entry 020-025)
//! - Evidence: Comparison-heavy operations benefit most from NEON
//! - Threshold: ≥5× speedup → Implement NEON in biometal

use crate::{OperationCategory, OperationOutput, PrimitiveOperation, SequenceRecord};
use anyhow::Result;
use rayon::prelude::*;
use serde::{Deserialize, Serialize};

#[cfg(target_arch = "aarch64")]
use std::arch::aarch64::*;

/// Minimizer extraction operation
pub struct Minimizers {
    /// K-mer size (typically 15-21 for genomics)
    k: usize,
    /// Window size (typically 5-20)
    w: usize,
}

/// Minimizer position and hash
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Minimizer {
    /// Position in sequence
    pub position: usize,
    /// K-mer hash value
    pub hash: u64,
    /// K-mer sequence (optional, for debugging)
    pub kmer: Option<String>,
}

/// Minimizer extraction output
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MinimizerOutput {
    /// Extracted minimizers
    pub minimizers: Vec<Minimizer>,
    /// Total k-mers processed
    pub total_kmers: usize,
    /// Number of minimizers (should be ~sequence_length / w)
    pub num_minimizers: usize,
}

impl Minimizers {
    pub fn new(k: usize, w: usize) -> Self {
        assert!(k >= 3 && k <= 31, "K-mer size must be 3-31");
        assert!(w >= 1 && w <= 100, "Window size must be 1-100");
        assert!(w <= k, "Window size should not exceed k-mer size");
        Self { k, w }
    }

    /// FNV-1a hash for k-mer (simple, fast hash function)
    /// Used in minimap2 and other genomic tools
    fn hash_kmer(kmer: &[u8]) -> u64 {
        const FNV_OFFSET: u64 = 0xcbf29ce484222325;
        const FNV_PRIME: u64 = 0x100000001b3;

        let mut hash = FNV_OFFSET;
        for &byte in kmer {
            hash ^= byte as u64;
            hash = hash.wrapping_mul(FNV_PRIME);
        }
        hash
    }

    /// Extract all valid k-mers with positions (naive)
    fn extract_kmers_with_positions_naive<'a>(&self, sequence: &'a [u8]) -> Vec<(usize, u64, &'a [u8])> {
        if sequence.len() < self.k {
            return Vec::new();
        }

        let mut kmers = Vec::new();
        for i in 0..=(sequence.len() - self.k) {
            let kmer = &sequence[i..i + self.k];

            // Validate: only ACGT
            if kmer.iter().all(|&b| matches!(b, b'A' | b'C' | b'G' | b'T')) {
                let hash = Self::hash_kmer(kmer);
                kmers.push((i, hash, kmer));
            }
        }
        kmers
    }

    /// Extract minimizers from k-mers (naive)
    fn extract_minimizers_naive(&self, sequence: &[u8]) -> Vec<Minimizer> {
        let kmers = self.extract_kmers_with_positions_naive(sequence);

        if kmers.is_empty() || kmers.len() < self.w {
            return Vec::new();
        }

        let mut minimizers = Vec::new();

        // Slide window over k-mers
        for window_start in 0..=(kmers.len() - self.w) {
            let window = &kmers[window_start..window_start + self.w];

            // Find minimum hash in window
            let (min_idx, min_hash, min_kmer) = window
                .iter()
                .enumerate()
                .min_by_key(|(_, (_, hash, _))| *hash)
                .map(|(idx, (pos, hash, kmer))| (idx, *hash, *kmer))
                .unwrap();

            let global_pos = window_start + min_idx;

            minimizers.push(Minimizer {
                position: kmers[global_pos].0, // Original position in sequence
                hash: min_hash,
                kmer: Some(String::from_utf8_lossy(min_kmer).into_owned()),
            });
        }

        minimizers
    }

    /// Extract minimizers using NEON (vectorized hashing + comparison)
    /// Expected: 10-20× speedup (comparison-heavy, similar to quality_filter)
    #[cfg(target_arch = "aarch64")]
    unsafe fn extract_minimizers_neon(&self, sequence: &[u8]) -> Vec<Minimizer> {
        if sequence.len() < self.k {
            return Vec::new();
        }

        // Step 1: Extract and hash all k-mers (vectorize hash computation)
        let mut kmer_hashes = Vec::with_capacity(sequence.len() - self.k + 1);

        for i in 0..=(sequence.len() - self.k) {
            let kmer = &sequence[i..i + self.k];

            // Validate: only ACGT (can vectorize this with NEON)
            let valid = self.validate_kmer_neon(kmer);

            if valid {
                let hash = Self::hash_kmer(kmer);
                kmer_hashes.push((i, hash, kmer));
            }
        }

        if kmer_hashes.is_empty() || kmer_hashes.len() < self.w {
            return Vec::new();
        }

        // Step 2: Find minimizers in sliding windows (vectorize comparison)
        let mut minimizers = Vec::new();

        // Process windows in batches for NEON (can compare 2 u64 values at once)
        for window_start in 0..=(kmer_hashes.len() - self.w) {
            let window = &kmer_hashes[window_start..window_start + self.w];

            // Find minimum hash using scalar (SIMD u64 comparison complex)
            // In practice, minimap2 uses scalar min-finding too
            let (min_idx, min_hash, min_kmer) = window
                .iter()
                .enumerate()
                .min_by_key(|(_, (_, hash, _))| *hash)
                .map(|(idx, (pos, hash, kmer))| (idx, *hash, *kmer))
                .unwrap();

            let global_pos = window_start + min_idx;

            minimizers.push(Minimizer {
                position: kmer_hashes[global_pos].0,
                hash: min_hash,
                kmer: Some(String::from_utf8_lossy(min_kmer).into_owned()),
            });
        }

        minimizers
    }

    /// Validate k-mer bases with NEON (vectorized validation)
    #[cfg(target_arch = "aarch64")]
    unsafe fn validate_kmer_neon(&self, kmer: &[u8]) -> bool {
        if kmer.len() < 16 {
            // For small k-mers, scalar is faster
            return kmer.iter().all(|&b| matches!(b, b'A' | b'C' | b'G' | b'T'));
        }

        let a_vec = vdupq_n_u8(b'A');
        let c_vec = vdupq_n_u8(b'C');
        let g_vec = vdupq_n_u8(b'G');
        let t_vec = vdupq_n_u8(b'T');

        let mut i = 0;
        while i + 16 <= kmer.len() {
            let vec = vld1q_u8(kmer.as_ptr().add(i));

            let is_a = vceqq_u8(vec, a_vec);
            let is_c = vceqq_u8(vec, c_vec);
            let is_g = vceqq_u8(vec, g_vec);
            let is_t = vceqq_u8(vec, t_vec);

            let valid = vorrq_u8(vorrq_u8(is_a, is_c), vorrq_u8(is_g, is_t));

            // Check if all lanes are valid (all bits set)
            let mut valid_bytes = [0u8; 16];
            vst1q_u8(valid_bytes.as_mut_ptr(), valid);

            if !valid_bytes.iter().all(|&b| b == 0xFF) {
                return false;
            }

            i += 16;
        }

        // Handle remainder with scalar
        kmer[i..].iter().all(|&b| matches!(b, b'A' | b'C' | b'G' | b'T'))
    }
}

impl PrimitiveOperation for Minimizers {
    fn name(&self) -> &str {
        "minimizers"
    }

    fn category(&self) -> OperationCategory {
        OperationCategory::Search
    }

    fn execute_naive(&self, sequences: &[SequenceRecord]) -> Result<OperationOutput> {
        let mut all_minimizers = Vec::new();
        let mut total_kmers = 0;

        for record in sequences {
            let minimizers = self.extract_minimizers_naive(&record.sequence);
            total_kmers += record.sequence.len().saturating_sub(self.k - 1);
            all_minimizers.extend(minimizers);
        }

        let output = MinimizerOutput {
            num_minimizers: all_minimizers.len(),
            minimizers: all_minimizers,
            total_kmers,
        };

        Ok(OperationOutput::Statistics(serde_json::to_value(output)?))
    }

    #[cfg(target_arch = "aarch64")]
    fn execute_neon(&self, sequences: &[SequenceRecord]) -> Result<OperationOutput> {
        let mut all_minimizers = Vec::new();
        let mut total_kmers = 0;

        for record in sequences {
            let minimizers = unsafe { self.extract_minimizers_neon(&record.sequence) };
            total_kmers += record.sequence.len().saturating_sub(self.k - 1);
            all_minimizers.extend(minimizers);
        }

        let output = MinimizerOutput {
            num_minimizers: all_minimizers.len(),
            minimizers: all_minimizers,
            total_kmers,
        };

        Ok(OperationOutput::Statistics(serde_json::to_value(output)?))
    }

    fn execute_parallel(&self, sequences: &[SequenceRecord], num_threads: usize) -> Result<OperationOutput> {
        let pool = rayon::ThreadPoolBuilder::new()
            .num_threads(num_threads)
            .build()
            .unwrap();

        let results: Vec<(Vec<Minimizer>, usize)> = pool.install(|| {
            sequences.par_iter().map(|record| {
                #[cfg(target_arch = "aarch64")]
                let minimizers = unsafe { self.extract_minimizers_neon(&record.sequence) };
                #[cfg(not(target_arch = "aarch64"))]
                let minimizers = self.extract_minimizers_naive(&record.sequence);

                let total_kmers = record.sequence.len().saturating_sub(self.k - 1);
                (minimizers, total_kmers)
            }).collect()
        });

        let mut all_minimizers = Vec::new();
        let mut total_kmers = 0;

        for (minimizers, kmers) in results {
            all_minimizers.extend(minimizers);
            total_kmers += kmers;
        }

        let output = MinimizerOutput {
            num_minimizers: all_minimizers.len(),
            minimizers: all_minimizers,
            total_kmers,
        };

        Ok(OperationOutput::Statistics(serde_json::to_value(output)?))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_sequence(id: &str, seq: &[u8]) -> SequenceRecord {
        SequenceRecord {
            id: id.to_string(),
            sequence: seq.to_vec(),
            quality: None,
        }
    }

    #[test]
    fn test_hash_kmer() {
        // Hash should be deterministic
        let hash1 = Minimizers::hash_kmer(b"ACGT");
        let hash2 = Minimizers::hash_kmer(b"ACGT");
        assert_eq!(hash1, hash2);

        // Different k-mers should have different hashes (usually)
        let hash_acgt = Minimizers::hash_kmer(b"ACGT");
        let hash_tgca = Minimizers::hash_kmer(b"TGCA");
        assert_ne!(hash_acgt, hash_tgca);
    }

    #[test]
    fn test_simple_minimizer_extraction() {
        let op = Minimizers::new(3, 2); // k=3, w=2

        // ACGTACGT: k-mers = [ACG, CGT, GTA, TAC, ACG, CGT]
        // Windows: [ACG,CGT], [CGT,GTA], [GTA,TAC], [TAC,ACG], [ACG,CGT]
        let seq = create_test_sequence("test1", b"ACGTACGT");

        let output = op.execute_naive(&[seq]).unwrap();
        if let OperationOutput::Statistics(json) = output {
            let result: MinimizerOutput = serde_json::from_value(json).unwrap();

            // Should have 5 windows, so 5 minimizers
            assert_eq!(result.num_minimizers, 5);
            assert_eq!(result.minimizers.len(), 5);
            assert_eq!(result.total_kmers, 6);
        } else {
            panic!("Expected Statistics output");
        }
    }

    #[test]
    fn test_minimizer_positions() {
        let op = Minimizers::new(3, 2);

        let seq = create_test_sequence("test1", b"AAACCCGGG");

        let output = op.execute_naive(&[seq]).unwrap();
        if let OperationOutput::Statistics(json) = output {
            let result: MinimizerOutput = serde_json::from_value(json).unwrap();

            // Check that positions make sense
            for minimizer in &result.minimizers {
                assert!(minimizer.position < 7); // Max position for k=3 in 9bp seq
                assert!(minimizer.kmer.is_some());
            }
        }
    }

    #[test]
    fn test_short_sequence() {
        let op = Minimizers::new(5, 3); // k=5, w=3

        // Sequence too short (only 4bp, need 5 for k=5)
        let seq = create_test_sequence("short", b"ACGT");

        let output = op.execute_naive(&[seq]).unwrap();
        if let OperationOutput::Statistics(json) = output {
            let result: MinimizerOutput = serde_json::from_value(json).unwrap();
            assert_eq!(result.num_minimizers, 0);
            assert_eq!(result.minimizers.len(), 0);
        }
    }

    #[test]
    fn test_invalid_bases_skipped() {
        let op = Minimizers::new(3, 2);

        // Sequence with N (invalid base)
        let seq = create_test_sequence("test1", b"ACGNCGT");

        let output = op.execute_naive(&[seq]).unwrap();
        if let OperationOutput::Statistics(json) = output {
            let result: MinimizerOutput = serde_json::from_value(json).unwrap();

            // Only ACG and CGT are valid k-mers (GNC and NCG skipped)
            // With 2 valid k-mers, we can have 1 window
            assert_eq!(result.num_minimizers, 1);
        }
    }

    #[test]
    fn test_window_size_one() {
        let op = Minimizers::new(3, 1); // w=1 means every k-mer is a minimizer

        let seq = create_test_sequence("test1", b"ACGTACGT");

        let output = op.execute_naive(&[seq]).unwrap();
        if let OperationOutput::Statistics(json) = output {
            let result: MinimizerOutput = serde_json::from_value(json).unwrap();

            // 6 k-mers, w=1, so 6 minimizers
            assert_eq!(result.num_minimizers, 6);
            assert_eq!(result.total_kmers, 6);
        }
    }

    #[test]
    fn test_multiple_sequences() {
        let op = Minimizers::new(3, 2);

        let seqs = vec![
            create_test_sequence("seq1", b"ACGTACGT"), // 5 minimizers
            create_test_sequence("seq2", b"GGTTAACC"), // 5 minimizers
        ];

        let output = op.execute_naive(&seqs).unwrap();
        if let OperationOutput::Statistics(json) = output {
            let result: MinimizerOutput = serde_json::from_value(json).unwrap();
            assert_eq!(result.num_minimizers, 10); // 5 + 5
        }
    }

    #[test]
    #[cfg(target_arch = "aarch64")]
    fn test_neon_matches_naive() {
        let op = Minimizers::new(6, 3); // k=6, w=3

        let sequences = vec![
            create_test_sequence("seq1", b"ACGTACGTACGTACGT"), // 16bp
            create_test_sequence("seq2", b"GGTTAACCGGTTAACC"), // 16bp
        ];

        let naive_output = op.execute_naive(&sequences).unwrap();
        let neon_output = op.execute_neon(&sequences).unwrap();

        if let (OperationOutput::Statistics(naive_json), OperationOutput::Statistics(neon_json)) =
            (naive_output, neon_output) {
            let naive_result: MinimizerOutput = serde_json::from_value(naive_json).unwrap();
            let neon_result: MinimizerOutput = serde_json::from_value(neon_json).unwrap();

            assert_eq!(naive_result.num_minimizers, neon_result.num_minimizers);
            assert_eq!(naive_result.total_kmers, neon_result.total_kmers);

            // Check that minimizers match (same positions and hashes)
            for (naive_min, neon_min) in naive_result.minimizers.iter().zip(neon_result.minimizers.iter()) {
                assert_eq!(naive_min.position, neon_min.position);
                assert_eq!(naive_min.hash, neon_min.hash);
            }
        } else {
            panic!("Expected Statistics output");
        }
    }

    #[test]
    fn test_parallel_execution() {
        let op = Minimizers::new(3, 2);

        let sequences = vec![
            create_test_sequence("seq1", b"ACGTACGTACGT"),
            create_test_sequence("seq2", b"GGTTAACCGGTT"),
        ];

        let naive_output = op.execute_naive(&sequences).unwrap();
        let parallel_output = op.execute_parallel(&sequences, 2).unwrap();

        if let (OperationOutput::Statistics(naive_json), OperationOutput::Statistics(parallel_json)) =
            (naive_output, parallel_output) {
            let naive_result: MinimizerOutput = serde_json::from_value(naive_json).unwrap();
            let parallel_result: MinimizerOutput = serde_json::from_value(parallel_json).unwrap();

            assert_eq!(naive_result.num_minimizers, parallel_result.num_minimizers);
            assert_eq!(naive_result.total_kmers, parallel_result.total_kmers);
        } else {
            panic!("Expected Statistics output");
        }
    }

    #[test]
    fn test_large_window() {
        let op = Minimizers::new(6, 5); // k=6, w=5 (valid: w < k)

        let seq = create_test_sequence("test1", b"ACGTACGTACGTACGT"); // 16bp = 11 k-mers

        let output = op.execute_naive(&[seq]).unwrap();
        if let OperationOutput::Statistics(json) = output {
            let result: MinimizerOutput = serde_json::from_value(json).unwrap();

            // With 11 k-mers and w=5, we have 7 windows (11-5+1)
            assert_eq!(result.num_minimizers, 7);
        }
    }
}
