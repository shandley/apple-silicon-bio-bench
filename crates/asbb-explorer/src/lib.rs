//! Experimental benchmarking harness for Apple Silicon Bio Bench
//!
//! This crate provides the infrastructure to systematically benchmark operations
//! across different hardware configurations and data scales.
//!
//! # Design
//!
//! The benchmarking harness:
//! 1. Loads test data with specific characteristics
//! 2. Runs an operation with a hardware configuration
//! 3. Measures performance metrics (timing, throughput, resources)
//! 4. Validates correctness against reference implementation
//! 5. Stores results for analysis
//!
//! # Apple Silicon Considerations
//!
//! - **Unified Memory**: Zero-copy between CPU/GPU
//! - **Heterogeneous Cores**: P-cores vs E-cores have different performance
//! - **Thermal Throttling**: Long runs may be affected by sustained load
//! - **Background Activity**: QoS scheduling affects priority

#![allow(dead_code)] // Temporary during development
#![allow(unused_variables)]

use anyhow::Result;
use asbb_core::{
    HardwareConfig, OperationOutput, PerformanceResult, PrimitiveOperation, SequenceRecord,
};
use std::time::Instant;

pub mod benchmark;
pub mod runner;
pub mod execution_engine;

pub use benchmark::Benchmark;
pub use runner::BenchmarkRunner;
pub use execution_engine::{ExecutionEngine, ExperimentConfig, ExperimentResult};

/// Benchmark a single operation with a specific configuration
///
/// Runs the operation multiple times (warmup + measured) and collects performance data.
pub fn benchmark_operation(
    operation: &dyn PrimitiveOperation,
    data: &[SequenceRecord],
    config: &HardwareConfig,
    warmup_runs: usize,
    measured_runs: usize,
) -> Result<PerformanceResult> {
    // Warmup runs (not measured)
    for _ in 0..warmup_runs {
        let _ = operation.execute_with_config(data, config)?;
    }

    // Measured runs
    let mut durations = Vec::with_capacity(measured_runs);
    let mut reference_output: Option<OperationOutput> = None;

    for i in 0..measured_runs {
        let start = Instant::now();
        let output = operation.execute_with_config(data, config)?;
        let duration = start.elapsed();

        durations.push(duration);

        // Save first output as reference for correctness validation
        if i == 0 {
            reference_output = Some(output.clone());
        }

        // Validate all outputs match reference
        if let Some(ref expected) = reference_output {
            if output != *expected {
                anyhow::bail!(
                    "Output mismatch on run {}: expected != actual",
                    i
                );
            }
        }
    }

    // Validate against naive baseline for correctness
    let naive_config = HardwareConfig::naive();
    let naive_output = operation.execute_with_config(data, &naive_config)?;
    let output_matches_reference = match (&naive_output, &reference_output) {
        (naive, Some(optimized)) => naive == optimized,
        _ => false,
    };

    // Calculate statistics
    durations.sort();
    let latency_p50 = durations[durations.len() / 2];
    let latency_p99 = durations[(durations.len() * 99) / 100];
    let latency_first_result = durations[0]; // Streaming: time to first result

    // Calculate throughput
    let total_sequences: usize = data.len();
    let total_bases: usize = data.iter().map(|r| r.len()).sum();
    let total_bytes = total_bases; // Approximate (ASCII encoding)

    let throughput_seqs_per_sec = total_sequences as f64 / latency_p50.as_secs_f64();
    let throughput_mbps = (total_bytes as f64 / 1_000_000.0) / latency_p50.as_secs_f64();

    // TODO: Measure actual resource usage (requires OS-specific APIs)
    // For now, use placeholder values
    let memory_peak = 0; // TODO
    let memory_avg = 0; // TODO
    let cpu_utilization = 0.0; // TODO
    let gpu_utilization = if config.use_gpu { Some(0.0) } else { None }; // TODO
    let energy_joules = None; // TODO

    Ok(PerformanceResult {
        throughput_seqs_per_sec,
        throughput_mbps,
        latency_first_result,
        latency_p50,
        latency_p99,
        memory_peak,
        memory_avg,
        cpu_utilization,
        gpu_utilization,
        energy_joules,
        output_matches_reference,
    })
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use asbb_ops::base_counting::BaseCounting;
    use std::time::Duration;

    fn create_test_data(num_sequences: usize, seq_length: usize) -> Vec<SequenceRecord> {
        (0..num_sequences)
            .map(|i| {
                let id = format!("seq_{}", i);
                let sequence = vec![b'A'; seq_length];
                SequenceRecord::fasta(id, sequence)
            })
            .collect()
    }

    #[test]
    fn test_benchmark_operation_naive() {
        let op = BaseCounting::new();
        let data = create_test_data(1000, 150);
        let config = HardwareConfig::naive();

        let result = benchmark_operation(&op, &data, &config, 2, 5).unwrap();

        assert!(result.throughput_seqs_per_sec > 0.0);
        assert!(result.throughput_mbps > 0.0);
        assert!(result.output_matches_reference);
        assert!(result.latency_p50 > Duration::from_nanos(1));
    }

    #[test]
    #[cfg(target_arch = "aarch64")]
    fn test_benchmark_operation_neon() {
        let op = BaseCounting::new();
        // Use larger dataset to see NEON benefits more clearly
        let data = create_test_data(10_000, 150);

        let naive_config = HardwareConfig::naive();
        let mut neon_config = HardwareConfig::naive();
        neon_config.use_neon = true;

        let naive_result = benchmark_operation(&op, &data, &naive_config, 2, 5).unwrap();
        let neon_result = benchmark_operation(&op, &data, &neon_config, 2, 5).unwrap();

        // Both should produce correct results
        assert!(naive_result.output_matches_reference);
        assert!(neon_result.output_matches_reference);

        // Print speedup for visibility (may be <1× in debug builds or small datasets)
        let speedup = neon_result.speedup_vs(&naive_result);
        println!("NEON speedup: {:.2}×", speedup);
        println!(
            "Naive: {:.2} seqs/sec, NEON: {:.2} seqs/sec",
            naive_result.throughput_seqs_per_sec, neon_result.throughput_seqs_per_sec
        );

        // Note: We don't assert NEON is always faster because:
        // - Debug builds may not optimize NEON well
        // - Very small datasets may show overhead
        // - The real speedup (~85×) is seen in release builds with larger datasets
    }

    #[test]
    fn test_benchmark_operation_parallel() {
        let op = BaseCounting::new();
        let data = create_test_data(10_000, 150);

        let naive_config = HardwareConfig::naive();
        let mut parallel_config = HardwareConfig::naive();
        parallel_config.num_threads = 4;

        let naive_result = benchmark_operation(&op, &data, &naive_config, 2, 5).unwrap();
        let parallel_result = benchmark_operation(&op, &data, &parallel_config, 2, 5).unwrap();

        // Parallel should be faster than naive (for large enough data)
        assert!(parallel_result.throughput_seqs_per_sec > naive_result.throughput_seqs_per_sec);

        // Both should produce correct results
        assert!(naive_result.output_matches_reference);
        assert!(parallel_result.output_matches_reference);

        let speedup = parallel_result.speedup_vs(&naive_result);
        println!("Parallel (4 threads) speedup: {:.2}×", speedup);
    }
}
