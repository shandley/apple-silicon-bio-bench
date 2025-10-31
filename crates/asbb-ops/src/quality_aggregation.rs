// Quality Score Aggregation Operation
//
// Calculates min/max/mean quality scores across all sequences.
// This is an element-wise counting operation similar to base_counting and gc_content.
//
// Expected patterns (from N=2 validation):
// - NEON: 14-35× speedup (scale-dependent, cache effects)
// - Parallel: Threshold at 1,000 sequences
// - Combined: Uses NEON per-thread (40-60× at large scale)

use crate::PrimitiveOperation;
use asbb_core::{HardwareConfig, OperationCategory, OperationOutput, SequenceRecord};
use anyhow::Result;
use rayon::prelude::*;
use serde::{Deserialize, Serialize};

pub struct QualityAggregation;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct QualityStats {
    pub min_quality: u8,
    pub max_quality: u8,
    pub total_quality: u64,  // Sum of all quality scores
    pub num_bases: u64,
    pub mean_quality: f64,
}

impl QualityStats {
    pub fn new() -> Self {
        Self {
            min_quality: 255,  // Will be replaced by first real value
            max_quality: 0,
            total_quality: 0,
            num_bases: 0,
            mean_quality: 0.0,
        }
    }

    pub fn add(&mut self, other: &Self) {
        if other.num_bases == 0 {
            return;
        }

        if self.num_bases == 0 {
            // First real data
            self.min_quality = other.min_quality;
            self.max_quality = other.max_quality;
        } else {
            self.min_quality = self.min_quality.min(other.min_quality);
            self.max_quality = self.max_quality.max(other.max_quality);
        }

        self.total_quality += other.total_quality;
        self.num_bases += other.num_bases;
    }

    pub fn finalize(&mut self) {
        if self.num_bases > 0 {
            self.mean_quality = self.total_quality as f64 / self.num_bases as f64;
        }
    }
}

impl PrimitiveOperation for QualityAggregation {
    fn name(&self) -> &str {
        "quality_aggregation"
    }

    fn category(&self) -> OperationCategory {
        OperationCategory::ElementWise
    }

    fn execute_naive(&self, data: &[SequenceRecord]) -> Result<OperationOutput> {
        let mut stats = QualityStats::new();

        for record in data {
            if let Some(qual) = &record.quality {
                let local_stats = aggregate_quality_naive(qual);
                stats.add(&local_stats);
            }
        }

        stats.finalize();
        Ok(OperationOutput::Statistics(serde_json::to_value(stats)?))
    }

    fn execute_neon(&self, data: &[SequenceRecord]) -> Result<OperationOutput> {
        #[cfg(target_arch = "aarch64")]
        {
            let mut stats = QualityStats::new();

            for record in data {
                if let Some(qual) = &record.quality {
                    let local_stats = aggregate_quality_neon(qual);
                    stats.add(&local_stats);
                }
            }

            stats.finalize();
            Ok(OperationOutput::Statistics(serde_json::to_value(stats)?))
        }

        #[cfg(not(target_arch = "aarch64"))]
        {
            // Fallback to naive on non-ARM platforms
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

        let mut stats = pool.install(|| {
            data.par_iter()
                .map(|record| {
                    if let Some(qual) = &record.quality {
                        // CRITICAL: Use NEON per-thread on ARM, naive otherwise
                        #[cfg(target_arch = "aarch64")]
                        {
                            aggregate_quality_neon(qual)
                        }

                        #[cfg(not(target_arch = "aarch64"))]
                        {
                            aggregate_quality_naive(qual)
                        }
                    } else {
                        QualityStats::new()
                    }
                })
                .reduce(
                    || QualityStats::new(),
                    |mut a, b| {
                        a.add(&b);
                        a
                    },
                )
        });

        stats.finalize();
        Ok(OperationOutput::Statistics(serde_json::to_value(stats)?))
    }

    fn execute_with_config(
        &self,
        data: &[SequenceRecord],
        config: &HardwareConfig,
    ) -> Result<OperationOutput> {
        // Execution precedence (from combined optimization findings):
        // Parallel takes precedence over NEON (parallel uses NEON per-thread)
        if config.num_threads > 1 {
            self.execute_parallel(data, config.num_threads)
        } else if config.use_neon {
            self.execute_neon(data)
        } else {
            self.execute_naive(data)
        }
    }
}

// Naive implementation: Simple loop with min/max/sum
fn aggregate_quality_naive(qual: &[u8]) -> QualityStats {
    let mut stats = QualityStats::new();

    if qual.is_empty() {
        return stats;
    }

    stats.num_bases = qual.len() as u64;
    stats.min_quality = *qual.iter().min().unwrap();
    stats.max_quality = *qual.iter().max().unwrap();
    stats.total_quality = qual.iter().map(|&q| q as u64).sum();

    stats
}

// NEON SIMD implementation: Vectorized min/max/sum
#[cfg(target_arch = "aarch64")]
fn aggregate_quality_neon(qual: &[u8]) -> QualityStats {
    use std::arch::aarch64::*;

    let mut stats = QualityStats::new();

    if qual.is_empty() {
        return stats;
    }

    stats.num_bases = qual.len() as u64;

    unsafe {
        // Initialize NEON accumulators
        let mut vec_min = vdupq_n_u8(255);  // Start with max value
        let mut vec_max = vdupq_n_u8(0);    // Start with min value
        let mut vec_sum_u16_1 = vdupq_n_u16(0);  // Sum accumulator (16-bit to avoid overflow)
        let mut vec_sum_u16_2 = vdupq_n_u16(0);  // Second accumulator for alternating lanes

        let chunks = qual.chunks_exact(16);
        let remainder = chunks.remainder();

        // Process 16 bytes at a time with NEON
        for chunk in chunks {
            let data = vld1q_u8(chunk.as_ptr());

            // Update min and max
            vec_min = vminq_u8(vec_min, data);
            vec_max = vmaxq_u8(vec_max, data);

            // Sum: Widen to u16 to avoid overflow, then accumulate
            // Split into low and high 8 bytes
            let low = vget_low_u8(data);
            let high = vget_high_u8(data);

            // Widen to u16
            let low_u16 = vmovl_u8(low);
            let high_u16 = vmovl_u8(high);

            // Accumulate
            vec_sum_u16_1 = vaddq_u16(vec_sum_u16_1, low_u16);
            vec_sum_u16_2 = vaddq_u16(vec_sum_u16_2, high_u16);
        }

        // Horizontal reduction for min
        let min_bytes: [u8; 16] = std::mem::transmute(vec_min);
        stats.min_quality = *min_bytes.iter().min().unwrap();

        // Horizontal reduction for max
        let max_bytes: [u8; 16] = std::mem::transmute(vec_max);
        stats.max_quality = *max_bytes.iter().max().unwrap();

        // Horizontal reduction for sum (16-bit accumulators)
        let sum_u16_1: [u16; 8] = std::mem::transmute(vec_sum_u16_1);
        let sum_u16_2: [u16; 8] = std::mem::transmute(vec_sum_u16_2);
        let mut total: u64 = 0;
        for &val in &sum_u16_1 {
            total += val as u64;
        }
        for &val in &sum_u16_2 {
            total += val as u64;
        }

        // Process remainder with scalar code
        for &q in remainder {
            stats.min_quality = stats.min_quality.min(q);
            stats.max_quality = stats.max_quality.max(q);
            total += q as u64;
        }

        stats.total_quality = total;
    }

    stats
}

#[cfg(not(target_arch = "aarch64"))]
fn aggregate_quality_neon(_qual: &[u8]) -> QualityStats {
    panic!("NEON not available on this platform");
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_records() -> Vec<SequenceRecord> {
        vec![
            SequenceRecord::fastq(
                "seq1".to_string(),
                b"ACGT".to_vec(),
                vec![30, 35, 40, 45],  // Quality scores
            ),
            SequenceRecord::fastq(
                "seq2".to_string(),
                b"GGGG".to_vec(),
                vec![20, 25, 30, 35],
            ),
            SequenceRecord::fastq(
                "seq3".to_string(),
                b"TTTT".to_vec(),
                vec![10, 15, 20, 25],
            ),
        ]
    }

    #[test]
    fn test_quality_aggregation_naive() {
        let records = create_test_records();
        let op = QualityAggregation;

        let result = op.execute_naive(&records).unwrap();

        if let OperationOutput::Statistics(value) = result {
            let stats: QualityStats = serde_json::from_value(value).unwrap();

            // Expected: min=10, max=45, total=30+35+40+45+20+25+30+35+10+15+20+25=330
            // mean = 330/12 = 27.5
            assert_eq!(stats.min_quality, 10);
            assert_eq!(stats.max_quality, 45);
            assert_eq!(stats.total_quality, 330);
            assert_eq!(stats.num_bases, 12);
            assert!((stats.mean_quality - 27.5).abs() < 0.01);
        } else {
            panic!("Expected Statistics output");
        }
    }

    #[test]
    #[cfg(target_arch = "aarch64")]
    fn test_quality_aggregation_neon() {
        let records = create_test_records();
        let op = QualityAggregation;

        let result = op.execute_neon(&records).unwrap();

        if let OperationOutput::Statistics(value) = result {
            let stats: QualityStats = serde_json::from_value(value).unwrap();

            assert_eq!(stats.min_quality, 10);
            assert_eq!(stats.max_quality, 45);
            assert_eq!(stats.total_quality, 330);
            assert_eq!(stats.num_bases, 12);
            assert!((stats.mean_quality - 27.5).abs() < 0.01);
        } else {
            panic!("Expected Statistics output");
        }
    }

    #[test]
    #[cfg(target_arch = "aarch64")]
    fn test_quality_aggregation_neon_matches_naive() {
        let records = create_test_records();
        let op = QualityAggregation;

        let naive_result = op.execute_naive(&records).unwrap();
        let neon_result = op.execute_neon(&records).unwrap();

        if let (OperationOutput::Statistics(naive_val), OperationOutput::Statistics(neon_val)) =
            (naive_result, neon_result)
        {
            let naive_stats: QualityStats = serde_json::from_value(naive_val).unwrap();
            let neon_stats: QualityStats = serde_json::from_value(neon_val).unwrap();

            assert_eq!(naive_stats.min_quality, neon_stats.min_quality);
            assert_eq!(naive_stats.max_quality, neon_stats.max_quality);
            assert_eq!(naive_stats.total_quality, neon_stats.total_quality);
            assert_eq!(naive_stats.num_bases, neon_stats.num_bases);
            assert!((naive_stats.mean_quality - neon_stats.mean_quality).abs() < 0.01);
        } else {
            panic!("Expected Statistics output");
        }
    }

    #[test]
    fn test_quality_aggregation_parallel() {
        let records = create_test_records();
        let op = QualityAggregation;

        let result = op.execute_parallel(&records, 2).unwrap();

        if let OperationOutput::Statistics(value) = result {
            let stats: QualityStats = serde_json::from_value(value).unwrap();

            assert_eq!(stats.min_quality, 10);
            assert_eq!(stats.max_quality, 45);
            assert_eq!(stats.total_quality, 330);
            assert_eq!(stats.num_bases, 12);
            assert!((stats.mean_quality - 27.5).abs() < 0.01);
        } else {
            panic!("Expected Statistics output");
        }
    }

    #[test]
    fn test_quality_aggregation_empty() {
        let records: Vec<SequenceRecord> = vec![];
        let op = QualityAggregation;

        let result = op.execute_naive(&records).unwrap();

        if let OperationOutput::Statistics(value) = result {
            let stats: QualityStats = serde_json::from_value(value).unwrap();

            assert_eq!(stats.num_bases, 0);
            assert_eq!(stats.mean_quality, 0.0);
        } else {
            panic!("Expected Statistics output");
        }
    }
}
