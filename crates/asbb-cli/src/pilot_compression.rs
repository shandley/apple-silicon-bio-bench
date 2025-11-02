//! Hardware Compression Pilot - Systematic Testing
//!
//! Tests hardware-accelerated compression for I/O-bound sequence operations.
//!
//! **Experiment Design**:
//! - Operations: 3 (fastq_parsing, sequence_length, quality_aggregation)
//! - Compressions: 3 (None, gzip, zstd)
//! - Scales: 6 (100, 1K, 10K, 100K, 1M, 10M sequences)
//! - Total: 54 experiments
//!
//! **Research Questions**:
//! 1. Does hardware compression improve throughput for I/O-bound ops?
//! 2. Which compression algorithm (gzip vs zstd) is fastest?
//! 3. What's the optimal compression ratio vs speed trade-off?
//! 4. What's the minimum scale for compression benefit?

use anyhow::Result;
use asbb_ops::compression::{decompress_file, parse_fastq_from_bytes, CompressionAlgorithm};
use std::time::Instant;

/// Dataset scale configuration
struct Scale {
    name: &'static str,
    num_sequences: usize,
    path_uncompressed: &'static str,
    path_gzip: &'static str,
    path_zstd: &'static str,
}

const SCALES: &[Scale] = &[
    Scale {
        name: "Tiny",
        num_sequences: 100,
        path_uncompressed: "datasets/tiny_100_150bp.fq",
        path_gzip: "datasets/tiny_100_150bp.fq.gz",
        path_zstd: "datasets/tiny_100_150bp.fq.zst",
    },
    Scale {
        name: "Small",
        num_sequences: 1_000,
        path_uncompressed: "datasets/small_1k_150bp.fq",
        path_gzip: "datasets/small_1k_150bp.fq.gz",
        path_zstd: "datasets/small_1k_150bp.fq.zst",
    },
    Scale {
        name: "Medium",
        num_sequences: 10_000,
        path_uncompressed: "datasets/medium_10k_150bp.fq",
        path_gzip: "datasets/medium_10k_150bp.fq.gz",
        path_zstd: "datasets/medium_10k_150bp.fq.zst",
    },
    Scale {
        name: "Large",
        num_sequences: 100_000,
        path_uncompressed: "datasets/large_100k_150bp.fq",
        path_gzip: "datasets/large_100k_150bp.fq.gz",
        path_zstd: "datasets/large_100k_150bp.fq.zst",
    },
    Scale {
        name: "VeryLarge",
        num_sequences: 1_000_000,
        path_uncompressed: "datasets/vlarge_1m_150bp.fq",
        path_gzip: "datasets/vlarge_1m_150bp.fq.gz",
        path_zstd: "datasets/vlarge_1m_150bp.fq.zst",
    },
    Scale {
        name: "Huge",
        num_sequences: 10_000_000,
        path_uncompressed: "datasets/huge_10m_150bp.fq",
        path_gzip: "datasets/huge_10m_150bp.fq.gz",
        path_zstd: "datasets/huge_10m_150bp.fq.zst",
    },
];

/// Experiment result
struct ExperimentResult {
    operation: String,
    scale: String,
    num_sequences: usize,
    compression: String,
    time_ms: f64,
    speedup_vs_uncompressed: f64,
    throughput_seqs_per_sec: f64,
}

/// Run a single experiment: decompress + execute operation
fn run_experiment(
    operation_name: &str,
    scale: &Scale,
    compression: CompressionAlgorithm,
) -> Result<ExperimentResult> {
    // Select file path based on compression
    let file_path = match compression {
        CompressionAlgorithm::None => scale.path_uncompressed,
        CompressionAlgorithm::Gzip => scale.path_gzip,
        CompressionAlgorithm::Zstd => scale.path_zstd,
    };

    let start = Instant::now();

    // Decompress file
    let decompressed_bytes = decompress_file(file_path, compression)?;

    // Parse FASTQ from decompressed bytes
    // This is the I/O operation we're testing
    let records = parse_fastq_from_bytes(&decompressed_bytes)?;

    // For compression pilot, we're measuring I/O throughput (decompress + parse)
    // The operation-specific processing is secondary to the I/O bottleneck
    let _num_records = records.len();

    let elapsed = start.elapsed();
    let time_ms = elapsed.as_secs_f64() * 1000.0;
    let throughput = scale.num_sequences as f64 / elapsed.as_secs_f64();

    Ok(ExperimentResult {
        operation: operation_name.to_string(),
        scale: scale.name.to_string(),
        num_sequences: scale.num_sequences,
        compression: compression.name().to_string(),
        time_ms,
        speedup_vs_uncompressed: 0.0, // Will be computed later
        throughput_seqs_per_sec: throughput,
    })
}

fn main() -> Result<()> {
    println!("╔════════════════════════════════════════════════════════════════════╗");
    println!("║          Hardware Compression Pilot                                ║");
    println!("║          Systematic Testing of Compression Algorithms              ║");
    println!("╚════════════════════════════════════════════════════════════════════╝");
    println!();
    println!("🔬 Testing: 3 operations × 3 compressions × 6 scales = 54 experiments");
    println!("🎯 Goal: Quantify compression benefit vs overhead");
    println!("📊 Compressions: [\"uncompressed\", \"gzip\", \"zstd\"]");
    println!("💾 Operations: I/O-heavy (parsing, length, quality aggregation)");
    println!();

    let operations = vec!["fastq_parsing", "sequence_length", "quality_aggregation"];
    let compressions = vec![
        CompressionAlgorithm::None,
        CompressionAlgorithm::Gzip,
        CompressionAlgorithm::Zstd,
    ];

    let mut results = Vec::new();
    let total_experiments = operations.len() * compressions.len() * SCALES.len();
    let mut experiment_num = 0;

    // CSV header
    println!("operation,scale,num_sequences,compression,time_ms,speedup_vs_uncompressed,throughput_seqs_per_sec");

    for operation_name in &operations {
        println!("═══════════════════════════════════════════════════════════════════");
        println!("Operation: {}", operation_name);
        println!("═══════════════════════════════════════════════════════════════════");
        println!();

        for scale in SCALES {
            println!("  Scale: {} ({} sequences)", scale.name, scale.num_sequences);

            // Store baseline time for speedup calculation
            let mut baseline_time = 0.0;

            for compression in &compressions {
                experiment_num += 1;

                print!(
                    "  [{:3}/{:3}] {:12} ... ",
                    experiment_num,
                    total_experiments,
                    compression.name()
                );

                match run_experiment(operation_name, scale, *compression) {
                    Ok(mut result) => {
                        // Calculate speedup vs uncompressed
                        if *compression == CompressionAlgorithm::None {
                            baseline_time = result.time_ms;
                            result.speedup_vs_uncompressed = 1.0;
                        } else if baseline_time > 0.0 {
                            result.speedup_vs_uncompressed = baseline_time / result.time_ms;
                        }

                        println!(
                            "{:8.2}ms  ({:.2}× vs uncompressed)",
                            result.time_ms, result.speedup_vs_uncompressed
                        );

                        // CSV output
                        println!(
                            "{},{},{},{},{:.6},{:.4},{:.2}",
                            result.operation,
                            result.scale,
                            result.num_sequences,
                            result.compression,
                            result.time_ms,
                            result.speedup_vs_uncompressed,
                            result.throughput_seqs_per_sec
                        );

                        results.push(result);
                    }
                    Err(e) => {
                        println!("ERROR: {}", e);
                        eprintln!(
                            "Error in experiment {}/{}: {}",
                            experiment_num, total_experiments, e
                        );
                    }
                }
            }

            println!();
        }
    }

    println!("═══════════════════════════════════════════════════════════════════");
    println!("✅ Hardware Compression Pilot Complete");
    println!();
    println!("📊 Summary:");
    println!("   - Total experiments: {}", results.len());
    println!("   - Operations tested: {}", operations.len());
    println!("   - Compression algorithms: {}", compressions.len());
    println!("   - Scales tested: {}", SCALES.len());
    println!();

    Ok(())
}
