//! Base counting operation
//!
//! Counts occurrences of each DNA base (A, C, G, T, N) in sequences.
//!
//! **Operation Category**: Element-wise
//! - Highly vectorizable (no dependencies between bases)
//! - Memory-bound (limited by read bandwidth)
//! - Embarrassingly parallel (no shared state)
//!
//! **Expected Results** (from BioMetal experience):
//! - NEON: ~85× speedup over naive
//! - Parallel (4 cores): ~1.5-1.6× on top of NEON
//! - GPU: Not beneficial (overhead > computation time)
//!
//! # Apple Silicon Considerations
//!
//! - **NEON**: 128-bit SIMD, process 16 bases per instruction
//! - **Unified Memory**: No CPU→GPU copy needed, but overhead still dominates
//! - **Memory Bandwidth**: M5 has 153 GB/s, but this operation is simple enough
//!   that bandwidth is rarely the bottleneck (more likely cache-bound)

use anyhow::Result;
use asbb_core::{OperationCategory, OperationOutput, PrimitiveOperation, SequenceRecord};
use rayon::prelude::*;
use serde::{Deserialize, Serialize};

/// Base counting operation
pub struct BaseCounting;

impl BaseCounting {
    pub fn new() -> Self {
        Self
    }
}

/// Result of base counting
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BaseCounts {
    pub count_a: usize,
    pub count_c: usize,
    pub count_g: usize,
    pub count_t: usize,
    pub count_n: usize,
    pub total: usize,
}

impl BaseCounts {
    fn new() -> Self {
        Self {
            count_a: 0,
            count_c: 0,
            count_g: 0,
            count_t: 0,
            count_n: 0,
            total: 0,
        }
    }

    fn add(&mut self, other: &BaseCounts) {
        self.count_a += other.count_a;
        self.count_c += other.count_c;
        self.count_g += other.count_g;
        self.count_t += other.count_t;
        self.count_n += other.count_n;
        self.total += other.total;
    }
}

impl PrimitiveOperation for BaseCounting {
    fn name(&self) -> &str {
        "base_counting"
    }

    fn category(&self) -> OperationCategory {
        OperationCategory::ElementWise
    }

    fn execute_naive(&self, data: &[SequenceRecord]) -> Result<OperationOutput> {
        let mut counts = BaseCounts::new();

        for record in data {
            counts.total += record.sequence.len();

            // Naive scalar loop
            for &base in &record.sequence {
                match base {
                    b'A' | b'a' => counts.count_a += 1,
                    b'C' | b'c' => counts.count_c += 1,
                    b'G' | b'g' => counts.count_g += 1,
                    b'T' | b't' => counts.count_t += 1,
                    b'N' | b'n' => counts.count_n += 1,
                    _ => {} // Ignore invalid bases
                }
            }
        }

        Ok(OperationOutput::Statistics(
            serde_json::to_value(counts)?,
        ))
    }

    fn execute_neon(&self, data: &[SequenceRecord]) -> Result<OperationOutput> {
        // NEON SIMD implementation
        #[cfg(target_arch = "aarch64")]
        {
            let mut counts = BaseCounts::new();

            for record in data {
                let seq = &record.sequence;
                counts.total += seq.len();

                // Count using NEON intrinsics
                let base_counts = count_bases_neon(seq);
                counts.count_a += base_counts.count_a;
                counts.count_c += base_counts.count_c;
                counts.count_g += base_counts.count_g;
                counts.count_t += base_counts.count_t;
                counts.count_n += base_counts.count_n;
            }

            Ok(OperationOutput::Statistics(
                serde_json::to_value(counts)?,
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

        let counts = pool.install(|| {
            data.par_iter()
                .map(|record| {
                    #[cfg(target_arch = "aarch64")]
                    {
                        // Use NEON per-thread for true combined optimization
                        count_bases_neon(&record.sequence)
                    }

                    #[cfg(not(target_arch = "aarch64"))]
                    {
                        // Fall back to naive on non-ARM
                        let mut counts = BaseCounts::new();
                        counts.total = record.sequence.len();

                        for &base in &record.sequence {
                            match base {
                                b'A' | b'a' => counts.count_a += 1,
                                b'C' | b'c' => counts.count_c += 1,
                                b'G' | b'g' => counts.count_g += 1,
                                b'T' | b't' => counts.count_t += 1,
                                b'N' | b'n' => counts.count_n += 1,
                                _ => {}
                            }
                        }

                        counts
                    }
                })
                .reduce(
                    || BaseCounts::new(),
                    |mut a, b| {
                        a.add(&b);
                        a
                    },
                )
        });

        Ok(OperationOutput::Statistics(
            serde_json::to_value(counts)?,
        ))
    }
}

impl Default for BaseCounting {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// NEON SIMD Implementation
// ============================================================================

#[cfg(target_arch = "aarch64")]
fn count_bases_neon(seq: &[u8]) -> BaseCounts {
    use std::arch::aarch64::*;

    let mut counts = BaseCounts::new();
    counts.total = seq.len();

    // Process 16 bytes at a time with NEON
    let chunks = seq.chunks_exact(16);
    let remainder = chunks.remainder();

    unsafe {
        // Accumulators for vectorized counts
        let mut vec_count_a = vdupq_n_u8(0);
        let mut vec_count_c = vdupq_n_u8(0);
        let mut vec_count_g = vdupq_n_u8(0);
        let mut vec_count_t = vdupq_n_u8(0);
        let mut vec_count_n = vdupq_n_u8(0);

        // Comparison values
        let cmp_a_upper = vdupq_n_u8(b'A');
        let cmp_a_lower = vdupq_n_u8(b'a');
        let cmp_c_upper = vdupq_n_u8(b'C');
        let cmp_c_lower = vdupq_n_u8(b'c');
        let cmp_g_upper = vdupq_n_u8(b'G');
        let cmp_g_lower = vdupq_n_u8(b'g');
        let cmp_t_upper = vdupq_n_u8(b'T');
        let cmp_t_lower = vdupq_n_u8(b't');
        let cmp_n_upper = vdupq_n_u8(b'N');
        let cmp_n_lower = vdupq_n_u8(b'n');

        let ones = vdupq_n_u8(1);

        for chunk in chunks {
            // Load 16 bytes
            let data = vld1q_u8(chunk.as_ptr());

            // Compare with each base (both upper and lower case)
            let mask_a = vorrq_u8(
                vceqq_u8(data, cmp_a_upper),
                vceqq_u8(data, cmp_a_lower),
            );
            let mask_c = vorrq_u8(
                vceqq_u8(data, cmp_c_upper),
                vceqq_u8(data, cmp_c_lower),
            );
            let mask_g = vorrq_u8(
                vceqq_u8(data, cmp_g_upper),
                vceqq_u8(data, cmp_g_lower),
            );
            let mask_t = vorrq_u8(
                vceqq_u8(data, cmp_t_upper),
                vceqq_u8(data, cmp_t_lower),
            );
            let mask_n = vorrq_u8(
                vceqq_u8(data, cmp_n_upper),
                vceqq_u8(data, cmp_n_lower),
            );

            // Increment counts where mask is true
            // mask is 0xFF for true, 0x00 for false
            // Bitwise AND with 1 to get count increment
            vec_count_a = vaddq_u8(vec_count_a, vandq_u8(mask_a, ones));
            vec_count_c = vaddq_u8(vec_count_c, vandq_u8(mask_c, ones));
            vec_count_g = vaddq_u8(vec_count_g, vandq_u8(mask_g, ones));
            vec_count_t = vaddq_u8(vec_count_t, vandq_u8(mask_t, ones));
            vec_count_n = vaddq_u8(vec_count_n, vandq_u8(mask_n, ones));
        }

        // Horizontal sum to get total counts
        counts.count_a = horizontal_sum_u8(vec_count_a);
        counts.count_c = horizontal_sum_u8(vec_count_c);
        counts.count_g = horizontal_sum_u8(vec_count_g);
        counts.count_t = horizontal_sum_u8(vec_count_t);
        counts.count_n = horizontal_sum_u8(vec_count_n);
    }

    // Process remainder with scalar code
    for &base in remainder {
        match base {
            b'A' | b'a' => counts.count_a += 1,
            b'C' | b'c' => counts.count_c += 1,
            b'G' | b'g' => counts.count_g += 1,
            b'T' | b't' => counts.count_t += 1,
            b'N' | b'n' => counts.count_n += 1,
            _ => {}
        }
    }

    counts
}

/// Horizontal sum of uint8x16_t vector
#[cfg(target_arch = "aarch64")]
unsafe fn horizontal_sum_u8(v: std::arch::aarch64::uint8x16_t) -> usize {
    use std::arch::aarch64::*;

    // Pairwise add to reduce vector
    // uint8x16_t → uint16x8_t → uint32x4_t → uint64x2_t → u64
    let sum16 = vpaddlq_u8(v); // 16x u8 → 8x u16
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
            SequenceRecord::fasta("seq2".to_string(), b"AAAACCCCGGGGTTTT".to_vec()),
            SequenceRecord::fasta("seq3".to_string(), b"NNNACGT".to_vec()),
        ]
    }

    #[test]
    fn test_base_counting_naive() {
        let op = BaseCounting::new();
        let data = create_test_data();

        let result = op.execute_naive(&data).unwrap();

        if let OperationOutput::Statistics(value) = result {
            let counts: BaseCounts = serde_json::from_value(value).unwrap();

            // seq1: 2A, 2C, 2G, 2T
            // seq2: 4A, 4C, 4G, 4T
            // seq3: 3N, 1A, 1C, 1G, 1T
            assert_eq!(counts.count_a, 7);
            assert_eq!(counts.count_c, 7);
            assert_eq!(counts.count_g, 7);
            assert_eq!(counts.count_t, 7);
            assert_eq!(counts.count_n, 3);
            assert_eq!(counts.total, 31);
        } else {
            panic!("Expected Statistics output");
        }
    }

    #[test]
    #[cfg(target_arch = "aarch64")]
    fn test_base_counting_neon() {
        let op = BaseCounting::new();
        let data = create_test_data();

        let result_naive = op.execute_naive(&data).unwrap();
        let result_neon = op.execute_neon(&data).unwrap();

        // NEON should produce identical results to naive
        assert_eq!(result_naive, result_neon);
    }

    #[test]
    fn test_base_counting_parallel() {
        let op = BaseCounting::new();
        let data = create_test_data();

        let result_naive = op.execute_naive(&data).unwrap();
        let result_parallel = op.execute_parallel(&data, 4).unwrap();

        // Parallel should produce identical results to naive
        assert_eq!(result_naive, result_parallel);
    }

    #[test]
    fn test_base_counting_case_insensitive() {
        let op = BaseCounting::new();
        let data = vec![SequenceRecord::fasta(
            "test".to_string(),
            b"AaCcGgTtNn".to_vec(),
        )];

        let result = op.execute_naive(&data).unwrap();

        if let OperationOutput::Statistics(value) = result {
            let counts: BaseCounts = serde_json::from_value(value).unwrap();
            assert_eq!(counts.count_a, 2);
            assert_eq!(counts.count_c, 2);
            assert_eq!(counts.count_g, 2);
            assert_eq!(counts.count_t, 2);
            assert_eq!(counts.count_n, 2);
        } else {
            panic!("Expected Statistics output");
        }
    }
}
