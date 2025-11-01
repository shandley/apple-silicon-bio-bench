//! Hamming distance operation
//!
//! Computes Hamming distance (number of mismatches) between sequence pairs.
//!
//! **Operation Category**: Pairwise
//! - Vectorizable (compare 16 bases at once with NEON)
//! - Compute-bound (returns numbers, not modified sequences)
//! - Common in contamination screening, duplicate detection, clustering
//!
//! **Expected Performance** (based on Phase 1 + sequence_masking findings):
//! - Complexity: 0.35 (simple comparison + counting)
//! - NEON: ~20-40× speedup (compute-bound, returns aggregated numbers)
//! - Parallel: ~2-4× additional speedup on 4 cores
//!
//! # Apple Silicon Considerations
//!
//! - **NEON**: Process 16 bases per instruction
//! - **Vectorized comparison**: vceqq_u8 (compare equal, returns 0xFF/0x00)
//! - **Population count**: Count mismatches using vcntq_u8 after inversion
//! - **Memory pattern**: Sequential reads of both sequences (good cache behavior)
//!
//! # Hamming Distance Definition
//!
//! For two sequences of equal length, count positions where bases differ:
//! - Sequence 1: ACGTACGT
//! - Sequence 2: ACGTTCGT
//! - Hamming distance: 1 (position 4: A vs T)

use anyhow::Result;
use asbb_core::{OperationCategory, OperationOutput, PrimitiveOperation, SequenceRecord};
use rayon::prelude::*;
use serde::{Deserialize, Serialize};

/// Hamming distance operation
pub struct HammingDistance;

impl HammingDistance {
    pub fn new() -> Self {
        Self
    }

    /// Compute Hamming distance between two sequences (naive)
    fn distance_naive(&self, seq1: &[u8], seq2: &[u8]) -> Result<usize> {
        if seq1.len() != seq2.len() {
            anyhow::bail!(
                "Sequences must have equal length for Hamming distance: {} vs {}",
                seq1.len(),
                seq2.len()
            );
        }

        let mismatches = seq1
            .iter()
            .zip(seq2.iter())
            .filter(|(a, b)| a != b)
            .count();

        Ok(mismatches)
    }

    /// Compute Hamming distance between two sequences (NEON)
    #[cfg(target_arch = "aarch64")]
    fn distance_neon(&self, seq1: &[u8], seq2: &[u8]) -> Result<usize> {
        use std::arch::aarch64::*;

        if seq1.len() != seq2.len() {
            anyhow::bail!(
                "Sequences must have equal length for Hamming distance: {} vs {}",
                seq1.len(),
                seq2.len()
            );
        }

        let mut total_mismatches = 0usize;
        let mut i = 0;

        // Process 16 bases at a time with NEON
        while i + 16 <= seq1.len() {
            unsafe {
                // Load 16 bases from each sequence
                let vec1 = vld1q_u8(seq1.as_ptr().add(i));
                let vec2 = vld1q_u8(seq2.as_ptr().add(i));

                // Compare: returns 0xFF where equal, 0x00 where different
                let equal_mask = vceqq_u8(vec1, vec2);

                // Invert: now 0xFF where different, 0x00 where equal
                let diff_mask = vmvnq_u8(equal_mask);

                // Count bits set (each mismatch contributes 8 bits = 0xFF)
                // We need to count how many bytes are 0xFF
                // Use vcntq_u8 to count bits in each byte, then sum
                let bit_counts = vcntq_u8(diff_mask);

                // Horizontal add: sum all bytes in the vector
                // vpaddlq_u8: pairwise add 8-bit → 16-bit
                let sum16 = vpaddlq_u8(bit_counts);
                // vpaddlq_u16: pairwise add 16-bit → 32-bit
                let sum32 = vpaddlq_u16(sum16);
                // vpaddlq_u32: pairwise add 32-bit → 64-bit
                let sum64 = vpaddlq_u32(sum32);

                // Extract both 64-bit lanes and add
                let counts = vgetq_lane_u64(sum64, 0) + vgetq_lane_u64(sum64, 1);

                // Each mismatch byte (0xFF) contributes 8 bits, so divide by 8
                total_mismatches += (counts / 8) as usize;
            }

            i += 16;
        }

        // Process remaining bases (<16) with scalar code
        for j in i..seq1.len() {
            if seq1[j] != seq2[j] {
                total_mismatches += 1;
            }
        }

        Ok(total_mismatches)
    }

    #[cfg(not(target_arch = "aarch64"))]
    fn distance_neon(&self, seq1: &[u8], seq2: &[u8]) -> Result<usize> {
        // Fall back to naive on non-ARM
        self.distance_naive(seq1, seq2)
    }

    /// Compute all pairwise Hamming distances (N×N matrix)
    fn all_pairs_naive(&self, sequences: &[SequenceRecord]) -> Result<Vec<Vec<usize>>> {
        let n = sequences.len();
        let mut distances = vec![vec![0usize; n]; n];

        for i in 0..n {
            for j in (i + 1)..n {
                let dist = self.distance_naive(&sequences[i].sequence, &sequences[j].sequence)?;
                distances[i][j] = dist;
                distances[j][i] = dist; // Symmetric
            }
        }

        Ok(distances)
    }

    /// Compute all pairwise Hamming distances (N×N matrix, NEON)
    fn all_pairs_neon(&self, sequences: &[SequenceRecord]) -> Result<Vec<Vec<usize>>> {
        let n = sequences.len();
        let mut distances = vec![vec![0usize; n]; n];

        for i in 0..n {
            for j in (i + 1)..n {
                let dist = self.distance_neon(&sequences[i].sequence, &sequences[j].sequence)?;
                distances[i][j] = dist;
                distances[j][i] = dist; // Symmetric
            }
        }

        Ok(distances)
    }
}

impl Default for HammingDistance {
    fn default() -> Self {
        Self::new()
    }
}

impl PrimitiveOperation for HammingDistance {
    fn name(&self) -> &str {
        "hamming_distance"
    }

    fn category(&self) -> OperationCategory {
        OperationCategory::Pairwise
    }

    fn execute_naive(&self, data: &[SequenceRecord]) -> Result<OperationOutput> {
        let distances = self.all_pairs_naive(data)?;

        // Convert to JSON for output
        let result = HammingDistanceResult {
            num_sequences: data.len(),
            distances,
        };

        Ok(OperationOutput::Statistics(serde_json::to_value(result)?))
    }

    fn execute_neon(&self, data: &[SequenceRecord]) -> Result<OperationOutput> {
        let distances = self.all_pairs_neon(data)?;

        let result = HammingDistanceResult {
            num_sequences: data.len(),
            distances,
        };

        Ok(OperationOutput::Statistics(serde_json::to_value(result)?))
    }

    fn execute_parallel(
        &self,
        data: &[SequenceRecord],
        num_threads: usize,
    ) -> Result<OperationOutput> {
        let pool = rayon::ThreadPoolBuilder::new()
            .num_threads(num_threads)
            .build()?;

        pool.install(|| {
            let n = data.len();
            let mut distances = vec![vec![0usize; n]; n];

            // Parallelize over rows
            let results: Vec<(usize, Vec<usize>)> = (0..n)
                .into_par_iter()
                .map(|i| {
                    let mut row = vec![0usize; n];
                    for j in (i + 1)..n {
                        let dist =
                            self.distance_naive(&data[i].sequence, &data[j].sequence).unwrap();
                        row[j] = dist;
                    }
                    (i, row)
                })
                .collect();

            // Populate distance matrix
            for (i, row) in results {
                for j in (i + 1)..n {
                    distances[i][j] = row[j];
                    distances[j][i] = row[j]; // Symmetric
                }
            }

            let result = HammingDistanceResult {
                num_sequences: n,
                distances,
            };

            Ok(OperationOutput::Statistics(serde_json::to_value(result)?))
        })
    }
}

// ============================================================================
// Output Types
// ============================================================================

/// Result of Hamming distance computation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HammingDistanceResult {
    /// Number of sequences compared
    pub num_sequences: usize,
    /// N×N distance matrix (distances[i][j] = Hamming distance between seq i and seq j)
    pub distances: Vec<Vec<usize>>,
}

impl HammingDistanceResult {
    /// Get distance between two sequences
    pub fn get_distance(&self, i: usize, j: usize) -> Option<usize> {
        self.distances.get(i)?.get(j).copied()
    }

    /// Get mean distance across all pairs
    pub fn mean_distance(&self) -> f64 {
        if self.num_sequences < 2 {
            return 0.0;
        }

        let mut sum = 0usize;
        let mut count = 0usize;

        for i in 0..self.num_sequences {
            for j in (i + 1)..self.num_sequences {
                sum += self.distances[i][j];
                count += 1;
            }
        }

        sum as f64 / count as f64
    }

    /// Get minimum distance (closest pair)
    pub fn min_distance(&self) -> Option<usize> {
        if self.num_sequences < 2 {
            return None;
        }

        let mut min_dist = usize::MAX;

        for i in 0..self.num_sequences {
            for j in (i + 1)..self.num_sequences {
                min_dist = min_dist.min(self.distances[i][j]);
            }
        }

        Some(min_dist)
    }

    /// Get maximum distance (most distant pair)
    pub fn max_distance(&self) -> Option<usize> {
        if self.num_sequences < 2 {
            return None;
        }

        let mut max_dist = 0usize;

        for i in 0..self.num_sequences {
            for j in (i + 1)..self.num_sequences {
                max_dist = max_dist.max(self.distances[i][j]);
            }
        }

        Some(max_dist)
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_record(id: &str, sequence: &[u8]) -> SequenceRecord {
        SequenceRecord::fasta(id.to_string(), sequence.to_vec())
    }

    #[test]
    fn test_hamming_identical() {
        let op = HammingDistance::new();
        let seq1 = b"ACGTACGT";
        let seq2 = b"ACGTACGT";

        let dist = op.distance_naive(seq1, seq2).unwrap();
        assert_eq!(dist, 0);
    }

    #[test]
    fn test_hamming_one_mismatch() {
        let op = HammingDistance::new();
        let seq1 = b"ACGTACGT";
        let seq2 = b"ACGTTCGT";

        let dist = op.distance_naive(seq1, seq2).unwrap();
        assert_eq!(dist, 1);
    }

    #[test]
    fn test_hamming_all_mismatches() {
        let op = HammingDistance::new();
        let seq1 = b"AAAA";
        let seq2 = b"TTTT";

        let dist = op.distance_naive(seq1, seq2).unwrap();
        assert_eq!(dist, 4);
    }

    #[test]
    fn test_hamming_mixed() {
        let op = HammingDistance::new();
        let seq1 = b"ACGTACGT";
        let seq2 = b"TCGTACGA";

        let dist = op.distance_naive(seq1, seq2).unwrap();
        assert_eq!(dist, 2); // Position 0: A vs T, Position 7: T vs A
    }

    #[test]
    fn test_hamming_length_mismatch() {
        let op = HammingDistance::new();
        let seq1 = b"ACGT";
        let seq2 = b"ACGTACGT";

        let result = op.distance_naive(seq1, seq2);
        assert!(result.is_err());
    }

    #[test]
    #[cfg(target_arch = "aarch64")]
    fn test_neon_matches_naive() {
        let op = HammingDistance::new();

        let test_cases = vec![
            // Identical
            (b"ACGTACGTACGTACGT".to_vec(), b"ACGTACGTACGTACGT".to_vec(), 0),
            // One mismatch
            (b"ACGTACGTACGTACGT".to_vec(), b"TCGTACGTACGTACGT".to_vec(), 1),
            // Multiple mismatches
            (b"ACGTACGTACGTACGT".to_vec(), b"TCGAACGTACGTACGT".to_vec(), 2),
            // All mismatches
            (b"AAAAAAAAAAAAAAAA".to_vec(), b"TTTTTTTTTTTTTTTT".to_vec(), 16),
            // Longer sequence (32 bases, tests remainder handling)
            (
                b"ACGTACGTACGTACGTACGTACGTACGTACGT".to_vec(),
                b"TCGTACGTACGTACGTACGTACGTACGTACGT".to_vec(),
                1,
            ),
            // Even longer (35 bases, non-multiple of 16)
            (
                b"ACGTACGTACGTACGTACGTACGTACGTACGTACG".to_vec(),
                b"TCGTACGTACGTACGTACGTACGTACGTACGTACG".to_vec(),
                1,
            ),
        ];

        for (seq1, seq2, expected) in test_cases {
            let naive_dist = op.distance_naive(&seq1, &seq2).unwrap();
            let neon_dist = op.distance_neon(&seq1, &seq2).unwrap();

            assert_eq!(
                naive_dist, expected,
                "Naive distance incorrect for sequences: {:?} vs {:?}",
                String::from_utf8_lossy(&seq1),
                String::from_utf8_lossy(&seq2)
            );

            assert_eq!(
                neon_dist, naive_dist,
                "NEON must match naive for sequences: {:?} vs {:?}",
                String::from_utf8_lossy(&seq1),
                String::from_utf8_lossy(&seq2)
            );
        }
    }

    #[test]
    fn test_all_pairs() {
        let op = HammingDistance::new();

        let sequences = vec![
            create_test_record("seq1", b"ACGT"),
            create_test_record("seq2", b"ACGT"), // Identical to seq1
            create_test_record("seq3", b"TCGT"), // 1 mismatch from seq1
        ];

        let data = sequences;
        let result = op.execute_naive(&data).unwrap();

        if let OperationOutput::Statistics(json) = result {
            let result: HammingDistanceResult = serde_json::from_value(json).unwrap();

            assert_eq!(result.num_sequences, 3);
            assert_eq!(result.get_distance(0, 1), Some(0)); // seq1 vs seq2: identical
            assert_eq!(result.get_distance(0, 2), Some(1)); // seq1 vs seq3: 1 mismatch
            assert_eq!(result.get_distance(1, 2), Some(1)); // seq2 vs seq3: 1 mismatch

            // Symmetric
            assert_eq!(result.get_distance(1, 0), Some(0));
            assert_eq!(result.get_distance(2, 0), Some(1));

            // Diagonal (self-distance)
            assert_eq!(result.get_distance(0, 0), Some(0));
            assert_eq!(result.get_distance(1, 1), Some(0));
            assert_eq!(result.get_distance(2, 2), Some(0));
        } else {
            panic!("Expected Statistics output");
        }
    }

    #[test]
    fn test_distance_statistics() {
        let op = HammingDistance::new();

        let sequences = vec![
            create_test_record("seq1", b"AAAA"),
            create_test_record("seq2", b"AAAT"), // dist=1 from seq1
            create_test_record("seq3", b"AATT"), // dist=2 from seq1, dist=1 from seq2
            create_test_record("seq4", b"TTTT"), // dist=4 from seq1
        ];

        let result = op.execute_naive(&sequences).unwrap();

        if let OperationOutput::Statistics(json) = result {
            let result: HammingDistanceResult = serde_json::from_value(json).unwrap();

            assert_eq!(result.min_distance(), Some(1)); // Closest pair
            assert_eq!(result.max_distance(), Some(4)); // Most distant pair

            // Mean distance: (1 + 2 + 4 + 1 + 3 + 2) / 6 pairs = 13 / 6 ≈ 2.17
            let mean = result.mean_distance();
            assert!((mean - 2.17).abs() < 0.01);
        } else {
            panic!("Expected Statistics output");
        }
    }

    #[test]
    fn test_parallel_execution() {
        let op = HammingDistance::new();

        let sequences: Vec<SequenceRecord> = (0..10)
            .map(|i| {
                let seq = format!("ACGTACGTACGTACGT{}", if i % 2 == 0 { "A" } else { "T" });
                create_test_record(&format!("seq{}", i), seq.as_bytes())
            })
            .collect();

        let naive_result = op.execute_naive(&sequences).unwrap();
        let parallel_result = op.execute_parallel(&sequences, 4).unwrap();

        // Both should produce same distances
        match (naive_result, parallel_result) {
            (OperationOutput::Statistics(naive_json), OperationOutput::Statistics(parallel_json)) => {
                let naive: HammingDistanceResult = serde_json::from_value(naive_json).unwrap();
                let parallel: HammingDistanceResult =
                    serde_json::from_value(parallel_json).unwrap();

                assert_eq!(naive.num_sequences, parallel.num_sequences);
                assert_eq!(naive.distances, parallel.distances);
            }
            _ => panic!("Expected Statistics output"),
        }
    }
}
