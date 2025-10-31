//! GC content calculation
//!
//! Calculates GC content (percentage of G+C bases) in sequences.
//!
//! **Operation Category**: Element-wise
//! - Highly vectorizable (no dependencies between bases)
//! - Memory-bound (limited by read bandwidth)
//! - Embarrassingly parallel (no shared state)
//!
//! **Expected Results** (based on base counting patterns):
//! - NEON: 16-65× speedup (scale-dependent)
//! - Parallel (4 cores): 1.5-3.5× on top of NEON
//! - Threshold: 1,000 sequences for parallel benefit
//!
//! # Apple Silicon Considerations
//!
//! Very similar to base counting - expect similar patterns.

use anyhow::Result;
use asbb_core::{OperationCategory, OperationOutput, PrimitiveOperation, SequenceRecord};
use rayon::prelude::*;
use serde::{Deserialize, Serialize};

/// GC content calculation operation
pub struct GcContent;

impl GcContent {
    pub fn new() -> Self {
        Self
    }
}

/// Result of GC content calculation
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GcResult {
    pub count_g: usize,
    pub count_c: usize,
    pub count_gc: usize,  // G + C
    pub count_at: usize,  // A + T
    pub count_n: usize,
    pub total_bases: usize,
    pub gc_percent: f64,  // (G+C) / total * 100
}

impl GcResult {
    fn new() -> Self {
        Self {
            count_g: 0,
            count_c: 0,
            count_gc: 0,
            count_at: 0,
            count_n: 0,
            total_bases: 0,
            gc_percent: 0.0,
        }
    }

    fn finalize(&mut self) {
        self.count_gc = self.count_g + self.count_c;
        let valid_bases = self.total_bases - self.count_n;
        if valid_bases > 0 {
            self.gc_percent = (self.count_gc as f64 / valid_bases as f64) * 100.0;
        }
    }

    fn add(&mut self, other: &GcResult) {
        self.count_g += other.count_g;
        self.count_c += other.count_c;
        self.count_gc += other.count_gc;
        self.count_at += other.count_at;
        self.count_n += other.count_n;
        self.total_bases += other.total_bases;
    }
}

impl PrimitiveOperation for GcContent {
    fn name(&self) -> &str {
        "gc_content"
    }

    fn category(&self) -> OperationCategory {
        OperationCategory::ElementWise
    }

    fn execute_naive(&self, data: &[SequenceRecord]) -> Result<OperationOutput> {
        let mut result = GcResult::new();

        for record in data {
            result.total_bases += record.sequence.len();

            // Naive scalar loop
            for &base in &record.sequence {
                match base {
                    b'G' | b'g' => result.count_g += 1,
                    b'C' | b'c' => result.count_c += 1,
                    b'A' | b'a' | b'T' | b't' => result.count_at += 1,
                    b'N' | b'n' => result.count_n += 1,
                    _ => {} // Ignore invalid bases
                }
            }
        }

        result.finalize();
        Ok(OperationOutput::Statistics(
            serde_json::to_value(result)?,
        ))
    }

    fn execute_neon(&self, data: &[SequenceRecord]) -> Result<OperationOutput> {
        // NEON SIMD implementation
        #[cfg(target_arch = "aarch64")]
        {
            let mut result = GcResult::new();

            for record in data {
                let seq = &record.sequence;
                result.total_bases += seq.len();

                // Count using NEON intrinsics
                let gc_result = count_gc_neon(seq);
                result.count_g += gc_result.count_g;
                result.count_c += gc_result.count_c;
                result.count_at += gc_result.count_at;
                result.count_n += gc_result.count_n;
            }

            result.finalize();
            Ok(OperationOutput::Statistics(
                serde_json::to_value(result)?,
            ))
        }

        #[cfg(not(target_arch = "aarch64"))]
        {
            // Fall back to naive on non-ARM
            self.execute_naive(data)
        }
    }

    fn execute_parallel(
        &self,
        data: &[SequenceRecord],
        num_threads: usize,
    ) -> Result<OperationOutput> {
        // Configure Rayon thread pool
        let pool = rayon::ThreadPoolBuilder::new()
            .num_threads(num_threads)
            .build()?;

        let mut result = pool.install(|| {
            data.par_iter()
                .map(|record| {
                    #[cfg(target_arch = "aarch64")]
                    {
                        // Use NEON per-thread for true combined optimization
                        count_gc_neon(&record.sequence)
                    }

                    #[cfg(not(target_arch = "aarch64"))]
                    {
                        // Fall back to naive on non-ARM
                        let mut result = GcResult::new();
                        result.total_bases = record.sequence.len();

                        for &base in &record.sequence {
                            match base {
                                b'G' | b'g' => result.count_g += 1,
                                b'C' | b'c' => result.count_c += 1,
                                b'A' | b'a' | b'T' | b't' => result.count_at += 1,
                                b'N' | b'n' => result.count_n += 1,
                                _ => {}
                            }
                        }

                        result
                    }
                })
                .reduce(
                    || GcResult::new(),
                    |mut a, b| {
                        a.add(&b);
                        a
                    },
                )
        });

        result.finalize();
        Ok(OperationOutput::Statistics(
            serde_json::to_value(result)?,
        ))
    }
}

impl Default for GcContent {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// NEON SIMD Implementation
// ============================================================================

#[cfg(target_arch = "aarch64")]
fn count_gc_neon(seq: &[u8]) -> GcResult {
    use std::arch::aarch64::*;

    let mut result = GcResult::new();
    result.total_bases = seq.len();

    // Process 16 bytes at a time with NEON
    let chunks = seq.chunks_exact(16);
    let remainder = chunks.remainder();

    unsafe {
        // Accumulators for vectorized counts
        let mut vec_count_g = vdupq_n_u8(0);
        let mut vec_count_c = vdupq_n_u8(0);
        let mut vec_count_at = vdupq_n_u8(0);
        let mut vec_count_n = vdupq_n_u8(0);

        // Comparison values
        let cmp_g_upper = vdupq_n_u8(b'G');
        let cmp_g_lower = vdupq_n_u8(b'g');
        let cmp_c_upper = vdupq_n_u8(b'C');
        let cmp_c_lower = vdupq_n_u8(b'c');
        let cmp_a_upper = vdupq_n_u8(b'A');
        let cmp_a_lower = vdupq_n_u8(b'a');
        let cmp_t_upper = vdupq_n_u8(b'T');
        let cmp_t_lower = vdupq_n_u8(b't');
        let cmp_n_upper = vdupq_n_u8(b'N');
        let cmp_n_lower = vdupq_n_u8(b'n');

        let ones = vdupq_n_u8(1);

        for chunk in chunks {
            // Load 16 bytes
            let data = vld1q_u8(chunk.as_ptr());

            // Compare with each base (both upper and lower case)
            let mask_g = vorrq_u8(
                vceqq_u8(data, cmp_g_upper),
                vceqq_u8(data, cmp_g_lower),
            );
            let mask_c = vorrq_u8(
                vceqq_u8(data, cmp_c_upper),
                vceqq_u8(data, cmp_c_lower),
            );
            let mask_a = vorrq_u8(
                vceqq_u8(data, cmp_a_upper),
                vceqq_u8(data, cmp_a_lower),
            );
            let mask_t = vorrq_u8(
                vceqq_u8(data, cmp_t_upper),
                vceqq_u8(data, cmp_t_lower),
            );
            let mask_at = vorrq_u8(mask_a, mask_t);
            let mask_n = vorrq_u8(
                vceqq_u8(data, cmp_n_upper),
                vceqq_u8(data, cmp_n_lower),
            );

            // Increment counts where mask is true
            vec_count_g = vaddq_u8(vec_count_g, vandq_u8(mask_g, ones));
            vec_count_c = vaddq_u8(vec_count_c, vandq_u8(mask_c, ones));
            vec_count_at = vaddq_u8(vec_count_at, vandq_u8(mask_at, ones));
            vec_count_n = vaddq_u8(vec_count_n, vandq_u8(mask_n, ones));
        }

        // Horizontal sum to get total counts
        result.count_g = horizontal_sum_u8(vec_count_g);
        result.count_c = horizontal_sum_u8(vec_count_c);
        result.count_at = horizontal_sum_u8(vec_count_at);
        result.count_n = horizontal_sum_u8(vec_count_n);
    }

    // Process remainder with scalar code
    for &base in remainder {
        match base {
            b'G' | b'g' => result.count_g += 1,
            b'C' | b'c' => result.count_c += 1,
            b'A' | b'a' | b'T' | b't' => result.count_at += 1,
            b'N' | b'n' => result.count_n += 1,
            _ => {}
        }
    }

    result
}

/// Horizontal sum of uint8x16_t vector
#[cfg(target_arch = "aarch64")]
unsafe fn horizontal_sum_u8(v: std::arch::aarch64::uint8x16_t) -> usize {
    use std::arch::aarch64::*;

    // Pairwise add to reduce vector
    let sum16 = vpaddlq_u8(v);     // 16x u8 → 8x u16
    let sum32 = vpaddlq_u16(sum16); // 8x u16 → 4x u32
    let sum64 = vpaddlq_u32(sum32); // 4x u32 → 2x u64

    // Extract and sum the two u64 values
    let sum = vgetq_lane_u64(sum64, 0) + vgetq_lane_u64(sum64, 1);
    sum as usize
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_data() -> Vec<SequenceRecord> {
        vec![
            SequenceRecord::fasta("seq1".to_string(), b"ACGTACGT".to_vec()),
            SequenceRecord::fasta("seq2".to_string(), b"GGGGCCCC".to_vec()),
            SequenceRecord::fasta("seq3".to_string(), b"AAAATTTT".to_vec()),
            SequenceRecord::fasta("seq4".to_string(), b"NNNACGT".to_vec()),
        ]
    }

    #[test]
    fn test_gc_content_naive() {
        let op = GcContent::new();
        let data = create_test_data();

        let result = op.execute_naive(&data).unwrap();

        if let OperationOutput::Statistics(value) = result {
            let gc: GcResult = serde_json::from_value(value).unwrap();

            // seq1: 2A, 2C, 2G, 2T = 4 AT, 4 GC
            // seq2: 4G, 4C = 0 AT, 8 GC
            // seq3: 4A, 4T = 8 AT, 0 GC
            // seq4: 1A, 1C, 1G, 1T, 3N = 2 AT, 2 GC, 3N
            assert_eq!(gc.count_g, 7);
            assert_eq!(gc.count_c, 7);
            assert_eq!(gc.count_gc, 14);
            assert_eq!(gc.count_at, 14);  // 4 + 0 + 8 + 2 = 14
            assert_eq!(gc.count_n, 3);
            assert_eq!(gc.total_bases, 31);

            // (14 GC / 28 valid bases) * 100 = 50%
            assert!((gc.gc_percent - 50.0).abs() < 0.01);
        } else {
            panic!("Expected Statistics output");
        }
    }

    #[test]
    #[cfg(target_arch = "aarch64")]
    fn test_gc_content_neon() {
        let op = GcContent::new();
        let data = create_test_data();

        let result_naive = op.execute_naive(&data).unwrap();
        let result_neon = op.execute_neon(&data).unwrap();

        // NEON should produce identical results to naive
        assert_eq!(result_naive, result_neon);
    }

    #[test]
    fn test_gc_content_parallel() {
        let op = GcContent::new();
        let data = create_test_data();

        let result_naive = op.execute_naive(&data).unwrap();
        let result_parallel = op.execute_parallel(&data, 4).unwrap();

        // Parallel should produce identical results to naive
        assert_eq!(result_naive, result_parallel);
    }

    #[test]
    fn test_gc_content_all_gc() {
        let op = GcContent::new();
        let data = vec![SequenceRecord::fasta(
            "test".to_string(),
            b"GGGGCCCC".to_vec(),
        )];

        let result = op.execute_naive(&data).unwrap();

        if let OperationOutput::Statistics(value) = result {
            let gc: GcResult = serde_json::from_value(value).unwrap();
            assert_eq!(gc.count_g, 4);
            assert_eq!(gc.count_c, 4);
            assert_eq!(gc.count_gc, 8);
            assert!((gc.gc_percent - 100.0).abs() < 0.01);
        } else {
            panic!("Expected Statistics output");
        }
    }

    #[test]
    fn test_gc_content_no_gc() {
        let op = GcContent::new();
        let data = vec![SequenceRecord::fasta(
            "test".to_string(),
            b"AAAATTTT".to_vec(),
        )];

        let result = op.execute_naive(&data).unwrap();

        if let OperationOutput::Statistics(value) = result {
            let gc: GcResult = serde_json::from_value(value).unwrap();
            assert_eq!(gc.count_g, 0);
            assert_eq!(gc.count_c, 0);
            assert_eq!(gc.count_gc, 0);
            assert!((gc.gc_percent - 0.0).abs() < 0.01);
        } else {
            panic!("Expected Statistics output");
        }
    }
}
