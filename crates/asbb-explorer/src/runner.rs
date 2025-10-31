//! Benchmark runner
//!
//! Executes multiple benchmarks and collects results

use anyhow::Result;
use asbb_core::{PerformanceResult, PrimitiveOperation, SequenceRecord};

use crate::Benchmark;

/// Runs multiple benchmarks and collects results
pub struct BenchmarkRunner {
    /// Benchmarks to run
    benchmarks: Vec<Benchmark>,
}

impl BenchmarkRunner {
    /// Create a new benchmark runner
    pub fn new() -> Self {
        Self {
            benchmarks: Vec::new(),
        }
    }

    /// Add a benchmark to the runner
    pub fn add_benchmark(&mut self, benchmark: Benchmark) {
        self.benchmarks.push(benchmark);
    }

    /// Run all benchmarks
    ///
    /// TODO: Implement result storage (Parquet, JSON, etc.)
    pub fn run_all(
        &self,
        operation: &dyn PrimitiveOperation,
        data: &[SequenceRecord],
    ) -> Result<Vec<PerformanceResult>> {
        let mut results = Vec::new();

        for benchmark in &self.benchmarks {
            println!("Running benchmark: {}", benchmark.name);

            let result = crate::benchmark_operation(
                operation,
                data,
                &benchmark.hardware_config,
                benchmark.warmup_runs,
                benchmark.measured_runs,
            )?;

            results.push(result);
        }

        Ok(results)
    }
}

impl Default for BenchmarkRunner {
    fn default() -> Self {
        Self::new()
    }
}
