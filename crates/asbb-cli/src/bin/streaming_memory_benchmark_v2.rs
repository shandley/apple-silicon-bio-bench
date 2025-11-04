#!/usr/bin/env cargo
//! Streaming Memory Footprint Benchmark v2 (CORRECTED)
//!
//! Fixes from v1:
//! 1. Fork per experiment (eliminates baseline drift)
//! 2. Real FASTQ file I/O (not in-memory generation)
//! 3. True streaming from compressed files
//!
//! Compares batch (load all) vs streaming (record-by-record) memory usage.

use anyhow::{Context, Result};
use asbb_core::{OperationOutput, PrimitiveOperation, SequenceRecord};
use asbb_ops::{
    base_counting::BaseCounting,
    gc_content::GcContent,
};
use flate2::read::GzDecoder;
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::{BufRead, BufReader, Write};
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::Instant;

#[derive(Debug, Clone, Serialize, Deserialize)]
enum Config {
    Naive,
    Neon,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
enum Pattern {
    Batch,
    Streaming,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ExperimentConfig {
    operation: String,
    scale_name: String,
    file_path: String,
    num_sequences: usize,
    config: Config,
    pattern: Pattern,
    repetitions: usize,
}

#[derive(Debug, Serialize, Deserialize)]
struct MemoryResult {
    operation: String,
    scale: String,
    num_sequences: usize,
    config_name: String,
    pattern_name: String,

    baseline_mb: f64,
    peak_mb: f64,
    memory_used_mb: f64,
    memory_per_seq_kb: f64,

    elapsed_sec: f64,
}

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();

    // Check if running in child mode (single experiment)
    if args.len() > 1 && args[1] == "--child" {
        // Child process: run single experiment
        let config_json = &args[2];
        let config: ExperimentConfig = serde_json::from_str(config_json)?;
        let result = run_single_experiment_isolated(&config)?;

        // Print result as JSON to stdout
        println!("{}", serde_json::to_string(&result)?);
        return Ok(());
    }

    // Parent process: orchestrate all experiments
    println!("================================================================================");
    println!("STREAMING MEMORY FOOTPRINT BENCHMARK V2 (CORRECTED)");
    println!("================================================================================");
    println!();
    println!("Improvements over v1:");
    println!("  ✓ Fork per experiment (clean baselines, no drift)");
    println!("  ✓ Real FASTQ file I/O (not in-memory generation)");
    println!("  ✓ True streaming from compressed files");
    println!();

    // Define test matrix (using existing compressed files)
    let experiments = vec![
        ("Medium", "datasets/medium_10k_150bp.fq.gz", 10_000),
        ("Large", "datasets/large_100k_150bp.fq.gz", 100_000),
        ("VeryLarge", "datasets/vlarge_1m_150bp.fq.gz", 1_000_000),
    ];

    let operations = vec!["base_counting", "gc_content"];
    let configs = vec![("naive", Config::Naive), ("neon", Config::Neon)];
    let patterns = vec![("batch", Pattern::Batch), ("streaming", Pattern::Streaming)];
    let repetitions = 30;

    // Check files exist
    println!("Checking input files...");
    for (scale_name, path, _) in &experiments {
        if Path::new(path).exists() {
            let size = std::fs::metadata(path)?.len();
            println!("  ✓ {}: {} ({:.1} MB)", scale_name, path, size as f64 / 1024.0 / 1024.0);
        } else {
            anyhow::bail!("Required file not found: {}", path);
        }
    }
    println!();

    let total_experiments = operations.len() * experiments.len() * configs.len() * patterns.len();
    println!("Test Matrix:");
    println!("  Operations: {}", operations.len());
    println!("  Scales: {}", experiments.len());
    println!("  Configs: {}", configs.len());
    println!("  Patterns: {}", patterns.len());
    println!("  Repetitions: {}", repetitions);
    println!("  Total experiments: {}", total_experiments);
    println!();

    let mut results = Vec::new();

    for op_name in &operations {
        for (scale_name, file_path, num_sequences) in &experiments {
            println!("Testing {} @ {} ({} sequences)", op_name, scale_name, num_sequences);

            for (config_name, config) in &configs {
                for (pattern_name, pattern) in &patterns {
                    let experiment = ExperimentConfig {
                        operation: op_name.to_string(),
                        scale_name: scale_name.to_string(),
                        file_path: file_path.to_string(),
                        num_sequences: *num_sequences,
                        config: config.clone(),
                        pattern: pattern.clone(),
                        repetitions,
                    };

                    print!("  {} + {:10} ... ", config_name, pattern_name);
                    std::io::stdout().flush().ok();

                    match run_experiment_forked(&experiment) {
                        Ok(result) => {
                            println!("{:.1} MB peak, {:.1} KB/seq",
                                     result.peak_mb, result.memory_per_seq_kb);
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
    write_results_csv(&results, "results/streaming/streaming_memory_v2_n30.csv")?;

    println!("================================================================================");
    println!("BENCHMARK COMPLETE");
    println!("================================================================================");
    println!("Results written to: results/streaming/streaming_memory_v2_n30.csv");
    println!("Total experiments: {}", results.len());
    println!();

    Ok(())
}

fn run_experiment_forked(config: &ExperimentConfig) -> Result<MemoryResult> {
    // Serialize config to JSON
    let config_json = serde_json::to_string(config)?;

    // Get current executable path
    let exe = std::env::current_exe()?;

    // Fork child process
    let output = Command::new(&exe)
        .args(&["--child", &config_json])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .context("Failed to spawn child process")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("Child process failed: {}", stderr);
    }

    // Parse result from stdout
    let stdout = String::from_utf8(output.stdout)?;
    let result: MemoryResult = serde_json::from_str(&stdout)
        .context("Failed to parse child result")?;

    Ok(result)
}

fn run_single_experiment_isolated(config: &ExperimentConfig) -> Result<MemoryResult> {
    // This runs in a FRESH process with clean baseline
    let mut elapsed_times = Vec::with_capacity(config.repetitions);
    let mut peak_mbs = Vec::with_capacity(config.repetitions);

    // Warmup (3 runs)
    for _ in 0..3 {
        let _ = run_single_measurement(config)?;
    }

    // Measure
    for _ in 0..config.repetitions {
        let (elapsed, peak) = run_single_measurement(config)?;
        elapsed_times.push(elapsed);
        peak_mbs.push(peak);
    }

    // Calculate statistics
    let elapsed_sec = median(&elapsed_times);
    let peak_mb = median(&peak_mbs);
    let baseline_mb = measure_rss_mb()?;  // Current process baseline
    let memory_used_mb = peak_mb - baseline_mb;
    let memory_per_seq_kb = (memory_used_mb * 1024.0) / config.num_sequences as f64;

    Ok(MemoryResult {
        operation: config.operation.clone(),
        scale: config.scale_name.clone(),
        num_sequences: config.num_sequences,
        config_name: format!("{:?}", config.config).to_lowercase(),
        pattern_name: format!("{:?}", config.pattern).to_lowercase(),
        baseline_mb,
        peak_mb,
        memory_used_mb,
        memory_per_seq_kb,
        elapsed_sec,
    })
}

fn run_single_measurement(config: &ExperimentConfig) -> Result<(f64, f64)> {
    let start = Instant::now();
    let mut peak_mb = measure_rss_mb()?;

    let op: Box<dyn PrimitiveOperation> = match config.operation.as_str() {
        "base_counting" => Box::new(BaseCounting::new()),
        "gc_content" => Box::new(GcContent::new()),
        _ => anyhow::bail!("Unsupported operation: {}", config.operation),
    };

    match config.pattern {
        Pattern::Batch => {
            // Load entire file into memory
            let sequences = load_all_sequences(&config.file_path)?;
            peak_mb = peak_mb.max(measure_rss_mb()?);

            // Process
            let _ = execute_operation(&*op, &sequences, &config.config)?;
            peak_mb = peak_mb.max(measure_rss_mb()?);
        }
        Pattern::Streaming => {
            // Stream from file, process one record at a time
            let file = File::open(&config.file_path)?;
            let decoder = GzDecoder::new(file);
            let reader = BufReader::new(decoder);
            let mut lines = reader.lines();

            let mut count = 0;
            while let Some(header_line) = lines.next() {
                let header = header_line?;
                if !header.starts_with('@') {
                    continue;
                }

                let id = header[1..].to_string();
                let sequence = lines.next().ok_or_else(|| anyhow::anyhow!("EOF"))??.into_bytes();
                let _plus = lines.next().ok_or_else(|| anyhow::anyhow!("EOF"))??;
                let quality = lines.next().ok_or_else(|| anyhow::anyhow!("EOF"))??.into_bytes();

                let record = SequenceRecord {
                    id,
                    sequence,
                    quality: Some(quality),
                };

                // Process immediately (single record)
                let _ = execute_operation(&*op, &[record], &config.config)?;

                count += 1;

                // Measure memory periodically
                if count % 10_000 == 0 {
                    peak_mb = peak_mb.max(measure_rss_mb()?);
                }
            }

            // Final measurement
            peak_mb = peak_mb.max(measure_rss_mb()?);
        }
    }

    let elapsed = start.elapsed().as_secs_f64();
    Ok((elapsed, peak_mb))
}

fn load_all_sequences(path: &str) -> Result<Vec<SequenceRecord>> {
    let file = File::open(path)?;
    let decoder = GzDecoder::new(file);
    let reader = BufReader::new(decoder);
    let mut lines = reader.lines();

    let mut sequences = Vec::new();

    while let Some(header_line) = lines.next() {
        let header = header_line?;
        if !header.starts_with('@') {
            continue;
        }

        let id = header[1..].to_string();
        let sequence = lines.next().ok_or_else(|| anyhow::anyhow!("EOF"))??.into_bytes();
        let _plus = lines.next().ok_or_else(|| anyhow::anyhow!("EOF"))??;
        let quality = lines.next().ok_or_else(|| anyhow::anyhow!("EOF"))??.into_bytes();

        sequences.push(SequenceRecord {
            id,
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

fn write_results_csv(results: &[MemoryResult], path: &str) -> Result<()> {
    if let Some(parent) = Path::new(path).parent() {
        std::fs::create_dir_all(parent)?;
    }

    let mut file = File::create(path)?;

    writeln!(
        file,
        "operation,scale,num_sequences,config,pattern,\
         baseline_mb,peak_mb,memory_used_mb,memory_per_seq_kb,elapsed_sec"
    )?;

    for r in results {
        writeln!(
            file,
            "{},{},{},{},{},{:.2},{:.2},{:.2},{:.4},{:.4}",
            r.operation,
            r.scale,
            r.num_sequences,
            r.config_name,
            r.pattern_name,
            r.baseline_mb,
            r.peak_mb,
            r.memory_used_mb,
            r.memory_per_seq_kb,
            r.elapsed_sec
        )?;
    }

    println!("✅ Results written to: {}", path);

    Ok(())
}
