#!/usr/bin/env cargo
//! Simplified I/O Overhead Benchmark
//!
//! Compares in-memory vs file-based execution to quantify I/O overhead
//! Tests 3 compression formats: uncompressed, gzip, zstd

use anyhow::{Context, Result};
use asbb_core::{SequenceRecord, PrimitiveOperation};
use asbb_ops::{
    base_counting::BaseCounting,
    gc_content::GcContent,
};
use flate2::read::GzDecoder;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;
use std::time::Instant;

#[derive(Debug, Clone)]
enum Config {
    Naive,
    Neon,
}

#[derive(Debug, Clone)]
struct BenchmarkConfig {
    operation: String,
    scale: String,
    file_path: String,
    compression: String,
    config: Config,
    repetitions: usize,
}

#[derive(Debug)]
struct BenchmarkResult {
    operation: String,
    scale: String,
    compression: String,
    config_name: String,

    // In-memory baseline (no I/O)
    memory_median: f64,
    memory_mean: f64,

    // File-based (with I/O)
    file_total_median: f64,
    file_total_mean: f64,
    file_load_median: f64,
    file_load_mean: f64,

    // Derived metrics
    io_overhead_pct: f64,
    amdahl_max_speedup: f64,
}

fn main() -> Result<()> {
    println!("================================================================================");
    println!("SIMPLIFIED I/O OVERHEAD BENCHMARK");
    println!("================================================================================");
    println!();

    // Define test matrix (simplified)
    let operations = vec!["base_counting", "gc_content"];
    let scales = vec![
        ("Medium", "datasets/medium_10k_150bp"),
        ("Large", "datasets/large_100k_150bp"),
        ("VeryLarge", "datasets/vlarge_1m_150bp"),
    ];
    let compressions = vec![
        ("uncompressed", ".fq"),
        ("gzip", ".fq.gz"),
        ("zstd", ".fq.zst"),
    ];
    let configs = vec![
        ("naive", Config::Naive),
        ("neon", Config::Neon),
    ];

    let repetitions = 10; // Simplified: N=10 instead of 30

    println!("Test Matrix:");
    println!("  Operations: {} ({:?})", operations.len(), operations);
    println!("  Scales: {}", scales.len());
    println!("  Compressions: {} ({:?})", compressions.len(), compressions.iter().map(|(n,_)| n).collect::<Vec<_>>());
    println!("  Configs: {} (naive, neon)", configs.len());
    println!("  Repetitions: {}", repetitions);
    println!("  Total experiments: {}", operations.len() * scales.len() * compressions.len() * configs.len());
    println!();

    let mut results = Vec::new();

    for op_name in &operations {
        for (scale_name, base_path) in &scales {
            println!("Testing {} @ {}", op_name, scale_name);

            for (compression_name, ext) in &compressions {
                let file_path = format!("{}{}", base_path, ext);

                // Check if file exists
                if !Path::new(&file_path).exists() {
                    println!("  ⚠️  Skipping {} (file not found: {})", compression_name, file_path);
                    continue;
                }

                println!("  📦 {}", compression_name);

                for (config_name, config) in &configs {
                    let bench_config = BenchmarkConfig {
                        operation: op_name.to_string(),
                        scale: scale_name.to_string(),
                        file_path: file_path.clone(),
                        compression: compression_name.to_string(),
                        config: config.clone(),
                        repetitions,
                    };

                    match run_benchmark(&bench_config) {
                        Ok(result) => {
                            println!("    {} - I/O overhead: {:.1}%, Amdahl limit: {:.1}×",
                                     config_name, result.io_overhead_pct, result.amdahl_max_speedup);
                            results.push(result);
                        }
                        Err(e) => {
                            println!("    ❌ {} failed: {}", config_name, e);
                        }
                    }
                }
            }
            println!();
        }
    }

    // Write results to CSV
    write_results_csv(&results, "results/io_overhead/io_overhead_n10.csv")?;

    println!("================================================================================");
    println!("BENCHMARK COMPLETE");
    println!("================================================================================");
    println!("Results written to: results/io_overhead/io_overhead_n10.csv");
    println!("Total experiments: {}", results.len());
    println!();

    Ok(())
}

fn run_benchmark(config: &BenchmarkConfig) -> Result<BenchmarkResult> {
    let op: Box<dyn PrimitiveOperation> = create_operation(&config.operation)?;

    // ========================================================================
    // Phase 1: In-memory baseline (no I/O overhead)
    // ========================================================================

    // Load sequences once
    let sequences = load_sequences(&config.file_path, &config.compression)?;

    // Warm up (3 runs)
    for _ in 0..3 {
        let _ = execute_operation(&*op, &sequences, &config.config)?;
    }

    // Measure in-memory execution (compute only)
    let mut memory_times = Vec::with_capacity(config.repetitions);
    for _ in 0..config.repetitions {
        let start = Instant::now();
        let _ = execute_operation(&*op, &sequences, &config.config)?;
        memory_times.push(start.elapsed().as_secs_f64());
    }

    let memory_median = median(&memory_times);
    let memory_mean = memory_times.iter().sum::<f64>() / memory_times.len() as f64;

    // ========================================================================
    // Phase 2: File-based execution (with I/O)
    // ========================================================================

    let mut file_total_times = Vec::with_capacity(config.repetitions);
    let mut file_load_times = Vec::with_capacity(config.repetitions);

    for _ in 0..config.repetitions {
        // Measure total time (load + execute)
        let start_total = Instant::now();

        // Measure load time
        let start_load = Instant::now();
        let loaded_sequences = load_sequences(&config.file_path, &config.compression)?;
        let load_time = start_load.elapsed().as_secs_f64();

        // Execute operation
        let _ = execute_operation(&*op, &loaded_sequences, &config.config)?;

        let total_time = start_total.elapsed().as_secs_f64();

        file_total_times.push(total_time);
        file_load_times.push(load_time);
    }

    let file_total_median = median(&file_total_times);
    let file_total_mean = file_total_times.iter().sum::<f64>() / file_total_times.len() as f64;
    let file_load_median = median(&file_load_times);
    let file_load_mean = file_load_times.iter().sum::<f64>() / file_load_times.len() as f64;

    // ========================================================================
    // Calculate derived metrics
    // ========================================================================

    // I/O overhead = (total_time - compute_time) / total_time
    let io_overhead_pct = ((file_total_median - memory_median) / file_total_median) * 100.0;

    // Amdahl's law: max_speedup = 1 / (f_serial + (1 - f_serial) / s_parallel)
    // Assume we have a 20× NEON speedup on compute portion
    let f_serial = (file_total_median - memory_median) / file_total_median; // I/O fraction
    let s_parallel = 20.0; // Assumed NEON speedup
    let amdahl_max_speedup = 1.0 / (f_serial + (1.0 - f_serial) / s_parallel);

    Ok(BenchmarkResult {
        operation: config.operation.clone(),
        scale: config.scale.clone(),
        compression: config.compression.clone(),
        config_name: format!("{:?}", config.config).to_lowercase(),

        memory_median,
        memory_mean,

        file_total_median,
        file_total_mean,
        file_load_median,
        file_load_mean,

        io_overhead_pct,
        amdahl_max_speedup,
    })
}

fn create_operation(name: &str) -> Result<Box<dyn PrimitiveOperation>> {
    match name {
        "base_counting" => Ok(Box::new(BaseCounting::new())),
        "gc_content" => Ok(Box::new(GcContent::new())),
        _ => anyhow::bail!("Unsupported operation: {}", name),
    }
}

fn execute_operation(
    op: &dyn PrimitiveOperation,
    sequences: &[SequenceRecord],
    config: &Config,
) -> Result<asbb_core::OperationOutput> {
    match config {
        Config::Naive => op.execute_naive(sequences),
        Config::Neon => op.execute_neon(sequences),
    }
}

fn load_sequences(path: &str, compression: &str) -> Result<Vec<SequenceRecord>> {
    let file = File::open(path)
        .with_context(|| format!("Failed to open file: {}", path))?;

    let reader: Box<dyn BufRead> = match compression {
        "gzip" => Box::new(BufReader::new(GzDecoder::new(file))),
        "zstd" => Box::new(BufReader::new(zstd::Decoder::new(file)?)),
        _ => Box::new(BufReader::new(file)),
    };

    let mut lines = reader.lines();
    let mut sequences = Vec::new();

    while let Some(header_line) = lines.next() {
        let header = header_line?;
        if !header.starts_with('@') {
            anyhow::bail!("Expected '@' at start of FASTQ record");
        }

        let id = header[1..].to_string();

        let sequence_line = lines.next()
            .ok_or_else(|| anyhow::anyhow!("Unexpected end of file"))?;
        let sequence = sequence_line?.into_bytes();

        let _plus_line = lines.next()
            .ok_or_else(|| anyhow::anyhow!("Unexpected end of file"))?;

        let quality_line = lines.next()
            .ok_or_else(|| anyhow::anyhow!("Unexpected end of file"))?;
        let quality = quality_line?.into_bytes();

        sequences.push(SequenceRecord {
            id,
            sequence,
            quality: Some(quality),
        });
    }

    Ok(sequences)
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

fn write_results_csv(results: &[BenchmarkResult], path: &str) -> Result<()> {
    // Create directory if needed
    if let Some(parent) = Path::new(path).parent() {
        std::fs::create_dir_all(parent)?;
    }

    let mut file = File::create(path)?;
    use std::io::Write;

    // Write header
    writeln!(file, "operation,scale,compression,config,\
                    memory_median,memory_mean,\
                    file_total_median,file_total_mean,\
                    file_load_median,file_load_mean,\
                    io_overhead_pct,amdahl_max_speedup")?;

    // Write data
    for r in results {
        writeln!(file, "{},{},{},{},{:.6},{:.6},{:.6},{:.6},{:.6},{:.6},{:.2},{:.2}",
                 r.operation, r.scale, r.compression, r.config_name,
                 r.memory_median, r.memory_mean,
                 r.file_total_median, r.file_total_mean,
                 r.file_load_median, r.file_load_mean,
                 r.io_overhead_pct, r.amdahl_max_speedup)?;
    }

    println!("✅ Results written to: {}", path);

    Ok(())
}
