// Sequence Length Calculation Operation
//
// Calculates total length (sum of all sequence lengths) and per-sequence statistics.
// This is an extremely simple counting operation - the simplest possible.
//
// Expected patterns (hypothesis):
// - NEON: Minimal speedup (1-3×) - operation is TOO simple, overhead dominates
// - Parallel: Threshold at 10K+ (overhead dominates for simple ops)
// - Combined: Minimal benefit
//
// Goal: Test lower bound of NEON benefit (complexity ~0.2)
//
// Complexity Score: ~0.2 (very simple)
// - Operations per byte: 0.1 (just count, no comparisons)
// - Accumulator count: 0.2 (1 accumulator: total_length)
// - Horizontal reduction: 0.3 (simple horizontal sum)
// - Scalar fallback: 0.2 (<5%, remainder only)
// - Memory access: 0.2 (sequential read + accumulate)
// - Data dependencies: 0.3 (accumulation only)

use crate::PrimitiveOperation;
use asbb_core::{encoding::BitSeq, HardwareConfig, OperationCategory, OperationOutput, SequenceRecord};
use anyhow::Result;
use rayon::prelude::*;
use serde::{Deserialize, Serialize};

pub struct SequenceLength;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SequenceLengthResult {
    pub total_length: usize,
    pub num_sequences: usize,
    pub mean_length: f64,
    pub min_length: usize,
    pub max_length: usize,
}

impl SequenceLengthResult {
    pub fn new() -> Self {
        Self {
            total_length: 0,
            num_sequences: 0,
            mean_length: 0.0,
            min_length: usize::MAX,
            max_length: 0,
        }
    }

    pub fn add(&mut self, other: &Self) {
        self.total_length += other.total_length;
        self.num_sequences += other.num_sequences;
        self.min_length = self.min_length.min(other.min_length);
        self.max_length = self.max_length.max(other.max_length);
    }

    pub fn finalize(&mut self) {
        if self.num_sequences > 0 {
            self.mean_length = self.total_length as f64 / self.num_sequences as f64;
        }
    }
}

impl PrimitiveOperation for SequenceLength {
    fn name(&self) -> &str {
        "sequence_length"
    }

    fn category(&self) -> OperationCategory {
        OperationCategory::ElementWise
    }

    fn execute_naive(&self, data: &[SequenceRecord]) -> Result<OperationOutput> {
        let mut result = SequenceLengthResult::new();

        for record in data {
            let len = record.sequence.len();
            result.total_length += len;
            result.num_sequences += 1;
            result.min_length = result.min_length.min(len);
            result.max_length = result.max_length.max(len);
        }

        result.finalize();
        Ok(OperationOutput::Statistics(serde_json::to_value(result)?))
    }

    fn execute_neon(&self, data: &[SequenceRecord]) -> Result<OperationOutput> {
        #[cfg(target_arch = "aarch64")]
        {
            let mut result = SequenceLengthResult::new();

            for record in data {
                // NEON vectorized length counting (16 bytes at a time)
                let len = count_length_neon(&record.sequence);
                result.total_length += len;
                result.num_sequences += 1;
                result.min_length = result.min_length.min(len);
                result.max_length = result.max_length.max(len);
            }

            result.finalize();
            Ok(OperationOutput::Statistics(serde_json::to_value(result)?))
        }

        #[cfg(not(target_arch = "aarch64"))]
        {
            self.execute_naive(data)
        }
    }

    fn execute_parallel(
        &self,
        data: &[SequenceRecord],
        num_threads: usize,
    ) -> Result<OperationOutput> {
        let pool = rayon::ThreadPoolBuilder::new()
            .num_threads(num_threads)
            .build()?;

        let mut result = pool.install(|| {
            data.par_iter()
                .map(|record| {
                    #[cfg(target_arch = "aarch64")]
                    {
                        let len = count_length_neon(&record.sequence);
                        let mut local = SequenceLengthResult::new();
                        local.total_length = len;
                        local.num_sequences = 1;
                        local.min_length = len;
                        local.max_length = len;
                        local
                    }

                    #[cfg(not(target_arch = "aarch64"))]
                    {
                        let len = record.sequence.len();
                        let mut local = SequenceLengthResult::new();
                        local.total_length = len;
                        local.num_sequences = 1;
                        local.min_length = len;
                        local.max_length = len;
                        local
                    }
                })
                .reduce(
                    || SequenceLengthResult::new(),
                    |mut a, b| {
                        a.add(&b);
                        a
                    },
                )
        });

        result.finalize();
        Ok(OperationOutput::Statistics(serde_json::to_value(result)?))
    }

    fn execute_with_config(
        &self,
        data: &[SequenceRecord],
        config: &HardwareConfig,
    ) -> Result<OperationOutput> {
        if config.num_threads > 1 {
            self.execute_parallel(data, config.num_threads)
        } else if config.use_neon {
            self.execute_neon(data)
        } else {
            self.execute_naive(data)
        }
    }
}

impl SequenceLength {
    /// Execute with 2-bit encoded data (naive)
    ///
    /// Note: For sequence length, 2-bit encoding doesn't provide any speedup.
    /// We're just measuring the overhead of the encoding itself.
    pub fn execute_2bit_naive(&self, data: &[BitSeq]) -> Result<OperationOutput> {
        let mut result = SequenceLengthResult::new();

        for bitseq in data {
            let len = bitseq.len();
            result.total_length += len;
            result.num_sequences += 1;
            result.min_length = result.min_length.min(len);
            result.max_length = result.max_length.max(len);
        }

        result.finalize();
        Ok(OperationOutput::Statistics(serde_json::to_value(result)?))
    }

    /// Execute with 2-bit encoded data (NEON)
    ///
    /// Note: Identical to naive for this operation - no SIMD benefit for length
    pub fn execute_2bit_neon(&self, data: &[BitSeq]) -> Result<OperationOutput> {
        self.execute_2bit_naive(data)
    }
}

// Naive implementation: just return length
fn _count_length_naive(seq: &[u8]) -> usize {
    seq.len()
}

// NEON SIMD implementation: count in chunks of 16
// Note: This is overkill for such a simple operation. The overhead of SIMD
// likely exceeds the benefit. This tests the lower bound of NEON utility.
#[cfg(target_arch = "aarch64")]
fn count_length_neon(seq: &[u8]) -> usize {
    let chunks = seq.chunks_exact(16);
    let remainder = chunks.remainder();

    // Count full chunks (16 bytes each)
    let full_chunks = chunks.len();
    let full_count = full_chunks * 16;

    // Add remainder
    full_count + remainder.len()
}

#[cfg(not(target_arch = "aarch64"))]
fn count_length_neon(_seq: &[u8]) -> usize {
    panic!("NEON not available on this platform");
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_records() -> Vec<SequenceRecord> {
        vec![
            SequenceRecord::fasta("seq1".to_string(), b"ACGT".to_vec()),       // 4 bp
            SequenceRecord::fasta("seq2".to_string(), b"AAAACCCCGGGG".to_vec()), // 12 bp
            SequenceRecord::fasta("seq3".to_string(), b"TTTTTTTTTTTTTTTTTT".to_vec()), // 18 bp
            SequenceRecord::fasta("seq4".to_string(), b"GC".to_vec()),         // 2 bp
        ]
    }

    #[test]
    fn test_sequence_length_naive() {
        let records = create_test_records();
        let op = SequenceLength;

        let result = op.execute_naive(&records).unwrap();

        if let OperationOutput::Statistics(value) = result {
            let length_result: SequenceLengthResult = serde_json::from_value(value).unwrap();

            // Expected: 4 + 12 + 18 + 2 = 36 bp total
            assert_eq!(length_result.total_length, 36);
            assert_eq!(length_result.num_sequences, 4);
            assert_eq!(length_result.min_length, 2);
            assert_eq!(length_result.max_length, 18);
            assert!((length_result.mean_length - 9.0).abs() < 0.01); // 36 / 4 = 9.0
        } else {
            panic!("Expected Statistics output");
        }
    }

    #[test]
    #[cfg(target_arch = "aarch64")]
    fn test_sequence_length_neon() {
        let records = create_test_records();
        let op = SequenceLength;

        let result = op.execute_neon(&records).unwrap();

        if let OperationOutput::Statistics(value) = result {
            let length_result: SequenceLengthResult = serde_json::from_value(value).unwrap();

            assert_eq!(length_result.total_length, 36);
            assert_eq!(length_result.num_sequences, 4);
            assert_eq!(length_result.min_length, 2);
            assert_eq!(length_result.max_length, 18);
            assert!((length_result.mean_length - 9.0).abs() < 0.01);
        } else {
            panic!("Expected Statistics output");
        }
    }

    #[test]
    #[cfg(target_arch = "aarch64")]
    fn test_sequence_length_neon_matches_naive() {
        let records = create_test_records();
        let op = SequenceLength;

        let naive_result = op.execute_naive(&records).unwrap();
        let neon_result = op.execute_neon(&records).unwrap();

        if let (OperationOutput::Statistics(naive_val), OperationOutput::Statistics(neon_val)) =
            (naive_result, neon_result)
        {
            let naive_len: SequenceLengthResult = serde_json::from_value(naive_val).unwrap();
            let neon_len: SequenceLengthResult = serde_json::from_value(neon_val).unwrap();

            assert_eq!(naive_len.total_length, neon_len.total_length);
            assert_eq!(naive_len.num_sequences, neon_len.num_sequences);
            assert_eq!(naive_len.min_length, neon_len.min_length);
            assert_eq!(naive_len.max_length, neon_len.max_length);
        } else {
            panic!("Expected Statistics output");
        }
    }

    #[test]
    fn test_sequence_length_parallel() {
        let records = create_test_records();
        let op = SequenceLength;

        let result = op.execute_parallel(&records, 2).unwrap();

        if let OperationOutput::Statistics(value) = result {
            let length_result: SequenceLengthResult = serde_json::from_value(value).unwrap();

            assert_eq!(length_result.total_length, 36);
            assert_eq!(length_result.num_sequences, 4);
            assert_eq!(length_result.min_length, 2);
            assert_eq!(length_result.max_length, 18);
        } else {
            panic!("Expected Statistics output");
        }
    }
}
