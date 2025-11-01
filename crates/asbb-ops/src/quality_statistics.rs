//! Quality statistics operation
//!
//! Computes per-position quality score statistics across all sequences.
//!
//! **Operation Category**: Aggregation
//! - Vectorizable (parallel statistics computation)
//! - Compute-bound (returns statistics, not modified sequences)
//! - Common in quality control reports (FastQC-like)
//!
//! **Expected Performance** (based on Phase 1 + recent findings):
//! - Complexity: 0.38 (aggregation with some computation)
//! - NEON: ~20-40× speedup (compute-bound, returns aggregated numbers)
//! - Parallel: ~2-4× additional speedup on 4 cores
//!
//! # Apple Silicon Considerations
//!
//! - **NEON**: Vectorized mean computation (parallel sums)
//! - **Statistics computed**: Mean, median, Q1 (25th percentile), Q3 (75th percentile)
//! - **Memory pattern**: Sequential reads of quality scores
//! - **Output**: Statistics per position (fixed-size, compute-bound)
//!
//! # Quality Statistics Definition
//!
//! For each position in sequences (e.g., position 0-149 for 150bp reads):
//! - Collect all quality scores at that position across sequences
//! - Compute: mean, median, Q1, Q3
//! - Used for quality profile visualization and QC

use anyhow::Result;
use asbb_core::{OperationCategory, OperationOutput, PrimitiveOperation, SequenceRecord};
use rayon::prelude::*;
use serde::{Deserialize, Serialize};

/// Quality statistics operation
pub struct QualityStatistics;

impl QualityStatistics {
    pub fn new() -> Self {
        Self
    }

    /// Compute statistics for a single position (naive)
    fn position_stats_naive(&self, qualities: &mut [u8]) -> PositionStats {
        if qualities.is_empty() {
            return PositionStats::default();
        }

        // Mean
        let sum: u64 = qualities.iter().map(|&q| q as u64).sum();
        let mean = sum as f64 / qualities.len() as f64;

        // Median and quartiles (requires sorting)
        qualities.sort_unstable();
        let median = percentile(qualities, 50.0);
        let q1 = percentile(qualities, 25.0);
        let q3 = percentile(qualities, 75.0);

        PositionStats {
            mean,
            median,
            q1,
            q3,
            count: qualities.len(),
        }
    }

    /// Compute statistics for a single position (NEON-accelerated mean)
    #[cfg(target_arch = "aarch64")]
    fn position_stats_neon(&self, qualities: &mut [u8]) -> PositionStats {
        use std::arch::aarch64::*;

        if qualities.is_empty() {
            return PositionStats::default();
        }

        // Compute mean with NEON (vectorized sum)
        let mut sum = 0u64;
        let mut i = 0;

        // Process 16 quality scores at a time
        while i + 16 <= qualities.len() {
            unsafe {
                let vec = vld1q_u8(qualities.as_ptr().add(i));

                // Widen to 16-bit and accumulate
                let vec16_low = vmovl_u8(vget_low_u8(vec));
                let vec16_high = vmovl_u8(vget_high_u8(vec));

                // Widen to 32-bit and accumulate
                let vec32_1 = vmovl_u16(vget_low_u16(vec16_low));
                let vec32_2 = vmovl_u16(vget_high_u16(vec16_low));
                let vec32_3 = vmovl_u16(vget_low_u16(vec16_high));
                let vec32_4 = vmovl_u16(vget_high_u16(vec16_high));

                // Widen to 64-bit and accumulate
                let vec64_1 = vmovl_u32(vget_low_u32(vec32_1));
                let vec64_2 = vmovl_u32(vget_high_u32(vec32_1));
                let vec64_3 = vmovl_u32(vget_low_u32(vec32_2));
                let vec64_4 = vmovl_u32(vget_high_u32(vec32_2));
                let vec64_5 = vmovl_u32(vget_low_u32(vec32_3));
                let vec64_6 = vmovl_u32(vget_high_u32(vec32_3));
                let vec64_7 = vmovl_u32(vget_low_u32(vec32_4));
                let vec64_8 = vmovl_u32(vget_high_u32(vec32_4));

                // Sum all 64-bit values
                sum += vgetq_lane_u64(vec64_1, 0) + vgetq_lane_u64(vec64_1, 1);
                sum += vgetq_lane_u64(vec64_2, 0) + vgetq_lane_u64(vec64_2, 1);
                sum += vgetq_lane_u64(vec64_3, 0) + vgetq_lane_u64(vec64_3, 1);
                sum += vgetq_lane_u64(vec64_4, 0) + vgetq_lane_u64(vec64_4, 1);
                sum += vgetq_lane_u64(vec64_5, 0) + vgetq_lane_u64(vec64_5, 1);
                sum += vgetq_lane_u64(vec64_6, 0) + vgetq_lane_u64(vec64_6, 1);
                sum += vgetq_lane_u64(vec64_7, 0) + vgetq_lane_u64(vec64_7, 1);
                sum += vgetq_lane_u64(vec64_8, 0) + vgetq_lane_u64(vec64_8, 1);
            }

            i += 16;
        }

        // Process remainder
        for j in i..qualities.len() {
            sum += qualities[j] as u64;
        }

        let mean = sum as f64 / qualities.len() as f64;

        // Median and quartiles still require sorting (not vectorizable)
        qualities.sort_unstable();
        let median = percentile(qualities, 50.0);
        let q1 = percentile(qualities, 25.0);
        let q3 = percentile(qualities, 75.0);

        PositionStats {
            mean,
            median,
            q1,
            q3,
            count: qualities.len(),
        }
    }

    #[cfg(not(target_arch = "aarch64"))]
    fn position_stats_neon(&self, qualities: &mut [u8]) -> PositionStats {
        self.position_stats_naive(qualities)
    }

    /// Compute statistics for all positions (naive)
    fn compute_all_stats_naive(&self, data: &[SequenceRecord]) -> Result<Vec<PositionStats>> {
        // Determine max sequence length
        let max_len = data
            .iter()
            .filter_map(|r| r.quality.as_ref().map(|q| q.len()))
            .max()
            .unwrap_or(0);

        if max_len == 0 {
            anyhow::bail!("No quality scores found in sequences (FASTA format?)");
        }

        // Collect quality scores per position
        let mut per_position: Vec<Vec<u8>> = vec![Vec::new(); max_len];

        for record in data {
            if let Some(quality) = &record.quality {
                for (pos, &qual) in quality.iter().enumerate() {
                    if pos < max_len {
                        per_position[pos].push(qual);
                    }
                }
            }
        }

        // Compute statistics for each position
        let stats: Vec<PositionStats> = per_position
            .iter_mut()
            .map(|qualities| self.position_stats_naive(qualities))
            .collect();

        Ok(stats)
    }

    /// Compute statistics for all positions (NEON)
    fn compute_all_stats_neon(&self, data: &[SequenceRecord]) -> Result<Vec<PositionStats>> {
        let max_len = data
            .iter()
            .filter_map(|r| r.quality.as_ref().map(|q| q.len()))
            .max()
            .unwrap_or(0);

        if max_len == 0 {
            anyhow::bail!("No quality scores found in sequences");
        }

        let mut per_position: Vec<Vec<u8>> = vec![Vec::new(); max_len];

        for record in data {
            if let Some(quality) = &record.quality {
                for (pos, &qual) in quality.iter().enumerate() {
                    if pos < max_len {
                        per_position[pos].push(qual);
                    }
                }
            }
        }

        let stats: Vec<PositionStats> = per_position
            .iter_mut()
            .map(|qualities| self.position_stats_neon(qualities))
            .collect();

        Ok(stats)
    }
}

impl Default for QualityStatistics {
    fn default() -> Self {
        Self::new()
    }
}

impl PrimitiveOperation for QualityStatistics {
    fn name(&self) -> &str {
        "quality_statistics"
    }

    fn category(&self) -> OperationCategory {
        OperationCategory::Aggregation
    }

    fn execute_naive(&self, data: &[SequenceRecord]) -> Result<OperationOutput> {
        let stats = self.compute_all_stats_naive(data)?;

        let result = QualityStatisticsResult {
            num_positions: stats.len(),
            num_sequences: data.len(),
            per_position: stats,
        };

        Ok(OperationOutput::Statistics(serde_json::to_value(result)?))
    }

    fn execute_neon(&self, data: &[SequenceRecord]) -> Result<OperationOutput> {
        let stats = self.compute_all_stats_neon(data)?;

        let result = QualityStatisticsResult {
            num_positions: stats.len(),
            num_sequences: data.len(),
            per_position: stats,
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
            let max_len = data
                .iter()
                .filter_map(|r| r.quality.as_ref().map(|q| q.len()))
                .max()
                .unwrap_or(0);

            if max_len == 0 {
                anyhow::bail!("No quality scores found");
            }

            // Parallel collection of quality scores per position
            let per_position: Vec<Vec<u8>> = (0..max_len)
                .into_par_iter()
                .map(|pos| {
                    let mut qualities = Vec::new();
                    for record in data {
                        if let Some(quality) = &record.quality {
                            if pos < quality.len() {
                                qualities.push(quality[pos]);
                            }
                        }
                    }
                    qualities
                })
                .collect();

            // Parallel computation of statistics
            let stats: Vec<PositionStats> = per_position
                .into_par_iter()
                .map(|mut qualities| self.position_stats_naive(&mut qualities))
                .collect();

            let result = QualityStatisticsResult {
                num_positions: stats.len(),
                num_sequences: data.len(),
                per_position: stats,
            };

            Ok(OperationOutput::Statistics(serde_json::to_value(result)?))
        })
    }
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Compute percentile from sorted data
fn percentile(sorted_data: &[u8], percentile: f64) -> f64 {
    if sorted_data.is_empty() {
        return 0.0;
    }

    let index = (percentile / 100.0) * (sorted_data.len() - 1) as f64;
    let lower = index.floor() as usize;
    let upper = index.ceil() as usize;

    if lower == upper {
        sorted_data[lower] as f64
    } else {
        let weight = index - lower as f64;
        (1.0 - weight) * sorted_data[lower] as f64 + weight * sorted_data[upper] as f64
    }
}

// ============================================================================
// Output Types
// ============================================================================

/// Statistics for a single position
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PositionStats {
    /// Mean quality score at this position
    pub mean: f64,
    /// Median quality score
    pub median: f64,
    /// First quartile (25th percentile)
    pub q1: f64,
    /// Third quartile (75th percentile)
    pub q3: f64,
    /// Number of sequences contributing to this position
    pub count: usize,
}

/// Result of quality statistics computation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualityStatisticsResult {
    /// Number of positions analyzed
    pub num_positions: usize,
    /// Number of sequences analyzed
    pub num_sequences: usize,
    /// Statistics per position
    pub per_position: Vec<PositionStats>,
}

impl QualityStatisticsResult {
    /// Get statistics for a specific position
    pub fn get_position(&self, pos: usize) -> Option<&PositionStats> {
        self.per_position.get(pos)
    }

    /// Get overall mean quality across all positions
    pub fn overall_mean(&self) -> f64 {
        if self.per_position.is_empty() {
            return 0.0;
        }

        let sum: f64 = self.per_position.iter().map(|s| s.mean).sum();
        sum / self.per_position.len() as f64
    }

    /// Get positions with mean quality below threshold
    pub fn low_quality_positions(&self, threshold: f64) -> Vec<usize> {
        self.per_position
            .iter()
            .enumerate()
            .filter(|(_, stats)| stats.mean < threshold)
            .map(|(pos, _)| pos)
            .collect()
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_record(id: &str, quality: &[u8]) -> SequenceRecord {
        let sequence = vec![b'A'; quality.len()];
        SequenceRecord::fastq(id.to_string(), sequence, quality.to_vec())
    }

    #[test]
    fn test_percentile() {
        let data = vec![1, 2, 3, 4, 5];

        assert_eq!(percentile(&data, 0.0), 1.0);
        assert_eq!(percentile(&data, 50.0), 3.0);
        assert_eq!(percentile(&data, 100.0), 5.0);
        assert!((percentile(&data, 25.0) - 2.0).abs() < 0.1);
        assert!((percentile(&data, 75.0) - 4.0).abs() < 0.1);
    }

    #[test]
    fn test_position_stats_uniform() {
        let op = QualityStatistics::new();
        let mut qualities = vec![40u8; 10]; // All Q40

        let stats = op.position_stats_naive(&mut qualities);

        assert_eq!(stats.mean, 40.0);
        assert_eq!(stats.median, 40.0);
        assert_eq!(stats.q1, 40.0);
        assert_eq!(stats.q3, 40.0);
        assert_eq!(stats.count, 10);
    }

    #[test]
    fn test_position_stats_varied() {
        let op = QualityStatistics::new();
        let mut qualities = vec![30, 35, 40, 45, 50]; // Range Q30-Q50

        let stats = op.position_stats_naive(&mut qualities);

        assert_eq!(stats.mean, 40.0);
        assert_eq!(stats.median, 40.0);
        assert_eq!(stats.q1, 35.0);
        assert_eq!(stats.q3, 45.0);
        assert_eq!(stats.count, 5);
    }

    #[test]
    fn test_quality_statistics_single_position() {
        let op = QualityStatistics::new();

        let sequences = vec![
            create_test_record("seq1", &[40]),
            create_test_record("seq2", &[50]),
            create_test_record("seq3", &[60]),
        ];

        let result = op.execute_naive(&sequences).unwrap();

        if let OperationOutput::Statistics(json) = result {
            let result: QualityStatisticsResult = serde_json::from_value(json).unwrap();

            assert_eq!(result.num_positions, 1);
            assert_eq!(result.num_sequences, 3);

            let pos0 = result.get_position(0).unwrap();
            assert_eq!(pos0.mean, 50.0);
            assert_eq!(pos0.median, 50.0);
            assert_eq!(pos0.count, 3);
        } else {
            panic!("Expected Statistics output");
        }
    }

    #[test]
    fn test_quality_statistics_multiple_positions() {
        let op = QualityStatistics::new();

        let sequences = vec![
            create_test_record("seq1", &[40, 45, 50]),
            create_test_record("seq2", &[40, 45, 50]),
            create_test_record("seq3", &[40, 45, 50]),
        ];

        let result = op.execute_naive(&sequences).unwrap();

        if let OperationOutput::Statistics(json) = result {
            let result: QualityStatisticsResult = serde_json::from_value(json).unwrap();

            assert_eq!(result.num_positions, 3);
            assert_eq!(result.num_sequences, 3);

            // Position 0: all 40
            let pos0 = result.get_position(0).unwrap();
            assert_eq!(pos0.mean, 40.0);

            // Position 1: all 45
            let pos1 = result.get_position(1).unwrap();
            assert_eq!(pos1.mean, 45.0);

            // Position 2: all 50
            let pos2 = result.get_position(2).unwrap();
            assert_eq!(pos2.mean, 50.0);

            // Overall mean
            assert_eq!(result.overall_mean(), 45.0);
        } else {
            panic!("Expected Statistics output");
        }
    }

    #[test]
    fn test_quality_statistics_variable_lengths() {
        let op = QualityStatistics::new();

        let sequences = vec![
            create_test_record("seq1", &[40, 45]),      // Length 2
            create_test_record("seq2", &[40, 45, 50]),  // Length 3
            create_test_record("seq3", &[40]),          // Length 1
        ];

        let result = op.execute_naive(&sequences).unwrap();

        if let OperationOutput::Statistics(json) = result {
            let result: QualityStatisticsResult = serde_json::from_value(json).unwrap();

            assert_eq!(result.num_positions, 3); // Max length

            // Position 0: 3 sequences contribute
            assert_eq!(result.get_position(0).unwrap().count, 3);
            assert_eq!(result.get_position(0).unwrap().mean, 40.0);

            // Position 1: 2 sequences contribute
            assert_eq!(result.get_position(1).unwrap().count, 2);
            assert_eq!(result.get_position(1).unwrap().mean, 45.0);

            // Position 2: 1 sequence contributes
            assert_eq!(result.get_position(2).unwrap().count, 1);
            assert_eq!(result.get_position(2).unwrap().mean, 50.0);
        } else {
            panic!("Expected Statistics output");
        }
    }

    #[test]
    fn test_low_quality_positions() {
        let op = QualityStatistics::new();

        let sequences = vec![
            create_test_record("seq1", &[40, 25, 50]),
            create_test_record("seq2", &[40, 20, 50]),
        ];

        let result = op.execute_naive(&sequences).unwrap();

        if let OperationOutput::Statistics(json) = result {
            let result: QualityStatisticsResult = serde_json::from_value(json).unwrap();

            let low_positions = result.low_quality_positions(30.0);
            assert_eq!(low_positions, vec![1]); // Position 1 has mean < 30
        } else {
            panic!("Expected Statistics output");
        }
    }

    #[test]
    #[cfg(target_arch = "aarch64")]
    fn test_neon_matches_naive() {
        let op = QualityStatistics::new();

        let sequences: Vec<SequenceRecord> = (0..100)
            .map(|i| {
                let quality: Vec<u8> = (0..150).map(|j| 33 + ((i + j) % 40) as u8).collect();
                create_test_record(&format!("seq{}", i), &quality)
            })
            .collect();

        let naive_result = op.execute_naive(&sequences).unwrap();
        let neon_result = op.execute_neon(&sequences).unwrap();

        match (naive_result, neon_result) {
            (OperationOutput::Statistics(naive_json), OperationOutput::Statistics(neon_json)) => {
                let naive: QualityStatisticsResult = serde_json::from_value(naive_json).unwrap();
                let neon: QualityStatisticsResult = serde_json::from_value(neon_json).unwrap();

                assert_eq!(naive.num_positions, neon.num_positions);
                assert_eq!(naive.num_sequences, neon.num_sequences);

                // Compare means (should be very close, allowing for floating point differences)
                for (i, (n, ne)) in naive
                    .per_position
                    .iter()
                    .zip(neon.per_position.iter())
                    .enumerate()
                {
                    assert!(
                        (n.mean - ne.mean).abs() < 0.01,
                        "Position {} mean mismatch: naive={}, neon={}",
                        i,
                        n.mean,
                        ne.mean
                    );
                }
            }
            _ => panic!("Expected Statistics output"),
        }
    }

    #[test]
    fn test_parallel_execution() {
        let op = QualityStatistics::new();

        let sequences: Vec<SequenceRecord> = (0..100)
            .map(|i| {
                let quality: Vec<u8> = (0..150).map(|j| 33 + ((i + j) % 40) as u8).collect();
                create_test_record(&format!("seq{}", i), &quality)
            })
            .collect();

        let naive_result = op.execute_naive(&sequences).unwrap();
        let parallel_result = op.execute_parallel(&sequences, 4).unwrap();

        match (naive_result, parallel_result) {
            (OperationOutput::Statistics(naive_json), OperationOutput::Statistics(parallel_json)) => {
                let naive: QualityStatisticsResult = serde_json::from_value(naive_json).unwrap();
                let parallel: QualityStatisticsResult =
                    serde_json::from_value(parallel_json).unwrap();

                assert_eq!(naive.num_positions, parallel.num_positions);
                assert_eq!(naive.num_sequences, parallel.num_sequences);
            }
            _ => panic!("Expected Statistics output"),
        }
    }
}
