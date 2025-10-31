// Quality Filter Operation
//
// Filters sequences based on mean quality score threshold.
// This is a FILTERING operation (branch-heavy, data-dependent).
//
// Expected patterns:
// - NEON: Moderate benefit (5-15×) - branches limit SIMD
// - Parallel: Good scaling (data-independent filtering)
// - Different from counting (filtering category)
//
// Goal: N=8 validation, test filtering vs counting patterns
//
// Complexity Score: ~0.55 (medium-high)
// - Operations per byte: 0.6 (quality conversion + comparison + accumulation)
// - Accumulator count: 0.4 (2 accumulators: sum, count)
// - Horizontal reduction: 0.5 (sum reduction + mean calculation)
// - Scalar fallback: 0.6 (branch on mean quality)
// - Memory access: 0.4 (sequential read + conditional write)
// - Data dependencies: 0.5 (filter decision depends on aggregation)

use crate::PrimitiveOperation;
use asbb_core::{HardwareConfig, OperationCategory, OperationOutput, SequenceRecord};
use anyhow::Result;
use rayon::prelude::*;
use serde::{Deserialize, Serialize};

pub struct QualityFilter {
    pub min_mean_quality: u8,
}

impl QualityFilter {
    pub fn new(min_mean_quality: u8) -> Self {
        Self { min_mean_quality }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct QualityFilterResult {
    pub total_sequences: usize,
    pub passed_sequences: usize,
    pub filtered_sequences: usize,
    pub pass_rate: f64,
}

impl QualityFilterResult {
    pub fn new() -> Self {
        Self {
            total_sequences: 0,
            passed_sequences: 0,
            filtered_sequences: 0,
            pass_rate: 0.0,
        }
    }

    pub fn add(&mut self, other: &Self) {
        self.total_sequences += other.total_sequences;
        self.passed_sequences += other.passed_sequences;
        self.filtered_sequences += other.filtered_sequences;
    }

    pub fn finalize(&mut self) {
        if self.total_sequences > 0 {
            self.pass_rate = (self.passed_sequences as f64 / self.total_sequences as f64) * 100.0;
        }
    }
}

impl PrimitiveOperation for QualityFilter {
    fn name(&self) -> &str {
        "quality_filter"
    }

    fn category(&self) -> OperationCategory {
        OperationCategory::ElementWise // Filter still processes each sequence independently
    }

    fn execute_naive(&self, data: &[SequenceRecord]) -> Result<OperationOutput> {
        let mut result = QualityFilterResult::new();

        for record in data {
            result.total_sequences += 1;

            if let Some(quality) = &record.quality {
                let mean_quality = calculate_mean_quality_naive(quality);
                if mean_quality >= self.min_mean_quality as f64 {
                    result.passed_sequences += 1;
                } else {
                    result.filtered_sequences += 1;
                }
            }
        }

        result.finalize();
        Ok(OperationOutput::Statistics(serde_json::to_value(result)?))
    }

    fn execute_neon(&self, data: &[SequenceRecord]) -> Result<OperationOutput> {
        #[cfg(target_arch = "aarch64")]
        {
            let mut result = QualityFilterResult::new();

            for record in data {
                result.total_sequences += 1;

                if let Some(quality) = &record.quality {
                    let mean_quality = calculate_mean_quality_neon(quality);
                    if mean_quality >= self.min_mean_quality as f64 {
                        result.passed_sequences += 1;
                    } else {
                        result.filtered_sequences += 1;
                    }
                }
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

        let threshold = self.min_mean_quality;

        let mut result = pool.install(|| {
            data.par_iter()
                .map(|record| {
                    let mut local = QualityFilterResult::new();
                    local.total_sequences = 1;

                    if let Some(quality) = &record.quality {
                        #[cfg(target_arch = "aarch64")]
                        let mean_quality = calculate_mean_quality_neon(quality);

                        #[cfg(not(target_arch = "aarch64"))]
                        let mean_quality = calculate_mean_quality_naive(quality);

                        if mean_quality >= threshold as f64 {
                            local.passed_sequences = 1;
                        } else {
                            local.filtered_sequences = 1;
                        }
                    }

                    local
                })
                .reduce(
                    || QualityFilterResult::new(),
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

// Naive mean quality calculation
fn calculate_mean_quality_naive(quality: &[u8]) -> f64 {
    if quality.is_empty() {
        return 0.0;
    }

    let sum: u64 = quality.iter().map(|&q| q as u64).sum();
    sum as f64 / quality.len() as f64
}

// NEON SIMD mean quality calculation
#[cfg(target_arch = "aarch64")]
fn calculate_mean_quality_neon(quality: &[u8]) -> f64 {
    use std::arch::aarch64::*;

    if quality.is_empty() {
        return 0.0;
    }

    unsafe {
        // Accumulate using u16 to prevent overflow
        let mut vec_sum_u16_1 = vdupq_n_u16(0);
        let mut vec_sum_u16_2 = vdupq_n_u16(0);

        let chunks = quality.chunks_exact(16);
        let remainder = chunks.remainder();

        // Process 16 bytes at a time
        for chunk in chunks {
            let data = vld1q_u8(chunk.as_ptr());

            // Widen to u16 (split into two u16x8 vectors)
            let data_u16_low = vmovl_u8(vget_low_u8(data));
            let data_u16_high = vmovl_u8(vget_high_u8(data));

            // Accumulate
            vec_sum_u16_1 = vaddq_u16(vec_sum_u16_1, data_u16_low);
            vec_sum_u16_2 = vaddq_u16(vec_sum_u16_2, data_u16_high);
        }

        // Horizontal sum
        let mut sum = horizontal_sum_u16(vec_sum_u16_1) + horizontal_sum_u16(vec_sum_u16_2);

        // Process remainder
        for &q in remainder {
            sum += q as u64;
        }

        sum as f64 / quality.len() as f64
    }
}

#[cfg(target_arch = "aarch64")]
fn horizontal_sum_u16(vec: std::arch::aarch64::uint16x8_t) -> u64 {
    unsafe {
        let values: [u16; 8] = std::mem::transmute(vec);
        values.iter().map(|&v| v as u64).sum()
    }
}

#[cfg(not(target_arch = "aarch64"))]
fn calculate_mean_quality_neon(_quality: &[u8]) -> f64 {
    panic!("NEON not available on this platform");
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_records() -> Vec<SequenceRecord> {
        vec![
            SequenceRecord::fastq(
                "high_qual".to_string(),
                b"ACGT".to_vec(),
                vec![40, 40, 40, 40], // Mean: 40
            ),
            SequenceRecord::fastq(
                "med_qual".to_string(),
                b"AAAA".to_vec(),
                vec![25, 25, 25, 25], // Mean: 25
            ),
            SequenceRecord::fastq(
                "low_qual".to_string(),
                b"TTTT".to_vec(),
                vec![10, 10, 10, 10], // Mean: 10
            ),
            SequenceRecord::fastq(
                "mixed_qual".to_string(),
                b"GGGG".to_vec(),
                vec![30, 20, 10, 40], // Mean: 25
            ),
        ]
    }

    #[test]
    fn test_quality_filter_naive() {
        let records = create_test_records();
        let op = QualityFilter::new(20); // Threshold: 20

        let result = op.execute_naive(&records).unwrap();

        if let OperationOutput::Statistics(value) = result {
            let filter_result: QualityFilterResult = serde_json::from_value(value).unwrap();

            // Expected: high_qual (40), med_qual (25), mixed_qual (25) pass (3/4)
            //           low_qual (10) filtered (1/4)
            assert_eq!(filter_result.total_sequences, 4);
            assert_eq!(filter_result.passed_sequences, 3);
            assert_eq!(filter_result.filtered_sequences, 1);
            assert!((filter_result.pass_rate - 75.0).abs() < 0.1);
        } else {
            panic!("Expected Statistics output");
        }
    }

    #[test]
    #[cfg(target_arch = "aarch64")]
    fn test_quality_filter_neon() {
        let records = create_test_records();
        let op = QualityFilter::new(20);

        let result = op.execute_neon(&records).unwrap();

        if let OperationOutput::Statistics(value) = result {
            let filter_result: QualityFilterResult = serde_json::from_value(value).unwrap();

            assert_eq!(filter_result.total_sequences, 4);
            assert_eq!(filter_result.passed_sequences, 3);
            assert_eq!(filter_result.filtered_sequences, 1);
        } else {
            panic!("Expected Statistics output");
        }
    }

    #[test]
    fn test_quality_filter_parallel() {
        let records = create_test_records();
        let op = QualityFilter::new(20);

        let result = op.execute_parallel(&records, 2).unwrap();

        if let OperationOutput::Statistics(value) = result {
            let filter_result: QualityFilterResult = serde_json::from_value(value).unwrap();

            assert_eq!(filter_result.total_sequences, 4);
            assert_eq!(filter_result.passed_sequences, 3);
            assert_eq!(filter_result.filtered_sequences, 1);
        } else {
            panic!("Expected Statistics output");
        }
    }
}
