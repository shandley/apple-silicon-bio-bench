// Sequence Complexity Score Operation
//
// Calculates sequence complexity (character diversity, entropy-like metric).
// Complexity: ~0.45 (multiple counters + simple calculation)

use crate::PrimitiveOperation;
use asbb_core::{HardwareConfig, OperationCategory, OperationOutput, SequenceRecord};
use anyhow::Result;
use rayon::prelude::*;
use serde::{Deserialize, Serialize};

pub struct ComplexityScore;

impl ComplexityScore {
    pub fn new() -> Self {
        Self
    }

    /// Execute complexity score using GPU (Metal)
    ///
    /// Complexity score is our most complex operation (0.61):
    /// - Character counting (256 possible values)
    /// - Unique character detection (bitset operations)
    /// - Complexity calculation (diversity metric)
    ///
    /// **Hypothesis**: Highest complexity operation, may show different GPU pattern
    #[cfg(all(target_os = "macos", feature = "gpu"))]
    pub fn execute_gpu(&self, data: &[SequenceRecord]) -> Result<(ComplexityResult, asbb_gpu::GpuMetrics)> {
        use asbb_gpu::MetalBackend;

        let backend = MetalBackend::new()?;
        let (complexity_scores, metrics) = backend.calculate_complexity_gpu(data)?;

        let mut total_complexity = 0.0;
        let mut low_count = 0;
        let mut high_count = 0;

        for &complexity in &complexity_scores {
            total_complexity += complexity;
            if complexity < 0.4 {
                low_count += 1;
            } else if complexity > 0.7 {
                high_count += 1;
            }
        }

        let result = ComplexityResult {
            total_sequences: data.len(),
            mean_complexity: if data.is_empty() { 0.0 } else { total_complexity / data.len() as f64 },
            low_complexity_count: low_count,
            high_complexity_count: high_count,
        };

        Ok((result, metrics))
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ComplexityResult {
    pub total_sequences: usize,
    pub mean_complexity: f64,
    pub low_complexity_count: usize,  // < 0.4
    pub high_complexity_count: usize, // > 0.7
}

impl ComplexityResult {
    pub fn new() -> Self {
        Self {
            total_sequences: 0,
            mean_complexity: 0.0,
            low_complexity_count: 0,
            high_complexity_count: 0,
        }
    }
}

impl PrimitiveOperation for ComplexityScore {
    fn name(&self) -> &str {
        "complexity_score"
    }

    fn category(&self) -> OperationCategory {
        OperationCategory::ElementWise
    }

    fn execute_naive(&self, data: &[SequenceRecord]) -> Result<OperationOutput> {
        let mut total_complexity = 0.0;
        let mut low_count = 0;
        let mut high_count = 0;

        for record in data {
            let complexity = calculate_complexity(&record.sequence);
            total_complexity += complexity;
            if complexity < 0.4 {
                low_count += 1;
            } else if complexity > 0.7 {
                high_count += 1;
            }
        }

        let result = ComplexityResult {
            total_sequences: data.len(),
            mean_complexity: if data.is_empty() { 0.0 } else { total_complexity / data.len() as f64 },
            low_complexity_count: low_count,
            high_complexity_count: high_count,
        };

        Ok(OperationOutput::Statistics(serde_json::to_value(result)?))
    }

    fn execute_neon(&self, data: &[SequenceRecord]) -> Result<OperationOutput> {
        // NEON benefit minimal for this operation (too simple)
        self.execute_naive(data)
    }

    fn execute_parallel(
        &self,
        data: &[SequenceRecord],
        num_threads: usize,
    ) -> Result<OperationOutput> {
        let pool = rayon::ThreadPoolBuilder::new()
            .num_threads(num_threads)
            .build()?;

        let (total_complexity, low_count, high_count) = pool.install(|| {
            data.par_iter()
                .map(|record| {
                    let complexity = calculate_complexity(&record.sequence);
                    let low = if complexity < 0.4 { 1 } else { 0 };
                    let high = if complexity > 0.7 { 1 } else { 0 };
                    (complexity, low, high)
                })
                .reduce(
                    || (0.0, 0, 0),
                    |(sum1, low1, high1), (sum2, low2, high2)| {
                        (sum1 + sum2, low1 + low2, high1 + high2)
                    },
                )
        });

        let result = ComplexityResult {
            total_sequences: data.len(),
            mean_complexity: if data.is_empty() { 0.0 } else { total_complexity / data.len() as f64 },
            low_complexity_count: low_count,
            high_complexity_count: high_count,
        };

        Ok(OperationOutput::Statistics(serde_json::to_value(result)?))
    }

    fn execute_with_config(
        &self,
        data: &[SequenceRecord],
        config: &HardwareConfig,
    ) -> Result<OperationOutput> {
        if config.num_threads > 1 {
            self.execute_parallel(data, config.num_threads)
        } else {
            self.execute_naive(data)
        }
    }
}

// Calculate sequence complexity (0.0 = low, 1.0 = high)
// Based on character diversity
fn calculate_complexity(seq: &[u8]) -> f64 {
    if seq.is_empty() {
        return 0.0;
    }

    let mut counts = [0usize; 256];
    for &base in seq {
        counts[base as usize] += 1;
    }

    // Count unique characters
    let unique = counts.iter().filter(|&&c| c > 0).count();

    // Simple complexity: ratio of unique chars to possible chars (normalized)
    let max_unique = seq.len().min(4); // Max 4 for ACGT
    unique as f64 / max_unique as f64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_complexity_score() {
        let records = vec![
            SequenceRecord::fasta("low".to_string(), b"AAAAAAAAAA".to_vec()),  // Low complexity
            SequenceRecord::fasta("high".to_string(), b"ACGTACGTAC".to_vec()), // High complexity
            SequenceRecord::fasta("med".to_string(), b"AACCGGTT".to_vec()),    // Medium
        ];

        let op = ComplexityScore;
        let result = op.execute_naive(&records).unwrap();

        if let OperationOutput::Statistics(value) = result {
            let complexity_result: ComplexityResult = serde_json::from_value(value).unwrap();
            assert_eq!(complexity_result.total_sequences, 3);
            assert!(complexity_result.low_complexity_count > 0);
        }
    }
}
