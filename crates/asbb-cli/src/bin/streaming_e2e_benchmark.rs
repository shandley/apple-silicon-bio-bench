#!/usr/bin/env cargo
//! Streaming End-to-End Pipeline Benchmark
//!
//! Measures real-world performance of streaming pipeline with file I/O.
//! Tests: Read FASTQ (gzip) → Process → Filter → Write output
//! Validates that NEON speedup is preserved with streaming architecture.

use anyhow::{Context, Result};
use asbb_core::{OperationOutput, PrimitiveOperation, SequenceRecord};
use asbb_ops::{
    base_counting::BaseCounting,
    gc_content::GcContent,
};
use flate2::read::GzDecoder;
use flate2::write::GzEncoder;
use flate2::Compression;
use std::fs::File;
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::path::Path;
use std::process::Command;
use std::time::Instant;

#[derive(Debug, Clone)]
enum Config {
    Naive,
    Neon,
}

#[derive(Debug)]
struct BenchmarkConfig {
    operation: String,
    scale_name: String,
    input_path: String,
    num_sequences: usize,
    config: Config,
    repetitions: usize,
}

#[derive(Debug)]
struct E2EResult {
    operation: String,
    scale: String,
    num_sequences: usize,
    config_name: String,

    throughput_median: f64,
    throughput_mean: f64,

    peak_memory_mb: f64,
    baseline_memory_mb: f64,

    elapsed_median: f64,
    elapsed_mean: f64,
}

fn main() -> Result<()> {
    println!("================================================================================");
    println!("STREAMING END-TO-END PIPELINE BENCHMARK");
    println!("================================================================================");
    println!();

    // Define test matrix
    let operations = vec![
        ("base_counting", Box::new(BaseCounting::new()) as Box<dyn PrimitiveOperation>),
        ("gc_content", Box::new(GcContent::new()) as Box<dyn PrimitiveOperation>),
    ];

    let scales = vec![
        ("Medium", "datasets/medium_10k_150bp.fq.gz", 10_000),
        ("Large", "datasets/large_100k_150bp.fq.gz", 100_000),
        ("VeryLarge", "datasets/vlarge_1m_150bp.fq.gz", 1_000_000),
    ];

    let configs = vec![
        ("naive", Config::Naive),
        ("neon", Config::Neon),
    ];

    let repetitions = 30;

    println!("Test Matrix:");
    println!("  Operations: {}", operations.len());
    println!("  Scales: {}", scales.len());
    println!("  Configs: {}", configs.len());
    println!("  Repetitions: {}", repetitions);
    println!("  Total experiments: {}", operations.len() * scales.len() * configs.len());
    println!();

    // Check that input files exist
    println!("Checking input files...");
    for (scale_name, path, _) in &scales {
        if Path::new(path).exists() {
            println!("  ✓ {}: {}", scale_name, path);
        } else {
            println!("  ⚠️  {}: {} NOT FOUND (will skip)", scale_name, path);
        }
    }
    println!();

    let mut results = Vec::new();

    for (op_name, op) in &operations {
        for (scale_name, input_path, num_sequences) in &scales {
            // Skip if file doesn't exist
            if !Path::new(input_path).exists() {
                println!("⚠️  Skipping {} @ {} (file not found)", op_name, scale_name);
                continue;
            }

            println!("Testing {} @ {} ({} sequences)", op_name, scale_name, num_sequences);

            for (config_name, config) in &configs {
                let bench_config = BenchmarkConfig {
                    operation: op_name.to_string(),
                    scale_name: scale_name.to_string(),
                    input_path: input_path.to_string(),
                    num_sequences: *num_sequences,
                    config: config.clone(),
                    repetitions,
                };

                print!("  {} ... ", config_name);
                std::io::stdout().flush().ok();

                match run_benchmark(&bench_config, &**op) {
                    Ok(result) => {
                        println!("{:.1} Kseq/s, {:.1} MB peak",
                                 result.throughput_median / 1000.0,
                                 result.peak_memory_mb);
                        results.push(result);
                    }
                    Err(e) => {
                        println!("❌ FAILED: {}", e);
                    }
                }
            }
            println!();
        }
    }

    // Write results to CSV
    write_results_csv(&results, "results/streaming/streaming_e2e_n30.csv")?;

    // Calculate speedup stats
    calculate_speedup_stats(&results);

    println!("================================================================================");
    println!("BENCHMARK COMPLETE");
    println!("================================================================================");
    println!("Results written to: results/streaming/streaming_e2e_n30.csv");
    println!("Total experiments: {}", results.len());
    println!();

    Ok(())
}

fn run_benchmark(
    config: &BenchmarkConfig,
    op: &dyn PrimitiveOperation,
) -> Result<E2EResult> {
    let mut elapsed_times = Vec::with_capacity(config.repetitions);
    let mut peak_memories = Vec::with_capacity(config.repetitions);

    // Warmup (3 runs)
    for _ in 0..3 {
        let _ = run_single_pipeline(config, op)?;
    }

    // Measure
    for _ in 0..config.repetitions {
        let (elapsed, peak_mem) = run_single_pipeline(config, op)?;
        elapsed_times.push(elapsed);
        peak_memories.push(peak_mem);
    }

    // Calculate statistics
    let elapsed_median = median(&elapsed_times);
    let elapsed_mean = elapsed_times.iter().sum::<f64>() / elapsed_times.len() as f64;

    let throughputs: Vec<f64> = elapsed_times.iter()
        .map(|&t| config.num_sequences as f64 / t)
        .collect();

    let throughput_median = median(&throughputs);
    let throughput_mean = throughputs.iter().sum::<f64>() / throughputs.len() as f64;

    let peak_memory_mb = median(&peak_memories);
    let baseline_memory_mb = measure_rss_mb()?;

    Ok(E2EResult {
        operation: config.operation.clone(),
        scale: config.scale_name.clone(),
        num_sequences: config.num_sequences,
        config_name: format!("{:?}", config.config).to_lowercase(),
        throughput_median,
        throughput_mean,
        peak_memory_mb,
        baseline_memory_mb,
        elapsed_median,
        elapsed_mean,
    })
}

fn run_single_pipeline(
    config: &BenchmarkConfig,
    op: &dyn PrimitiveOperation,
) -> Result<(f64, f64)> {
    let start = Instant::now();
    let mut peak_mem = measure_rss_mb()?;

    // Output path (temporary, will be deleted)
    let output_path = format!("/tmp/streaming_output_{}.fq.gz",
                             std::process::id());

    // Open input (gzip compressed FASTQ)
    let file = File::open(&config.input_path)
        .with_context(|| format!("Failed to open input: {}", config.input_path))?;
    let decoder = GzDecoder::new(file);
    let reader = BufReader::new(decoder);
    let mut lines = reader.lines();

    // Open output (gzip compressed FASTQ)
    let out_file = File::create(&output_path)?;
    let encoder = GzEncoder::new(out_file, Compression::default());
    let mut writer = BufWriter::new(encoder);

    let mut count = 0;

    // Streaming pipeline: Read → Process → Filter → Write
    while let Some(header_line) = lines.next() {
        let header = header_line?;
        if !header.starts_with('@') {
            continue; // Skip malformed records
        }

        let id = header[1..].to_string();

        let sequence_line = lines.next()
            .ok_or_else(|| anyhow::anyhow!("Unexpected EOF"))??;
        let sequence = sequence_line.into_bytes();

        let _plus_line = lines.next()
            .ok_or_else(|| anyhow::anyhow!("Unexpected EOF"))??;

        let quality_line = lines.next()
            .ok_or_else(|| anyhow::anyhow!("Unexpected EOF"))??;
        let quality = quality_line.into_bytes();

        let record = SequenceRecord {
            id: id.clone(),
            sequence: sequence.clone(),
            quality: Some(quality.clone()),
        };

        // Process (compute operation)
        let _ = execute_operation(op, &record, &config.config)?;

        // Filter (example: keep sequences with mean quality >= 20)
        let mean_quality = calculate_mean_quality(&quality);
        if mean_quality >= 20.0 {
            // Write to output
            writeln!(writer, "@{}", id)?;
            writeln!(writer, "{}", String::from_utf8_lossy(&sequence))?;
            writeln!(writer, "+")?;
            writeln!(writer, "{}", String::from_utf8_lossy(&quality))?;
        }

        count += 1;

        // Measure memory periodically
        if count % 10000 == 0 {
            peak_mem = peak_mem.max(measure_rss_mb()?);
        }
    }

    // Flush and close output
    drop(writer);

    // Final memory measurement
    peak_mem = peak_mem.max(measure_rss_mb()?);

    let elapsed = start.elapsed().as_secs_f64();

    // Clean up output file
    let _ = std::fs::remove_file(output_path);

    Ok((elapsed, peak_mem))
}

fn execute_operation(
    op: &dyn PrimitiveOperation,
    record: &SequenceRecord,
    config: &Config,
) -> Result<OperationOutput> {
    let sequences = std::slice::from_ref(record);
    match config {
        Config::Naive => op.execute_naive(sequences),
        Config::Neon => op.execute_neon(sequences),
    }
}

fn calculate_mean_quality(quality: &[u8]) -> f64 {
    if quality.is_empty() {
        return 0.0;
    }

    let sum: u32 = quality.iter()
        .map(|&q| (q - 33) as u32) // Phred+33 encoding
        .sum();

    sum as f64 / quality.len() as f64
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

    Ok(rss_kb / 1024.0)
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

fn write_results_csv(results: &[E2EResult], path: &str) -> Result<()> {
    // Create directory if needed
    if let Some(parent) = std::path::Path::new(path).parent() {
        std::fs::create_dir_all(parent)?;
    }

    let mut file = File::create(path)?;

    // Write header
    writeln!(
        file,
        "operation,scale,num_sequences,config,\
         throughput_median,throughput_mean,\
         peak_memory_mb,baseline_memory_mb,\
         elapsed_median,elapsed_mean"
    )?;

    // Write data
    for r in results {
        writeln!(
            file,
            "{},{},{},{},{:.2},{:.2},{:.2},{:.2},{:.6},{:.6}",
            r.operation,
            r.scale,
            r.num_sequences,
            r.config_name,
            r.throughput_median,
            r.throughput_mean,
            r.peak_memory_mb,
            r.baseline_memory_mb,
            r.elapsed_median,
            r.elapsed_mean
        )?;
    }

    println!("✅ Results written to: {}", path);

    Ok(())
}

fn calculate_speedup_stats(results: &[E2EResult]) {
    println!();
    println!("================================================================================");
    println!("END-TO-END NEON SPEEDUP SUMMARY");
    println!("================================================================================");
    println!();

    for operation in ["base_counting", "gc_content"] {
        println!("{}: ", operation);

        for scale in ["Medium", "Large", "VeryLarge"] {
            let naive_result = results.iter()
                .find(|r| r.operation == operation
                      && r.scale == scale
                      && r.config_name == "naive");

            let neon_result = results.iter()
                .find(|r| r.operation == operation
                      && r.scale == scale
                      && r.config_name == "neon");

            if let (Some(naive), Some(neon)) = (naive_result, neon_result) {
                let speedup = naive.throughput_median / neon.throughput_median;
                println!("  {}: {:.2}× speedup ({:.1} → {:.1} Kseq/s)",
                         scale,
                         1.0/speedup,
                         naive.throughput_median / 1000.0,
                         neon.throughput_median / 1000.0);
            }
        }
        println!();
    }
}
