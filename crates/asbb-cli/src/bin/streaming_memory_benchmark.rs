#!/usr/bin/env cargo
//! Streaming Memory Footprint Benchmark
//!
//! Compares memory usage of batch (load-all) vs streaming patterns
//! to validate claimed memory reduction from streaming architecture.

use anyhow::{Context, Result};
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
use std::process::Command;
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
struct MemoryResult {
    operation: String,
    scale: String,
    num_sequences: usize,
    config_name: String,
    pattern_name: String,

    baseline_mb: f64,
    peak_mb: f64,
    memory_used_mb: f64,
    memory_per_seq_bytes: f64,

    elapsed_sec: f64,
}

fn main() -> Result<()> {
    println!("================================================================================");
    println!("STREAMING MEMORY FOOTPRINT BENCHMARK");
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

    let repetitions = 30; // N=30 for statistical rigor

    println!("Test Matrix:");
    println!("  Operations: {}", operations.len());
    println!("  Scales: {}", scales.len());
    println!("  Configs: {}", configs.len());
    println!("  Patterns: {} (batch, streaming)", patterns.len());
    println!("  Repetitions: {}", repetitions);
    println!("  Total experiments: {}",
             operations.len() * scales.len() * configs.len() * patterns.len());
    println!();

    let mut results = Vec::new();

    for (op_name, op) in &operations {
        for (scale_name, num_sequences) in &scales {
            println!("Testing {} @ {} ({} sequences)", op_name, scale_name, num_sequences);

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

                    match run_benchmark(&bench_config, &**op) {
                        Ok(result) => {
                            println!("{:.1} MB peak, {:.0} bytes/seq",
                                     result.peak_mb, result.memory_per_seq_bytes);
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
    write_results_csv(&results, "results/streaming/streaming_memory_n30.csv")?;

    println!("================================================================================");
    println!("BENCHMARK COMPLETE");
    println!("================================================================================");
    println!("Results written to: results/streaming/streaming_memory_n30.csv");
    println!("Total experiments: {}", results.len());
    println!();

    Ok(())
}

fn run_benchmark(
    config: &BenchmarkConfig,
    op: &dyn PrimitiveOperation,
) -> Result<MemoryResult> {
    // Run multiple repetitions and take median
    let mut peak_mbs = Vec::with_capacity(config.repetitions);
    let mut elapsed_secs = Vec::with_capacity(config.repetitions);

    // Warmup (3 runs)
    for _ in 0..3 {
        let _ = run_single_benchmark(config, op)?;
    }

    // Measure
    for _ in 0..config.repetitions {
        let (peak_mb, elapsed) = run_single_benchmark(config, op)?;
        peak_mbs.push(peak_mb);
        elapsed_secs.push(elapsed);
    }

    // Calculate statistics (use median for robustness)
    let peak_mb = median(&peak_mbs);
    let elapsed_sec = median(&elapsed_secs);

    // Baseline is current process RSS before benchmark
    let baseline_mb = measure_rss_mb()?;

    let memory_used_mb = peak_mb - baseline_mb;
    let memory_per_seq_bytes = (memory_used_mb * 1024.0 * 1024.0) / config.num_sequences as f64;

    Ok(MemoryResult {
        operation: config.operation.clone(),
        scale: config.scale_name.clone(),
        num_sequences: config.num_sequences,
        config_name: format!("{:?}", config.config).to_lowercase(),
        pattern_name: format!("{:?}", config.pattern).to_lowercase(),
        baseline_mb,
        peak_mb,
        memory_used_mb,
        memory_per_seq_bytes,
        elapsed_sec,
    })
}

fn run_single_benchmark(
    config: &BenchmarkConfig,
    op: &dyn PrimitiveOperation,
) -> Result<(f64, f64)> {
    let start = Instant::now();
    let mut peak_mb: f64 = 0.0;

    match config.pattern {
        Pattern::Batch => {
            // Batch: Generate all sequences into Vec, then process
            let sequences = generate_sequences(config.num_sequences)?;
            peak_mb = peak_mb.max(measure_rss_mb()?);

            // Process all
            let _ = execute_operation(op, &sequences, &config.config)?;
            peak_mb = peak_mb.max(measure_rss_mb()?);
        }
        Pattern::Streaming => {
            // Streaming: Generate and process one at a time
            let mut rng = ChaCha8Rng::seed_from_u64(42);

            for i in 0..config.num_sequences {
                let seq = generate_single_sequence(&mut rng, i);

                // Process immediately (no accumulation)
                let _ = execute_single_sequence(op, &seq, &config.config)?;

                // Measure memory periodically (every 1000 sequences)
                if i % 1000 == 0 {
                    peak_mb = peak_mb.max(measure_rss_mb()?);
                }
            }

            // Final measurement
            peak_mb = peak_mb.max(measure_rss_mb()?);
        }
    }

    let elapsed = start.elapsed().as_secs_f64();

    Ok((peak_mb, elapsed))
}

fn generate_sequences(count: usize) -> Result<Vec<SequenceRecord>> {
    let mut rng = ChaCha8Rng::seed_from_u64(42);
    let mut sequences = Vec::with_capacity(count);

    for i in 0..count {
        sequences.push(generate_single_sequence(&mut rng, i));
    }

    Ok(sequences)
}

fn generate_single_sequence(rng: &mut ChaCha8Rng, index: usize) -> SequenceRecord {
    const SEQ_LENGTH: usize = 150;
    const BASES: &[u8] = b"ACGT";

    let mut sequence = Vec::with_capacity(SEQ_LENGTH);
    let mut quality = Vec::with_capacity(SEQ_LENGTH);

    for _ in 0..SEQ_LENGTH {
        sequence.push(BASES[rng.gen_range(0..4)]);
        quality.push(b'I'); // Q40
    }

    SequenceRecord {
        id: format!("seq_{}", index),
        sequence,
        quality: Some(quality),
    }
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

fn execute_single_sequence(
    op: &dyn PrimitiveOperation,
    seq: &SequenceRecord,
    config: &Config,
) -> Result<OperationOutput> {
    // Execute on single sequence (wrapped in slice)
    let sequences = std::slice::from_ref(seq);
    execute_operation(op, sequences, config)
}

fn measure_rss_mb() -> Result<f64> {
    let output = Command::new("ps")
        .args(["-o", "rss=", "-p", &std::process::id().to_string()])
        .output()
        .context("Failed to execute ps command")?;

    let rss_kb = String::from_utf8(output.stdout)?
        .trim()
        .parse::<f64>()
        .context("Failed to parse RSS")?;

    Ok(rss_kb / 1024.0) // Convert KB to MB
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

fn write_results_csv(results: &[MemoryResult], path: &str) -> Result<()> {
    // Create directory if needed
    if let Some(parent) = std::path::Path::new(path).parent() {
        std::fs::create_dir_all(parent)?;
    }

    let mut file = File::create(path)?;

    // Write header
    writeln!(
        file,
        "operation,scale,num_sequences,config,pattern,\
         baseline_mb,peak_mb,memory_used_mb,memory_per_seq_bytes,elapsed_sec"
    )?;

    // Write data
    for r in results {
        writeln!(
            file,
            "{},{},{},{},{},{:.2},{:.2},{:.2},{:.2},{:.4}",
            r.operation,
            r.scale,
            r.num_sequences,
            r.config_name,
            r.pattern_name,
            r.baseline_mb,
            r.peak_mb,
            r.memory_used_mb,
            r.memory_per_seq_bytes,
            r.elapsed_sec
        )?;
    }

    println!("✅ Results written to: {}", path);

    Ok(())
}
