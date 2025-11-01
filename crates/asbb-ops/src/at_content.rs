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
use asbb_core::{encoding::BitSeq, HardwareConfig, OperationCategory, OperationOutput, SequenceRecord};
use anyhow::Result;
use rayon::prelude::*;
use serde::{Deserialize, Serialize};

pub struct ATContent;

impl ATContent {
    /// Execute AT content on 2-bit encoded sequences (naive)
    pub fn execute_2bit_naive(&self, data: &[BitSeq]) -> Result<OperationOutput> {
        let mut result = ATContentResult::new();

        for bitseq in data {
            result.total_bases += bitseq.len();
            result.at_count += bitseq.count_at(); // Use BitSeq's helper
        }

        result.finalize();
        Ok(OperationOutput::Statistics(serde_json::to_value(result)?))
    }

    /// Execute AT content on 2-bit encoded sequences (NEON)
    ///
    /// Expected: 1.2-1.5× speedup over ASCII NEON due to:
    /// - 4× data density (better cache utilization)
    /// - Same NEON operations, but on more compact data
    pub fn execute_2bit_neon(&self, data: &[BitSeq]) -> Result<OperationOutput> {
        #[cfg(target_arch = "aarch64")]
        {
            let mut result = ATContentResult::new();

            for bitseq in data {
                let local_result = count_at_2bit_neon(bitseq);
                result.add(&local_result);
            }

            result.finalize();
            Ok(OperationOutput::Statistics(serde_json::to_value(result)?))
        }

        #[cfg(not(target_arch = "aarch64"))]
        {
            // Fall back to naive on non-ARM
            self.execute_2bit_naive(data)
        }
    }
}

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

/// Count AT bases in 2-bit encoded data using NEON
///
/// 2-bit encoding: A=00, C=01, G=10, T=11
/// AT bases: A=00, T=11
///
/// Expected speedup: 1.2-1.5× over ASCII NEON
/// - Same NEON operations, but 4× data density
/// - Better cache utilization
#[cfg(target_arch = "aarch64")]
fn count_at_2bit_neon(bitseq: &BitSeq) -> ATContentResult {
    use std::arch::aarch64::*;

    let mut result = ATContentResult::new();
    result.total_bases = bitseq.len();

    let data = bitseq.data();

    // Process 16 bytes at a time (16 bytes = 64 bases)
    let chunks = data.chunks_exact(16);
    let remainder_bytes = chunks.remainder();

    unsafe {
        // Accumulator for AT count
        let mut vec_count_at = vdupq_n_u8(0);

        // Masks for extracting 2-bit pairs
        let mask_bits_67 = vdupq_n_u8(0b11000000);
        let mask_bits_45 = vdupq_n_u8(0b00110000);
        let mask_bits_23 = vdupq_n_u8(0b00001100);
        let mask_bits_01 = vdupq_n_u8(0b00000011);

        // Comparison values
        let cmp_a = vdupq_n_u8(0b00); // A = 00
        let cmp_t = vdupq_n_u8(0b11); // T = 11

        let ones = vdupq_n_u8(1);

        for chunk in chunks {
            // Load 16 bytes (64 bases)
            let data_vec = vld1q_u8(chunk.as_ptr());

            // Process each 2-bit position
            // Position 0 (bits 6-7)
            let bases_0 = vshrq_n_u8(vandq_u8(data_vec, mask_bits_67), 6);
            let mask_at_0 = vorrq_u8(vceqq_u8(bases_0, cmp_a), vceqq_u8(bases_0, cmp_t));
            vec_count_at = vaddq_u8(vec_count_at, vandq_u8(mask_at_0, ones));

            // Position 1 (bits 4-5)
            let bases_1 = vshrq_n_u8(vandq_u8(data_vec, mask_bits_45), 4);
            let mask_at_1 = vorrq_u8(vceqq_u8(bases_1, cmp_a), vceqq_u8(bases_1, cmp_t));
            vec_count_at = vaddq_u8(vec_count_at, vandq_u8(mask_at_1, ones));

            // Position 2 (bits 2-3)
            let bases_2 = vshrq_n_u8(vandq_u8(data_vec, mask_bits_23), 2);
            let mask_at_2 = vorrq_u8(vceqq_u8(bases_2, cmp_a), vceqq_u8(bases_2, cmp_t));
            vec_count_at = vaddq_u8(vec_count_at, vandq_u8(mask_at_2, ones));

            // Position 3 (bits 0-1)
            let bases_3 = vandq_u8(data_vec, mask_bits_01);
            let mask_at_3 = vorrq_u8(vceqq_u8(bases_3, cmp_a), vceqq_u8(bases_3, cmp_t));
            vec_count_at = vaddq_u8(vec_count_at, vandq_u8(mask_at_3, ones));
        }

        // Horizontal sum to get total AT count
        result.at_count = horizontal_sum_u8(vec_count_at);
    }

    // Process remainder bytes with scalar code
    let num_full_bytes = data.len() - remainder_bytes.len();
    let bases_in_full_bytes = num_full_bytes * 4;

    for i in bases_in_full_bytes..bitseq.len() {
        let byte_idx = i / 4;
        let bit_offset = 6 - (i % 4) * 2;
        let encoded = (data[byte_idx] >> bit_offset) & 0b11;

        match encoded {
            0b00 | 0b11 => result.at_count += 1, // A or T
            _ => {}                               // C or G
        }
    }

    result
}

#[cfg(not(target_arch = "aarch64"))]
fn count_at_2bit_neon(_bitseq: &BitSeq) -> ATContentResult {
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

    #[test]
    fn test_at_content_2bit_naive() {
        let op = ATContent;

        // Create 2-bit encoded sequences
        let bitseqs = vec![
            BitSeq::from_ascii(b"ACGT"),        // 2 AT, 2 GC
            BitSeq::from_ascii(b"AAAATTTT"),    // 8 AT, 0 GC
            BitSeq::from_ascii(b"GGGGCCCC"),    // 0 AT, 8 GC
        ];

        let result = op.execute_2bit_naive(&bitseqs).unwrap();

        if let OperationOutput::Statistics(value) = result {
            let at_result: ATContentResult = serde_json::from_value(value).unwrap();

            // Expected: 2 + 8 + 0 = 10 AT bases out of 20 total
            assert_eq!(at_result.at_count, 10);
            assert_eq!(at_result.total_bases, 20);
            assert!((at_result.at_percent - 50.0).abs() < 0.01); // 10/20 * 100 = 50%
        } else {
            panic!("Expected Statistics output");
        }
    }

    #[test]
    #[cfg(target_arch = "aarch64")]
    fn test_at_content_2bit_neon() {
        let op = ATContent;

        // Create 2-bit encoded sequences
        let bitseqs = vec![
            BitSeq::from_ascii(b"ACGT"),
            BitSeq::from_ascii(b"AAAATTTT"),
            BitSeq::from_ascii(b"GGGGCCCC"),
        ];

        let result_naive = op.execute_2bit_naive(&bitseqs).unwrap();
        let result_neon = op.execute_2bit_neon(&bitseqs).unwrap();

        // NEON should produce identical results to naive
        assert_eq!(result_naive, result_neon);
    }

    #[test]
    #[cfg(target_arch = "aarch64")]
    fn test_at_content_2bit_neon_large() {
        let op = ATContent;

        // Create a large sequence to test vectorization (>64 bases)
        let large_seq = b"ACGTACGTACGTACGTACGTACGTACGTACGTACGTACGTACGTACGTACGTACGTACGTACGTACGTACGTACGTACGTACGTACGTACGTACGTACGTACGTACGTACGTACGTACGTACGTACGTACGTACGTACGTACGTACGTACGTACGTACGTACGTACGTACGTACGT";
        let bitseqs = vec![BitSeq::from_ascii(large_seq)];

        let result_naive = op.execute_2bit_naive(&bitseqs).unwrap();
        let result_neon = op.execute_2bit_neon(&bitseqs).unwrap();

        // NEON should produce identical results to naive
        assert_eq!(result_naive, result_neon);

        // Verify AT content is 50% (ACGTACGT pattern)
        if let OperationOutput::Statistics(value) = result_neon {
            let at_result: ATContentResult = serde_json::from_value(value).unwrap();
            assert!((at_result.at_percent - 50.0).abs() < 0.01);
        }
    }
}
