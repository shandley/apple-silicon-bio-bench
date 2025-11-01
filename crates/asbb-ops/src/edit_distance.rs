//! Edit Distance (Levenshtein Distance) Operation
//!
//! Computes pairwise edit distance between sequences using dynamic programming.
//!
//! # Operation Characteristics
//! - **Category**: Pairwise
//! - **Complexity**: 0.70 (O(n²) dynamic programming)
//! - **Output**: Distance matrix (compute-bound)
//! - **NEON benefit**: Moderate (DP inner loop vectorizable, but memory access patterns complex)
//!
//! # Implementation Notes
//! - Classic Wagner-Fischer algorithm (dynamic programming)
//! - Computes insertion, deletion, substitution costs
//! - Space-optimized with rolling buffer (O(n) space)
//! - NEON accelerates DP row computation

use crate::{OperationCategory, OperationOutput, PrimitiveOperation, SequenceRecord};
use anyhow::Result;
use rayon::prelude::*;
use serde::{Deserialize, Serialize};

#[cfg(target_arch = "aarch64")]
use std::arch::aarch64::*;

/// Edit distance operation
pub struct EditDistance {
    /// Maximum sequences to compare (for N×N matrix)
    max_sequences: usize,
}

/// Edit distance result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EditDistanceMatrix {
    pub sequences: Vec<String>,
    pub distances: Vec<Vec<usize>>,
    pub num_sequences: usize,
}

impl EditDistance {
    pub fn new(max_sequences: usize) -> Self {
        Self { max_sequences }
    }

    /// Compute edit distance between two sequences (naive DP)
    fn distance_naive(&self, seq1: &[u8], seq2: &[u8]) -> usize {
        let len1 = seq1.len();
        let len2 = seq2.len();

        if len1 == 0 {
            return len2;
        }
        if len2 == 0 {
            return len1;
        }

        // Use two rows for space optimization (current and previous)
        let mut prev_row = vec![0; len2 + 1];
        let mut curr_row = vec![0; len2 + 1];

        // Initialize first row (0, 1, 2, 3, ...)
        for j in 0..=len2 {
            prev_row[j] = j;
        }

        // Fill DP table row by row
        for i in 1..=len1 {
            curr_row[0] = i; // Initialize first column

            for j in 1..=len2 {
                let cost = if seq1[i - 1] == seq2[j - 1] { 0 } else { 1 };

                curr_row[j] = std::cmp::min(
                    std::cmp::min(
                        prev_row[j] + 1,      // deletion
                        curr_row[j - 1] + 1,  // insertion
                    ),
                    prev_row[j - 1] + cost,   // substitution
                );
            }

            // Swap rows
            std::mem::swap(&mut prev_row, &mut curr_row);
        }

        prev_row[len2]
    }

    /// Compute edit distance using NEON (vectorized DP inner loop)
    #[cfg(target_arch = "aarch64")]
    fn distance_neon(&self, seq1: &[u8], seq2: &[u8]) -> usize {
        let len1 = seq1.len();
        let len2 = seq2.len();

        if len1 == 0 {
            return len2;
        }
        if len2 == 0 {
            return len1;
        }

        // For NEON optimization, we can vectorize the inner loop
        // However, DP has dependencies that make full vectorization complex
        // This implementation uses NEON for comparisons

        let mut prev_row = vec![0; len2 + 1];
        let mut curr_row = vec![0; len2 + 1];

        for j in 0..=len2 {
            prev_row[j] = j;
        }

        unsafe {
            for i in 1..=len1 {
                curr_row[0] = i;

                let base = seq1[i - 1];

                // Process multiple positions with NEON where possible
                let mut j = 1;

                // Vectorized comparison (8 bytes at a time)
                while j + 8 <= len2 {
                    // Load 8 bytes from seq2
                    let seq2_bytes = vld1_u8(seq2.as_ptr().add(j - 1));
                    let base_vec = vdup_n_u8(base);

                    // Compare
                    let equal_mask = vceq_u8(base_vec, seq2_bytes);

                    // Store mask to array for indexing
                    let mut mask_array = [0u8; 8];
                    vst1_u8(mask_array.as_mut_ptr(), equal_mask);

                    // Process each position
                    for k in 0..8 {
                        let cost = if mask_array[k] == 0xFF { 0 } else { 1 };

                        curr_row[j + k] = std::cmp::min(
                            std::cmp::min(
                                prev_row[j + k] + 1,
                                curr_row[j + k - 1] + 1,
                            ),
                            prev_row[j + k - 1] + cost,
                        );
                    }

                    j += 8;
                }

                // Handle remainder
                while j <= len2 {
                    let cost = if seq1[i - 1] == seq2[j - 1] { 0 } else { 1 };

                    curr_row[j] = std::cmp::min(
                        std::cmp::min(
                            prev_row[j] + 1,
                            curr_row[j - 1] + 1,
                        ),
                        prev_row[j - 1] + cost,
                    );

                    j += 1;
                }

                std::mem::swap(&mut prev_row, &mut curr_row);
            }
        }

        prev_row[len2]
    }
}

impl PrimitiveOperation for EditDistance {
    fn name(&self) -> &str {
        "edit_distance"
    }

    fn category(&self) -> OperationCategory {
        OperationCategory::Pairwise
    }

    fn execute_naive(&self, sequences: &[SequenceRecord]) -> Result<OperationOutput> {
        let num_seqs = std::cmp::min(sequences.len(), self.max_sequences);
        let sequences = &sequences[..num_seqs];

        let mut distances = vec![vec![0; num_seqs]; num_seqs];

        for i in 0..num_seqs {
            for j in i..num_seqs {
                if i == j {
                    distances[i][j] = 0;
                } else {
                    let dist = self.distance_naive(&sequences[i].sequence, &sequences[j].sequence);
                    distances[i][j] = dist;
                    distances[j][i] = dist; // Symmetric
                }
            }
        }

        let result = EditDistanceMatrix {
            sequences: sequences.iter().map(|s| s.id.clone()).collect(),
            distances,
            num_sequences: num_seqs,
        };

        Ok(OperationOutput::Statistics(serde_json::to_value(result)?))
    }

    #[cfg(target_arch = "aarch64")]
    fn execute_neon(&self, sequences: &[SequenceRecord]) -> Result<OperationOutput> {
        let num_seqs = std::cmp::min(sequences.len(), self.max_sequences);
        let sequences = &sequences[..num_seqs];

        let mut distances = vec![vec![0; num_seqs]; num_seqs];

        for i in 0..num_seqs {
            for j in i..num_seqs {
                if i == j {
                    distances[i][j] = 0;
                } else {
                    let dist = self.distance_neon(&sequences[i].sequence, &sequences[j].sequence);
                    distances[i][j] = dist;
                    distances[j][i] = dist;
                }
            }
        }

        let result = EditDistanceMatrix {
            sequences: sequences.iter().map(|s| s.id.clone()).collect(),
            distances,
            num_sequences: num_seqs,
        };

        Ok(OperationOutput::Statistics(serde_json::to_value(result)?))
    }

    fn execute_parallel(&self, sequences: &[SequenceRecord], num_threads: usize) -> Result<OperationOutput> {
        let num_seqs = std::cmp::min(sequences.len(), self.max_sequences);
        let sequences = &sequences[..num_seqs];

        let pool = rayon::ThreadPoolBuilder::new()
            .num_threads(num_threads)
            .build()
            .unwrap();

        // Compute upper triangle in parallel
        let pairs: Vec<(usize, usize)> = (0..num_seqs)
            .flat_map(|i| (i + 1..num_seqs).map(move |j| (i, j)))
            .collect();

        let distances_vec: Vec<((usize, usize), usize)> = pool.install(|| {
            pairs.par_iter().map(|&(i, j)| {
                #[cfg(target_arch = "aarch64")]
                let dist = self.distance_neon(&sequences[i].sequence, &sequences[j].sequence);
                #[cfg(not(target_arch = "aarch64"))]
                let dist = self.distance_naive(&sequences[i].sequence, &sequences[j].sequence);

                ((i, j), dist)
            }).collect()
        });

        // Build matrix
        let mut distances = vec![vec![0; num_seqs]; num_seqs];
        for ((i, j), dist) in distances_vec {
            distances[i][j] = dist;
            distances[j][i] = dist;
        }

        let result = EditDistanceMatrix {
            sequences: sequences.iter().map(|s| s.id.clone()).collect(),
            distances,
            num_sequences: num_seqs,
        };

        Ok(OperationOutput::Statistics(serde_json::to_value(result)?))
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
    fn test_identical_sequences() {
        let op = EditDistance::new(10);

        let seq1 = create_test_sequence("seq1", b"ACGT");
        let seq2 = create_test_sequence("seq2", b"ACGT");

        let output = op.execute_naive(&[seq1, seq2]).unwrap();
        if let OperationOutput::Statistics(json) = output {
            let result: EditDistanceMatrix = serde_json::from_value(json).unwrap();
            assert_eq!(result.distances[0][1], 0);
            assert_eq!(result.distances[1][0], 0);
        } else {
            panic!("Expected Statistics output");
        }
    }

    #[test]
    fn test_single_substitution() {
        let op = EditDistance::new(10);

        let seq1 = create_test_sequence("seq1", b"ACGT");
        let seq2 = create_test_sequence("seq2", b"ACCT");

        let output = op.execute_naive(&[seq1, seq2]).unwrap();
        if let OperationOutput::Statistics(json) = output {
            let result: EditDistanceMatrix = serde_json::from_value(json).unwrap();
            assert_eq!(result.distances[0][1], 1); // One substitution (G -> C)
        }
    }

    #[test]
    fn test_single_insertion() {
        let op = EditDistance::new(10);

        let seq1 = create_test_sequence("seq1", b"ACGT");
        let seq2 = create_test_sequence("seq2", b"ACGGT");

        let output = op.execute_naive(&[seq1, seq2]).unwrap();
        if let OperationOutput::Statistics(json) = output {
            let result: EditDistanceMatrix = serde_json::from_value(json).unwrap();
            assert_eq!(result.distances[0][1], 1); // One insertion
        }
    }

    #[test]
    fn test_single_deletion() {
        let op = EditDistance::new(10);

        let seq1 = create_test_sequence("seq1", b"ACGGT");
        let seq2 = create_test_sequence("seq2", b"ACGT");

        let output = op.execute_naive(&[seq1, seq2]).unwrap();
        if let OperationOutput::Statistics(json) = output {
            let result: EditDistanceMatrix = serde_json::from_value(json).unwrap();
            assert_eq!(result.distances[0][1], 1); // One deletion
        }
    }

    #[test]
    fn test_empty_sequences() {
        let op = EditDistance::new(10);

        let seq1 = create_test_sequence("seq1", b"ACGT");
        let seq2 = create_test_sequence("seq2", b"");

        let output = op.execute_naive(&[seq1, seq2]).unwrap();
        if let OperationOutput::Statistics(json) = output {
            let result: EditDistanceMatrix = serde_json::from_value(json).unwrap();
            assert_eq!(result.distances[0][1], 4); // Delete all 4 bases
        }
    }

    #[test]
    fn test_completely_different() {
        let op = EditDistance::new(10);

        let seq1 = create_test_sequence("seq1", b"AAAA");
        let seq2 = create_test_sequence("seq2", b"TTTT");

        let output = op.execute_naive(&[seq1, seq2]).unwrap();
        if let OperationOutput::Statistics(json) = output {
            let result: EditDistanceMatrix = serde_json::from_value(json).unwrap();
            assert_eq!(result.distances[0][1], 4); // All substitutions
        }
    }

    #[test]
    fn test_matrix_symmetry() {
        let op = EditDistance::new(10);

        let sequences = vec![
            create_test_sequence("seq1", b"ACGT"),
            create_test_sequence("seq2", b"ACCT"),
            create_test_sequence("seq3", b"GGTT"),
        ];

        let output = op.execute_naive(&sequences).unwrap();
        if let OperationOutput::Statistics(json) = output {
            let result: EditDistanceMatrix = serde_json::from_value(json).unwrap();

            // Check symmetry
            for i in 0..result.num_sequences {
                for j in 0..result.num_sequences {
                    assert_eq!(result.distances[i][j], result.distances[j][i]);
                }
            }

            // Diagonal should be zero
            for i in 0..result.num_sequences {
                assert_eq!(result.distances[i][i], 0);
            }
        }
    }

    #[test]
    fn test_max_sequences_limit() {
        let op = EditDistance::new(2);

        let sequences = vec![
            create_test_sequence("seq1", b"ACGT"),
            create_test_sequence("seq2", b"ACCT"),
            create_test_sequence("seq3", b"GGTT"),
        ];

        let output = op.execute_naive(&sequences).unwrap();
        if let OperationOutput::Statistics(json) = output {
            let result: EditDistanceMatrix = serde_json::from_value(json).unwrap();
            assert_eq!(result.num_sequences, 2); // Limited to 2
        }
    }

    #[test]
    #[cfg(target_arch = "aarch64")]
    fn test_neon_matches_naive() {
        let op = EditDistance::new(10);

        let sequences = vec![
            create_test_sequence("seq1", b"ACGTACGTACGT"),
            create_test_sequence("seq2", b"ACCTACGTACGT"),
            create_test_sequence("seq3", b"GGTTAACCGGTT"),
        ];

        let naive_output = op.execute_naive(&sequences).unwrap();
        let neon_output = op.execute_neon(&sequences).unwrap();

        if let (OperationOutput::Statistics(naive_json), OperationOutput::Statistics(neon_json)) =
            (naive_output, neon_output) {
            let naive_result: EditDistanceMatrix = serde_json::from_value(naive_json).unwrap();
            let neon_result: EditDistanceMatrix = serde_json::from_value(neon_json).unwrap();

            assert_eq!(naive_result.distances, neon_result.distances);
        } else {
            panic!("Expected Statistics output");
        }
    }

    #[test]
    fn test_parallel_execution() {
        let op = EditDistance::new(10);

        let sequences = vec![
            create_test_sequence("seq1", b"ACGTACGT"),
            create_test_sequence("seq2", b"ACCTACGT"),
            create_test_sequence("seq3", b"GGTTAACC"),
        ];

        let naive_output = op.execute_naive(&sequences).unwrap();
        let parallel_output = op.execute_parallel(&sequences, 2).unwrap();

        if let (OperationOutput::Statistics(naive_json), OperationOutput::Statistics(parallel_json)) =
            (naive_output, parallel_output) {
            let naive_result: EditDistanceMatrix = serde_json::from_value(naive_json).unwrap();
            let parallel_result: EditDistanceMatrix = serde_json::from_value(parallel_json).unwrap();

            assert_eq!(naive_result.distances, parallel_result.distances);
        } else {
            panic!("Expected Statistics output");
        }
    }
}
