// Length Filter Operation
//
// Filters sequences based on length threshold (simple comparison).
// Complexity: ~0.40 (simpler than quality filter, no aggregation)

use crate::PrimitiveOperation;
use asbb_core::{HardwareConfig, OperationCategory, OperationOutput, SequenceRecord};
use anyhow::Result;
use rayon::prelude::*;
use serde::{Deserialize, Serialize};

pub struct LengthFilter {
    pub min_length: usize,
}

impl LengthFilter {
    pub fn new(min_length: usize) -> Self {
        Self { min_length }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LengthFilterResult {
    pub total_sequences: usize,
    pub passed_sequences: usize,
    pub filtered_sequences: usize,
    pub pass_rate: f64,
}

impl LengthFilterResult {
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

impl PrimitiveOperation for LengthFilter {
    fn name(&self) -> &str {
        "length_filter"
    }

    fn category(&self) -> OperationCategory {
        OperationCategory::ElementWise
    }

    fn execute_naive(&self, data: &[SequenceRecord]) -> Result<OperationOutput> {
        let mut result = LengthFilterResult::new();

        for record in data {
            result.total_sequences += 1;
            if record.sequence.len() >= self.min_length {
                result.passed_sequences += 1;
            } else {
                result.filtered_sequences += 1;
            }
        }

        result.finalize();
        Ok(OperationOutput::Statistics(serde_json::to_value(result)?))
    }

    fn execute_neon(&self, data: &[SequenceRecord]) -> Result<OperationOutput> {
        // NEON provides minimal benefit for simple length comparison
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

        let threshold = self.min_length;

        let mut result = pool.install(|| {
            data.par_iter()
                .map(|record| {
                    let mut local = LengthFilterResult::new();
                    local.total_sequences = 1;
                    if record.sequence.len() >= threshold {
                        local.passed_sequences = 1;
                    } else {
                        local.filtered_sequences = 1;
                    }
                    local
                })
                .reduce(
                    || LengthFilterResult::new(),
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
        } else {
            self.execute_naive(data)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_length_filter() {
        let records = vec![
            SequenceRecord::fasta("short".to_string(), b"ACGT".to_vec()),           // 4 bp
            SequenceRecord::fasta("medium".to_string(), b"ACGTACGTACGT".to_vec()),  // 12 bp
            SequenceRecord::fasta("long".to_string(), b"ACGTACGTACGTACGTACGT".to_vec()), // 20 bp
        ];

        let op = LengthFilter::new(10);
        let result = op.execute_naive(&records).unwrap();

        if let OperationOutput::Statistics(value) = result {
            let filter_result: LengthFilterResult = serde_json::from_value(value).unwrap();
            assert_eq!(filter_result.total_sequences, 3);
            assert_eq!(filter_result.passed_sequences, 2); // medium + long
            assert_eq!(filter_result.filtered_sequences, 1); // short
        }
    }
}
