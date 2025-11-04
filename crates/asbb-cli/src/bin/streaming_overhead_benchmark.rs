#!/usr/bin/env cargo
//! Streaming Overhead Benchmark
//!
//! Measures the performance cost of streaming pattern vs batch processing.
//! Compares throughput to quantify iterator/abstraction overhead.

use anyhow::Result;
use asbb_core::{OperationOutput, PrimitiveOperation, SequenceRecord};
use asbb_ops::{
    base_counting::BaseCounting,
    gc_content::GcContent,
    quality_filter::QualityFilter,
};
use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;
use std::fs::File;
use std::io::Write;
use std::time::Instant;

#[derive(Debug, Clone)]
enum Config {
    Naive,
    Neon,
}

#[derive(Debug, Clone)]
enum Pattern {
    Batch,
    Streaming,
}

#[derive(Debug)]
struct BenchmarkConfig {
    operation: String,
    scale_name: String,
    num_sequences: usize,
    config: Config,
    pattern: Pattern,
    repetitions: usize,
}

#[derive(Debug)]
struct OverheadResult {
    operation: String,
    scale: String,
    num_sequences: usize,
    config_name: String,
    pattern_name: String,

    throughput_median: f64,
    throughput_mean: f64,
    throughput_std_dev: f64,

    elapsed_median: f64,
    elapsed_mean: f64,
}

fn main() -> Result<()> {
    println!("================================================================================");
    println!("STREAMING OVERHEAD BENCHMARK");
    println!("================================================================================");
    println!();

    // Define test matrix
    let operations = vec![
        ("base_counting", Box::new(BaseCounting::new()) as Box<dyn PrimitiveOperation>),
        ("gc_content", Box::new(GcContent::new()) as Box<dyn PrimitiveOperation>),
        ("quality_filter", Box::new(QualityFilter::new(20)) as Box<dyn PrimitiveOperation>),
    ];

    let scales = vec![
        ("Small", 1_000),
        ("Medium", 10_000),
        ("Large", 100_000),
        ("VeryLarge", 1_000_000),
    ];

    let configs = vec![
        ("naive", Config::Naive),
        ("neon", Config::Neon),
    ];

    let patterns = vec![
        ("batch", Pattern::Batch),
        ("streaming", Pattern::Streaming),
    ];

    let repetitions = 30;

    println!("Test Matrix:");
    println!("  Operations: {}", operations.len());
    println!("  Scales: {}", scales.len());
    println!("  Configs: {}", configs.len());
    println!("  Patterns: {}", patterns.len());
    println!("  Repetitions: {}", repetitions);
    println!("  Total experiments: {}",
             operations.len() * scales.len() * configs.len() * patterns.len());
    println!();

    // Pre-generate sequences for all scales (reused across experiments)
    println!("Pre-generating sequence datasets...");
    let mut datasets = std::collections::HashMap::new();
    for (scale_name, num_sequences) in &scales {
        print!("  Generating {} ({} sequences) ... ", scale_name, num_sequences);
        std::io::stdout().flush().ok();
        datasets.insert(scale_name, generate_sequences(*num_sequences)?);
        println!("✓");
    }
    println!();

    let mut results = Vec::new();

    for (op_name, op) in &operations {
        for (scale_name, num_sequences) in &scales {
            println!("Testing {} @ {} ({} sequences)", op_name, scale_name, num_sequences);

            let sequences = datasets.get(scale_name).unwrap();

            for (config_name, config) in &configs {
                for (pattern_name, pattern) in &patterns {
                    let bench_config = BenchmarkConfig {
                        operation: op_name.to_string(),
                        scale_name: scale_name.to_string(),
                        num_sequences: *num_sequences,
                        config: config.clone(),
                        pattern: pattern.clone(),
                        repetitions,
                    };

                    print!("  {} + {:10} ... ", config_name, pattern_name);
                    std::io::stdout().flush().ok();

                    match run_benchmark(&bench_config, &**op, sequences) {
                        Ok(result) => {
                            println!("{:.1} Kseq/s", result.throughput_median / 1000.0);
                            results.push(result);
                        }
                        Err(e) => {
                            println!("❌ FAILED: {}", e);
                        }
                    }
                }
            }
            println!();
        }
    }

    // Write results to CSV
    write_results_csv(&results, "results/streaming/streaming_overhead_n30.csv")?;

    // Calculate and display overhead statistics
    calculate_overhead_stats(&results);

    println!("================================================================================");
    println!("BENCHMARK COMPLETE");
    println!("================================================================================");
    println!("Results written to: results/streaming/streaming_overhead_n30.csv");
    println!("Total experiments: {}", results.len());
    println!();

    Ok(())
}

fn run_benchmark(
    config: &BenchmarkConfig,
    op: &dyn PrimitiveOperation,
    sequences: &[SequenceRecord],
) -> Result<OverheadResult> {
    let mut elapsed_times = Vec::with_capacity(config.repetitions);

    // Warmup (3 runs)
    for _ in 0..3 {
        let _ = run_single_benchmark(config, op, sequences)?;
    }

    // Measure
    for _ in 0..config.repetitions {
        let elapsed = run_single_benchmark(config, op, sequences)?;
        elapsed_times.push(elapsed);
    }

    // Calculate statistics
    let elapsed_median = median(&elapsed_times);
    let elapsed_mean = elapsed_times.iter().sum::<f64>() / elapsed_times.len() as f64;

    let throughputs: Vec<f64> = elapsed_times.iter()
        .map(|&t| config.num_sequences as f64 / t)
        .collect();

    let throughput_median = median(&throughputs);
    let throughput_mean = throughputs.iter().sum::<f64>() / throughputs.len() as f64;
    let throughput_std_dev = std_dev(&throughputs, throughput_mean);

    Ok(OverheadResult {
        operation: config.operation.clone(),
        scale: config.scale_name.clone(),
        num_sequences: config.num_sequences,
        config_name: format!("{:?}", config.config).to_lowercase(),
        pattern_name: format!("{:?}", config.pattern).to_lowercase(),
        throughput_median,
        throughput_mean,
        throughput_std_dev,
        elapsed_median,
        elapsed_mean,
    })
}

fn run_single_benchmark(
    config: &BenchmarkConfig,
    op: &dyn PrimitiveOperation,
    sequences: &[SequenceRecord],
) -> Result<f64> {
    let start = Instant::now();

    match config.pattern {
        Pattern::Batch => {
            // Batch: Process all sequences at once
            let _ = execute_operation(op, sequences, &config.config)?;
        }
        Pattern::Streaming => {
            // Streaming: Process one sequence at a time
            for seq in sequences {
                let _ = execute_operation(op, std::slice::from_ref(seq), &config.config)?;
            }
        }
    }

    Ok(start.elapsed().as_secs_f64())
}

fn generate_sequences(count: usize) -> Result<Vec<SequenceRecord>> {
    let mut rng = ChaCha8Rng::seed_from_u64(42);
    let mut sequences = Vec::with_capacity(count);

    const SEQ_LENGTH: usize = 150;
    const BASES: &[u8] = b"ACGT";

    for i in 0..count {
        let mut sequence = Vec::with_capacity(SEQ_LENGTH);
        let mut quality = Vec::with_capacity(SEQ_LENGTH);

        for _ in 0..SEQ_LENGTH {
            sequence.push(BASES[rng.gen_range(0..4)]);
            quality.push(b'I'); // Q40
        }

        sequences.push(SequenceRecord {
            id: format!("seq_{}", i),
            sequence,
            quality: Some(quality),
        });
    }

    Ok(sequences)
}

fn execute_operation(
    op: &dyn PrimitiveOperation,
    sequences: &[SequenceRecord],
    config: &Config,
) -> Result<OperationOutput> {
    match config {
        Config::Naive => op.execute_naive(sequences),
        Config::Neon => op.execute_neon(sequences),
    }
}

fn median(values: &[f64]) -> f64 {
    let mut sorted = values.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let mid = sorted.len() / 2;
    if sorted.len() % 2 == 0 {
        (sorted[mid - 1] + sorted[mid]) / 2.0
    } else {
        sorted[mid]
    }
}

fn std_dev(values: &[f64], mean: f64) -> f64 {
    let variance = values.iter()
        .map(|&v| (v - mean).powi(2))
        .sum::<f64>() / values.len() as f64;
    variance.sqrt()
}

fn write_results_csv(results: &[OverheadResult], path: &str) -> Result<()> {
    // Create directory if needed
    if let Some(parent) = std::path::Path::new(path).parent() {
        std::fs::create_dir_all(parent)?;
    }

    let mut file = File::create(path)?;

    // Write header
    writeln!(
        file,
        "operation,scale,num_sequences,config,pattern,\
         throughput_median,throughput_mean,throughput_std_dev,\
         elapsed_median,elapsed_mean"
    )?;

    // Write data
    for r in results {
        writeln!(
            file,
            "{},{},{},{},{},{:.2},{:.2},{:.2},{:.6},{:.6}",
            r.operation,
            r.scale,
            r.num_sequences,
            r.config_name,
            r.pattern_name,
            r.throughput_median,
            r.throughput_mean,
            r.throughput_std_dev,
            r.elapsed_median,
            r.elapsed_mean
        )?;
    }

    println!("✅ Results written to: {}", path);

    Ok(())
}

fn calculate_overhead_stats(results: &[OverheadResult]) {
    println!();
    println!("================================================================================");
    println!("STREAMING OVERHEAD SUMMARY");
    println!("================================================================================");
    println!();

    // Group by operation and config, calculate overhead
    let mut overheads = Vec::new();

    for operation in ["base_counting", "gc_content", "quality_filter"] {
        for config in ["naive", "neon"] {
            // Find batch and streaming results
            let batch_results: Vec<_> = results.iter()
                .filter(|r| r.operation == operation
                        && r.config_name == config
                        && r.pattern_name == "batch")
                .collect();

            let streaming_results: Vec<_> = results.iter()
                .filter(|r| r.operation == operation
                        && r.config_name == config
                        && r.pattern_name == "streaming")
                .collect();

            if batch_results.len() == streaming_results.len() {
                let batch_avg: f64 = batch_results.iter()
                    .map(|r| r.throughput_median)
                    .sum::<f64>() / batch_results.len() as f64;

                let streaming_avg: f64 = streaming_results.iter()
                    .map(|r| r.throughput_median)
                    .sum::<f64>() / streaming_results.len() as f64;

                let overhead_pct = ((batch_avg - streaming_avg) / batch_avg) * 100.0;

                println!("{} + {}:", operation, config);
                println!("  Batch:     {:.1} Kseq/s", batch_avg / 1000.0);
                println!("  Streaming: {:.1} Kseq/s", streaming_avg / 1000.0);
                println!("  Overhead:  {:.1}%", overhead_pct);
                println!();

                overheads.push(overhead_pct);
            }
        }
    }

    let avg_overhead = overheads.iter().sum::<f64>() / overheads.len() as f64;
    println!("Average streaming overhead: {:.1}%", avg_overhead);
    println!();
}
