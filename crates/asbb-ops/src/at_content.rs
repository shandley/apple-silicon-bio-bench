// AT Content Calculation Operation
//
// Calculates AT content (percentage of A and T bases) in sequences.
// Very similar to GC content but for AT bases.
//
// Expected patterns:
// - NEON: 20-40× at tiny, 12-16× at large (similar to GC content)
// - Parallel: Threshold at 1K sequences (simple counting)
// - Combined: 40-75× at large scale
//
// Goal: N=7 validation, confirm simple counting pattern with AT vs GC
//
// Complexity Score: ~0.35 (simple)
// - Operations per byte: 0.35 (4 comparisons: A, T, upper/lower)
// - Accumulator count: 0.4 (2 accumulators: at_count, total)
// - Horizontal reduction: 0.3 (2 simple horizontal sums)
// - Scalar fallback: 0.2 (<5%, remainder only)
// - Memory access: 0.2 (sequential read + accumulate)
// - Data dependencies: 0.3 (accumulation only)

use crate::PrimitiveOperation;
use asbb_core::{HardwareConfig, OperationCategory, OperationOutput, SequenceRecord};
use anyhow::Result;
use rayon::prelude::*;
use serde::{Deserialize, Serialize};

pub struct ATContent;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ATContentResult {
    pub at_count: usize,
    pub total_bases: usize,
    pub at_percent: f64,
}

impl ATContentResult {
    pub fn new() -> Self {
        Self {
            at_count: 0,
            total_bases: 0,
            at_percent: 0.0,
        }
    }

    pub fn add(&mut self, other: &Self) {
        self.at_count += other.at_count;
        self.total_bases += other.total_bases;
    }

    pub fn finalize(&mut self) {
        if self.total_bases > 0 {
            self.at_percent = (self.at_count as f64 / self.total_bases as f64) * 100.0;
        }
    }
}

impl PrimitiveOperation for ATContent {
    fn name(&self) -> &str {
        "at_content"
    }

    fn category(&self) -> OperationCategory {
        OperationCategory::ElementWise
    }

    fn execute_naive(&self, data: &[SequenceRecord]) -> Result<OperationOutput> {
        let mut result = ATContentResult::new();

        for record in data {
            let local_result = count_at_naive(&record.sequence);
            result.add(&local_result);
        }

        result.finalize();
        Ok(OperationOutput::Statistics(serde_json::to_value(result)?))
    }

    fn execute_neon(&self, data: &[SequenceRecord]) -> Result<OperationOutput> {
        #[cfg(target_arch = "aarch64")]
        {
            let mut result = ATContentResult::new();

            for record in data {
                let local_result = count_at_neon(&record.sequence);
                result.add(&local_result);
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
                        count_at_neon(&record.sequence)
                    }

                    #[cfg(not(target_arch = "aarch64"))]
                    {
                        count_at_naive(&record.sequence)
                    }
                })
                .reduce(
                    || ATContentResult::new(),
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

// Naive implementation
fn count_at_naive(seq: &[u8]) -> ATContentResult {
    let mut result = ATContentResult::new();
    result.total_bases = seq.len();

    for &base in seq {
        if matches!(base, b'A' | b'a' | b'T' | b't') {
            result.at_count += 1;
        }
    }

    result
}

// NEON SIMD implementation
#[cfg(target_arch = "aarch64")]
fn count_at_neon(seq: &[u8]) -> ATContentResult {
    use std::arch::aarch64::*;

    let mut result = ATContentResult::new();
    result.total_bases = seq.len();

    unsafe {
        // NEON accumulator for AT count
        let mut vec_at_count = vdupq_n_u8(0);

        // Comparison values for A and T (upper and lower case)
        let cmp_a_upper = vdupq_n_u8(b'A');
        let cmp_a_lower = vdupq_n_u8(b'a');
        let cmp_t_upper = vdupq_n_u8(b'T');
        let cmp_t_lower = vdupq_n_u8(b't');

        let ones = vdupq_n_u8(1);

        let chunks = seq.chunks_exact(16);
        let remainder = chunks.remainder();

        // Process 16 bytes at a time
        for chunk in chunks {
            let data = vld1q_u8(chunk.as_ptr());

            // Check for A (upper or lower)
            let mask_a = vorrq_u8(vceqq_u8(data, cmp_a_upper), vceqq_u8(data, cmp_a_lower));

            // Check for T (upper or lower)
            let mask_t = vorrq_u8(vceqq_u8(data, cmp_t_upper), vceqq_u8(data, cmp_t_lower));

            // Combine A and T masks
            let mask_at = vorrq_u8(mask_a, mask_t);

            // Accumulate (mask is 0xFF for match, 0x00 for no match, so AND with 1)
            vec_at_count = vaddq_u8(vec_at_count, vandq_u8(mask_at, ones));
        }

        // Horizontal sum: reduce vector to scalar
        result.at_count = horizontal_sum_u8(vec_at_count);

        // Process remainder with scalar code
        for &base in remainder {
            if matches!(base, b'A' | b'a' | b'T' | b't') {
                result.at_count += 1;
            }
        }
    }

    result
}

#[cfg(target_arch = "aarch64")]
fn horizontal_sum_u8(vec: std::arch::aarch64::uint8x16_t) -> usize {
    unsafe {
        let bytes: [u8; 16] = std::mem::transmute(vec);
        bytes.iter().map(|&b| b as usize).sum()
    }
}

#[cfg(not(target_arch = "aarch64"))]
fn count_at_neon(_seq: &[u8]) -> ATContentResult {
    panic!("NEON not available on this platform");
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_records() -> Vec<SequenceRecord> {
        vec![
            SequenceRecord::fasta("seq1".to_string(), b"ACGT".to_vec()),       // 2 AT, 2 GC
            SequenceRecord::fasta("seq2".to_string(), b"AAAATTTT".to_vec()),   // 8 AT, 0 GC
            SequenceRecord::fasta("seq3".to_string(), b"GGGGCCCC".to_vec()),   // 0 AT, 8 GC
            SequenceRecord::fasta("seq4".to_string(), b"ATATatat".to_vec()),   // 8 AT, 0 GC (mixed case)
        ]
    }

    #[test]
    fn test_at_content_naive() {
        let records = create_test_records();
        let op = ATContent;

        let result = op.execute_naive(&records).unwrap();

        if let OperationOutput::Statistics(value) = result {
            let at_result: ATContentResult = serde_json::from_value(value).unwrap();

            // Expected: 2 + 8 + 0 + 8 = 18 AT bases out of 28 total
            assert_eq!(at_result.at_count, 18);
            assert_eq!(at_result.total_bases, 28);
            assert!((at_result.at_percent - 64.29).abs() < 0.1); // 18/28 * 100 ≈ 64.29%
        } else {
            panic!("Expected Statistics output");
        }
    }

    #[test]
    #[cfg(target_arch = "aarch64")]
    fn test_at_content_neon() {
        let records = create_test_records();
        let op = ATContent;

        let result = op.execute_neon(&records).unwrap();

        if let OperationOutput::Statistics(value) = result {
            let at_result: ATContentResult = serde_json::from_value(value).unwrap();

            assert_eq!(at_result.at_count, 18);
            assert_eq!(at_result.total_bases, 28);
            assert!((at_result.at_percent - 64.29).abs() < 0.1);
        } else {
            panic!("Expected Statistics output");
        }
    }

    #[test]
    #[cfg(target_arch = "aarch64")]
    fn test_at_content_neon_matches_naive() {
        let records = create_test_records();
        let op = ATContent;

        let naive_result = op.execute_naive(&records).unwrap();
        let neon_result = op.execute_neon(&records).unwrap();

        if let (OperationOutput::Statistics(naive_val), OperationOutput::Statistics(neon_val)) =
            (naive_result, neon_result)
        {
            let naive_at: ATContentResult = serde_json::from_value(naive_val).unwrap();
            let neon_at: ATContentResult = serde_json::from_value(neon_val).unwrap();

            assert_eq!(naive_at.at_count, neon_at.at_count);
            assert_eq!(naive_at.total_bases, neon_at.total_bases);
        } else {
            panic!("Expected Statistics output");
        }
    }

    #[test]
    fn test_at_content_parallel() {
        let records = create_test_records();
        let op = ATContent;

        let result = op.execute_parallel(&records, 2).unwrap();

        if let OperationOutput::Statistics(value) = result {
            let at_result: ATContentResult = serde_json::from_value(value).unwrap();

            assert_eq!(at_result.at_count, 18);
            assert_eq!(at_result.total_bases, 28);
        } else {
            panic!("Expected Statistics output");
        }
    }
}
