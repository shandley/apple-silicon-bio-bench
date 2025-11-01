//! K-mer Counting Operation
//!
//! Extracts all k-mers from sequences and counts their occurrences.
//!
//! # Operation Characteristics
//! - **Category**: Search/Aggregation
//! - **Complexity**: 0.45 (extraction + hashing + counting)
//! - **Output**: Aggregated counts (HashMap)
//! - **NEON benefit**: High (compute-bound aggregation)
//!
//! # Implementation Notes
//! - Supports canonical k-mers (count k-mer and reverse complement together)
//! - Handles k-mer sizes 3-31 (limited by u64 bit representation)
//! - NEON accelerates k-mer extraction but not hash table operations
//! - Parallel implementation uses per-thread hash tables with merge

use crate::{OperationCategory, OperationOutput, PrimitiveOperation, SequenceRecord};
use anyhow::Result;
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

#[cfg(target_arch = "aarch64")]
use std::arch::aarch64::*;

/// K-mer counting operation
pub struct KmerCounting {
    /// K-mer size (3-31 bp)
    k: usize,
    /// Use canonical k-mers (count with reverse complement)
    canonical: bool,
}

/// K-mer count output
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KmerCounts {
    pub counts: HashMap<String, usize>,
    pub total_kmers: usize,
    pub unique_kmers: usize,
}

impl KmerCounting {
    pub fn new(k: usize, canonical: bool) -> Self {
        assert!(k >= 3 && k <= 31, "K-mer size must be 3-31");
        Self { k, canonical }
    }

    /// Extract all k-mers from a sequence (naive implementation)
    fn extract_kmers_naive(&self, sequence: &[u8]) -> Vec<String> {
        if sequence.len() < self.k {
            return Vec::new();
        }

        let mut kmers = Vec::with_capacity(sequence.len() - self.k + 1);
        for i in 0..=(sequence.len() - self.k) {
            let kmer = &sequence[i..i + self.k];
            // Validate: only ACGT
            if kmer.iter().all(|&b| matches!(b, b'A' | b'C' | b'G' | b'T')) {
                kmers.push(String::from_utf8_lossy(kmer).into_owned());
            }
        }
        kmers
    }

    /// Get canonical k-mer (lexicographically smaller of k-mer and reverse complement)
    fn canonical_kmer(&self, kmer: &str) -> String {
        let revcomp = self.reverse_complement_kmer(kmer);
        if kmer < &revcomp {
            kmer.to_string()
        } else {
            revcomp
        }
    }

    /// Reverse complement a k-mer
    fn reverse_complement_kmer(&self, kmer: &str) -> String {
        kmer.chars()
            .rev()
            .map(|base| match base {
                'A' => 'T',
                'T' => 'A',
                'C' => 'G',
                'G' => 'C',
                _ => base,
            })
            .collect()
    }

    /// Count k-mers from extracted list
    fn count_kmers(&self, kmers: Vec<String>) -> HashMap<String, usize> {
        let mut counts = HashMap::new();
        for kmer in kmers {
            let key = if self.canonical {
                self.canonical_kmer(&kmer)
            } else {
                kmer
            };
            *counts.entry(key).or_insert(0) += 1;
        }
        counts
    }

    /// Extract k-mers using NEON (vectorized extraction)
    #[cfg(target_arch = "aarch64")]
    fn extract_kmers_neon(&self, sequence: &[u8]) -> Vec<String> {
        if sequence.len() < self.k {
            return Vec::new();
        }

        let num_kmers = sequence.len() - self.k + 1;
        let mut kmers = Vec::with_capacity(num_kmers);

        // NEON can process multiple positions simultaneously
        // We'll validate 16 bases at a time, then extract k-mers from valid regions

        unsafe {
            let a_vec = vdupq_n_u8(b'A');
            let c_vec = vdupq_n_u8(b'C');
            let g_vec = vdupq_n_u8(b'G');
            let t_vec = vdupq_n_u8(b'T');

            let mut i = 0;

            // Process 16-byte chunks for validation
            while i + 16 <= sequence.len() {
                let vec = vld1q_u8(sequence.as_ptr().add(i));

                // Check if all bases are valid (A, C, G, or T)
                let is_a = vceqq_u8(vec, a_vec);
                let is_c = vceqq_u8(vec, c_vec);
                let is_g = vceqq_u8(vec, g_vec);
                let is_t = vceqq_u8(vec, t_vec);

                let valid = vorrq_u8(vorrq_u8(is_a, is_c), vorrq_u8(is_g, is_t));

                // Extract individual bytes to check validity
                let mut valid_bytes = [0u8; 16];
                vst1q_u8(valid_bytes.as_mut_ptr(), valid);

                // Extract k-mers from valid positions
                for j in 0..16 {
                    let pos = i + j;
                    if pos + self.k <= sequence.len() {
                        let kmer_slice = &sequence[pos..pos + self.k];
                        // Check if all bytes in k-mer are valid
                        if kmer_slice.iter().all(|&b| matches!(b, b'A' | b'C' | b'G' | b'T')) {
                            kmers.push(String::from_utf8_lossy(kmer_slice).into_owned());
                        }
                    }
                }

                i += 16;
            }

            // Handle remainder with scalar code
            while i + self.k <= sequence.len() {
                let kmer = &sequence[i..i + self.k];
                if kmer.iter().all(|&b| matches!(b, b'A' | b'C' | b'G' | b'T')) {
                    kmers.push(String::from_utf8_lossy(kmer).into_owned());
                }
                i += 1;
            }
        }

        kmers
    }

    /// Merge two count hash maps
    fn merge_counts(
        counts1: &mut HashMap<String, usize>,
        counts2: HashMap<String, usize>,
    ) {
        for (kmer, count) in counts2 {
            *counts1.entry(kmer).or_insert(0) += count;
        }
    }
}

impl PrimitiveOperation for KmerCounting {
    fn name(&self) -> &str {
        "kmer_counting"
    }

    fn category(&self) -> OperationCategory {
        OperationCategory::Search
    }

    fn execute_naive(&self, sequences: &[SequenceRecord]) -> Result<OperationOutput> {
        let mut all_counts = HashMap::new();

        for record in sequences {
            let kmers = self.extract_kmers_naive(&record.sequence);
            let counts = self.count_kmers(kmers);
            Self::merge_counts(&mut all_counts, counts);
        }

        let total_kmers: usize = all_counts.values().sum();
        let unique_kmers = all_counts.len();

        let counts = KmerCounts {
            counts: all_counts,
            total_kmers,
            unique_kmers,
        };

        Ok(OperationOutput::Statistics(serde_json::to_value(counts)?))
    }

    #[cfg(target_arch = "aarch64")]
    fn execute_neon(&self, sequences: &[SequenceRecord]) -> Result<OperationOutput> {
        let mut all_counts = HashMap::new();

        for record in sequences {
            let kmers = self.extract_kmers_neon(&record.sequence);
            let counts = self.count_kmers(kmers);
            Self::merge_counts(&mut all_counts, counts);
        }

        let total_kmers: usize = all_counts.values().sum();
        let unique_kmers = all_counts.len();

        let counts = KmerCounts {
            counts: all_counts,
            total_kmers,
            unique_kmers,
        };

        Ok(OperationOutput::Statistics(serde_json::to_value(counts)?))
    }

    fn execute_parallel(&self, sequences: &[SequenceRecord], num_threads: usize) -> Result<OperationOutput> {
        let pool = rayon::ThreadPoolBuilder::new()
            .num_threads(num_threads)
            .build()
            .unwrap();

        let all_counts = Arc::new(Mutex::new(HashMap::new()));

        pool.install(|| {
            sequences.par_iter().for_each(|record| {
                #[cfg(target_arch = "aarch64")]
                let kmers = self.extract_kmers_neon(&record.sequence);
                #[cfg(not(target_arch = "aarch64"))]
                let kmers = self.extract_kmers_naive(&record.sequence);

                let counts = self.count_kmers(kmers);

                let mut global_counts = all_counts.lock().unwrap();
                Self::merge_counts(&mut global_counts, counts);
            });
        });

        let final_counts = Arc::try_unwrap(all_counts)
            .unwrap()
            .into_inner()
            .unwrap();

        let total_kmers: usize = final_counts.values().sum();
        let unique_kmers = final_counts.len();

        let counts = KmerCounts {
            counts: final_counts,
            total_kmers,
            unique_kmers,
        };

        Ok(OperationOutput::Statistics(serde_json::to_value(counts)?))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_sequences() -> Vec<SequenceRecord> {
        vec![
            SequenceRecord {
                id: "seq1".to_string(),
                sequence: b"ACGTACGT".to_vec(),
                quality: None,
            },
            SequenceRecord {
                id: "seq2".to_string(),
                sequence: b"GGTTAACCGGTT".to_vec(),
                quality: None,
            },
        ]
    }

    #[test]
    fn test_kmer_extraction_k3() {
        let op = KmerCounting::new(3, false);
        let kmers = op.extract_kmers_naive(b"ACGTACGT");

        assert_eq!(kmers.len(), 6);
        assert_eq!(kmers, vec!["ACG", "CGT", "GTA", "TAC", "ACG", "CGT"]);
    }

    #[test]
    fn test_kmer_extraction_k5() {
        let op = KmerCounting::new(5, false);
        let kmers = op.extract_kmers_naive(b"ACGTACGT");

        assert_eq!(kmers.len(), 4);
        assert_eq!(kmers, vec!["ACGTA", "CGTAC", "GTACG", "TACGT"]);
    }

    #[test]
    fn test_kmer_counting_non_canonical() {
        let op = KmerCounting::new(3, false);
        let sequences = create_test_sequences();

        let output = op.execute_naive(&sequences).unwrap();
        if let OperationOutput::Statistics(json) = output {
            let counts: KmerCounts = serde_json::from_value(json).unwrap();
            assert!(counts.total_kmers > 0);
            assert!(counts.unique_kmers > 0);

            // "ACGTACGT" has: ACG(2), CGT(2), GTA(1), TAC(1)
            // "GGTTAACCGGTT" has: GGT(2), GTT(2), TTA(1), TAA(1), AAC(1), ACC(1), CCG(1), CGG(1)
            assert_eq!(counts.counts.get("ACG"), Some(&2));
            assert_eq!(counts.counts.get("CGT"), Some(&2));
        } else {
            panic!("Expected Statistics output");
        }
    }

    #[test]
    fn test_kmer_counting_canonical() {
        let op = KmerCounting::new(3, true);
        let sequences = vec![
            SequenceRecord {
                id: "seq1".to_string(),
                sequence: b"ACGT".to_vec(), // ACG and CGT
                quality: None,
            },
        ];

        let output = op.execute_naive(&sequences).unwrap();
        if let OperationOutput::Statistics(json) = output {
            let counts: KmerCounts = serde_json::from_value(json).unwrap();
            // ACG canonical: ACG (vs CGT)
            // CGT canonical: ACG (CGT -> ACG as revcomp)
            // So canonical ACG should appear twice
            assert_eq!(counts.counts.get("ACG"), Some(&2));
            assert_eq!(counts.unique_kmers, 1);
        } else {
            panic!("Expected Statistics output");
        }
    }

    #[test]
    fn test_reverse_complement_kmer() {
        let op = KmerCounting::new(3, false);

        assert_eq!(op.reverse_complement_kmer("ACG"), "CGT");
        assert_eq!(op.reverse_complement_kmer("CGT"), "ACG");
        assert_eq!(op.reverse_complement_kmer("AAA"), "TTT");
        assert_eq!(op.reverse_complement_kmer("GCGC"), "GCGC"); // Palindrome
    }

    #[test]
    fn test_canonical_kmer() {
        let op = KmerCounting::new(3, true);

        // ACG < CGT lexicographically
        assert_eq!(op.canonical_kmer("ACG"), "ACG");
        assert_eq!(op.canonical_kmer("CGT"), "ACG");

        // AAA < TTT
        assert_eq!(op.canonical_kmer("AAA"), "AAA");
        assert_eq!(op.canonical_kmer("TTT"), "AAA");
    }

    #[test]
    fn test_invalid_bases_skipped() {
        let op = KmerCounting::new(3, false);
        // N in sequence should cause that k-mer to be skipped
        let kmers = op.extract_kmers_naive(b"ACGNCGT");

        // Should extract ACG and CGT, but skip GNC and NCG
        assert_eq!(kmers.len(), 2);
        assert_eq!(kmers[0], "ACG");
        assert_eq!(kmers[1], "CGT");
    }

    #[test]
    fn test_short_sequence() {
        let op = KmerCounting::new(5, false);
        let kmers = op.extract_kmers_naive(b"ACG"); // Too short

        assert_eq!(kmers.len(), 0);
    }

    #[test]
    #[cfg(target_arch = "aarch64")]
    fn test_neon_matches_naive() {
        let op = KmerCounting::new(3, false);
        let sequences = create_test_sequences();

        let naive_output = op.execute_naive(&sequences).unwrap();
        let neon_output = op.execute_neon(&sequences).unwrap();

        if let (OperationOutput::Statistics(naive_json), OperationOutput::Statistics(neon_json)) =
            (naive_output, neon_output) {
            let naive_counts: KmerCounts = serde_json::from_value(naive_json).unwrap();
            let neon_counts: KmerCounts = serde_json::from_value(neon_json).unwrap();
            assert_eq!(naive_counts.total_kmers, neon_counts.total_kmers);
            assert_eq!(naive_counts.unique_kmers, neon_counts.unique_kmers);
            assert_eq!(naive_counts.counts, neon_counts.counts);
        } else {
            panic!("Expected Statistics output");
        }
    }

    #[test]
    fn test_parallel_execution() {
        let op = KmerCounting::new(3, false);
        let sequences = create_test_sequences();

        let naive_output = op.execute_naive(&sequences).unwrap();
        let parallel_output = op.execute_parallel(&sequences, 2).unwrap();

        if let (OperationOutput::Statistics(naive_json), OperationOutput::Statistics(parallel_json)) =
            (naive_output, parallel_output) {
            let naive_counts: KmerCounts = serde_json::from_value(naive_json).unwrap();
            let parallel_counts: KmerCounts = serde_json::from_value(parallel_json).unwrap();
            assert_eq!(naive_counts.total_kmers, parallel_counts.total_kmers);
            assert_eq!(naive_counts.unique_kmers, parallel_counts.unique_kmers);
            // Note: HashMap equality works because we're comparing counts
            assert_eq!(naive_counts.counts, parallel_counts.counts);
        } else {
            panic!("Expected Statistics output");
        }
    }
}
