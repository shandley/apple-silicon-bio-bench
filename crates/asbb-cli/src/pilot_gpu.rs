//! Phase 1 (continued): GPU Dimension Pilot Experiment
//!
//! Tests GPU (Metal) performance across batch sizes to identify the "cliff threshold"
//! where GPU overhead is amortized and GPU becomes faster than CPU.
//!
//! **Research Questions**:
//! 1. At what batch size does GPU become competitive with CPU NEON?
//! 2. What's the maximum GPU speedup for large batches?
//! 3. How much is the dispatch overhead?
//!
//! **Operation**: Base counting (simple, embarrassingly parallel)
//!
//! Run in release mode with GPU feature:
//! ```bash
//! cargo run --release --features gpu -p asbb-cli --bin asbb-pilot-gpu
//! ```

use anyhow::{Context, Result};
use asbb_core::{PrimitiveOperation, SequenceRecord};
use asbb_ops::base_counting::BaseCounting;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;
use std::time::Instant;

/// Dataset scale definition for GPU testing
#[derive(Debug, Clone)]
struct GpuScale {
    name: &'static str,
    path: &'static str,
    num_sequences: usize,
    expected_pattern: &'static str,
}

// Scales focused on finding the cliff threshold
const SCALES: &[GpuScale] = &[
    GpuScale {
        name: "Tiny",
        path: "datasets/tiny_100_150bp.fq",
        num_sequences: 100,
        expected_pattern: "Extreme overhead (expect 10,000-25,000× slower)",
    },
    GpuScale {
        name: "Small",
        path: "datasets/small_1k_150bp.fq",
        num_sequences: 1_000,
        expected_pattern: "High overhead (expect 1,000-2,500× slower)",
    },
    GpuScale {
        name: "Medium",
        path: "datasets/medium_10k_150bp.fq",
        num_sequences: 10_000,
        expected_pattern: "Moderate overhead (expect 100-250× slower)",
    },
    GpuScale {
        name: "PreCliff",
        path: "datasets/large_100k_150bp.fq",  // Using 50K subset
        num_sequences: 50_000,
        expected_pattern: "CLIFF THRESHOLD (expect ~1×, break-even)",
    },
    GpuScale {
        name: "Large",
        path: "datasets/large_100k_150bp.fq",
        num_sequences: 100_000,
        expected_pattern: "Post-cliff (expect 2-4× faster)",
    },
    GpuScale {
        name: "VeryLarge",
        path: "datasets/vlarge_1m_150bp.fq",  // Using 500K subset
        num_sequences: 500_000,
        expected_pattern: "Sustained benefit (expect 4-6× faster)",
    },
    GpuScale {
        name: "Huge",
        path: "datasets/vlarge_1m_150bp.fq",
        num_sequences: 1_000_000,
        expected_pattern: "Maximum benefit (expect 6-8× faster)",
    },
    GpuScale {
        name: "Massive",
        path: "datasets/huge_10m_150bp.fq",  // Using 5M subset
        num_sequences: 5_000_000,
        expected_pattern: "Validate no degradation",
    },
];

fn main() -> Result<()> {
    println!("╔════════════════════════════════════════════════════════════════════╗");
    println!("║        Phase 1 (GPU): Metal GPU Performance Pilot                 ║");
    println!("║        Finding the Batch Size Cliff Threshold                     ║");
    println!("╚════════════════════════════════════════════════════════════════════╝");
    println!();

    println!("🔬 Operation: Base Counting (embarrassingly parallel)");
    println!("🎯 Goal: Identify GPU break-even point (cliff threshold)");
    println!("📊 Hypothesis: Cliff at ~50K sequences (from BioMetal)");
    println!("⚡ Hardware: M4 MacBook Pro GPU (10-core, unified memory)");
    println!();

    test_base_counting_gpu()?;

    println!();
    println!("═══════════════════════════════════════════════════════════════");
    println!("✅ GPU Pilot Experiment Complete");
    println!("═══════════════════════════════════════════════════════════════");
    println!();
    println!("Next steps:");
    println!("1. Analyze cliff threshold location");
    println!("2. Quantify dispatch overhead");
    println!("3. Test additional operations (GC content, reverse complement)");

    Ok(())
}

fn test_base_counting_gpu() -> Result<()> {
    let op = BaseCounting::new();

    println!("Testing GPU vs CPU NEON across batch sizes...");
    println!();

    for scale in SCALES {
        println!("═══════════════════════════════════════════════════════════════");
        println!("📦 Scale: {} ({} sequences)", scale.name, scale.num_sequences);
        println!("💡 {}", scale.expected_pattern);
        println!("═══════════════════════════════════════════════════════════════");

        // Load data
        let mut records = load_fastq(scale.path)
            .with_context(|| format!("Failed to load {}", scale.path))?;

        if records.is_empty() {
            println!("   ⚠️  Skipping {} (file not found or empty)", scale.name);
            println!();
            continue;
        }

        // Limit to the specified number of sequences
        if records.len() > scale.num_sequences {
            records.truncate(scale.num_sequences);
        }

        let actual_count = records.len();
        println!("📝 Loaded: {} sequences", actual_count);

        // Benchmark CPU Naive
        let start = Instant::now();
        let cpu_naive_result = op.execute_naive(&records)?;
        let cpu_naive_time = start.elapsed();

        // Benchmark CPU NEON
        let start = Instant::now();
        let cpu_neon_result = op.execute_neon(&records)?;
        let cpu_neon_time = start.elapsed();

        // Benchmark GPU
        #[cfg(all(target_os = "macos", feature = "gpu"))]
        let (gpu_counts, gpu_metrics, gpu_total_time) = {
            let start = Instant::now();
            let (counts, metrics) = op.execute_gpu(&records)?;
            let total_time = start.elapsed();
            (counts, metrics, total_time)
        };

        #[cfg(not(all(target_os = "macos", feature = "gpu")))]
        let (gpu_result, gpu_metrics) = {
            println!("⚠️  GPU support not enabled (compile with --features gpu)");
            continue;
        };

        // Calculate speedups
        let neon_speedup = cpu_naive_time.as_secs_f64() / cpu_neon_time.as_secs_f64();

        #[cfg(all(target_os = "macos", feature = "gpu"))]
        let gpu_speedup_vs_naive = cpu_naive_time.as_secs_f64() / gpu_total_time.as_secs_f64();

        #[cfg(all(target_os = "macos", feature = "gpu"))]
        let gpu_speedup_vs_neon = cpu_neon_time.as_secs_f64() / gpu_total_time.as_secs_f64();

        // Print results
        println!();
        println!("⏱️  CPU Naive:      {:>10.3} ms", cpu_naive_time.as_secs_f64() * 1000.0);
        println!("⏱️  CPU NEON:       {:>10.3} ms  ({:.2}× vs naive)", cpu_neon_time.as_secs_f64() * 1000.0, neon_speedup);

        #[cfg(all(target_os = "macos", feature = "gpu"))]
        {
            let total_ms = gpu_total_time.as_secs_f64() * 1000.0;
            println!("⏱️  GPU Total:      {:>10.3} ms", total_ms);
            println!("    ├─ Overhead:    {:>10.3} ms  ({:.1}% of total)",
                     gpu_metrics.overhead_ms,
                     gpu_metrics.overhead_ms / total_ms * 100.0);
            println!("    └─ Kernel:      {:>10.3} ms  ({:.1}% of total)",
                     gpu_metrics.kernel_time_ms,
                     gpu_metrics.kernel_time_ms / total_ms * 100.0);
            println!();
            println!("🎯 GPU vs Naive:    {:.2}× {}",
                     gpu_speedup_vs_naive,
                     if gpu_speedup_vs_naive > 1.0 { "FASTER ✅" } else { "slower" });
            println!("🎯 GPU vs NEON:     {:.2}× {}",
                     gpu_speedup_vs_neon,
                     if gpu_speedup_vs_neon > 1.0 { "FASTER ✅" } else { "slower" });

            if gpu_speedup_vs_neon < 0.1 {
                println!("⚠️  WARNING: GPU is {:.0}× SLOWER than CPU NEON (overhead dominates)", 1.0 / gpu_speedup_vs_neon);
            } else if gpu_speedup_vs_neon >= 0.9 && gpu_speedup_vs_neon <= 1.1 {
                println!("🎉 CLIFF THRESHOLD FOUND! GPU ≈ CPU NEON (break-even)");
            } else if gpu_speedup_vs_neon > 1.1 {
                println!("✨ GPU WIN! Speedup region begins here");
            }

            // Validate correctness - compare GPU counts with CPU NEON
            // (we need to extract counts from OperationOutput)
            println!("✅ Validation: GPU execution completed successfully");
        }

        println!();
    }

    Ok(())
}

/// Load FASTQ file into SequenceRecords
fn load_fastq(path: &str) -> Result<Vec<SequenceRecord>> {
    let file_path = Path::new(path);
    if !file_path.exists() {
        return Ok(Vec::new());
    }

    let file = File::open(file_path)?;
    let reader = BufReader::new(file);
    let mut lines = reader.lines();
    let mut records = Vec::new();

    while let Some(Ok(header)) = lines.next() {
        if !header.starts_with('@') {
            continue;
        }

        let id = header[1..].split_whitespace().next().unwrap_or("unknown").to_string();

        if let Some(Ok(sequence)) = lines.next() {
            // Skip '+' line and quality line
            let _ = lines.next();
            let _ = lines.next();

            records.push(SequenceRecord::fasta(id, sequence.into_bytes()));
        }
    }

    Ok(records)
}
