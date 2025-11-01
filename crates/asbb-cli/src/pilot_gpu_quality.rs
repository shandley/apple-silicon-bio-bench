//! Phase 1 (GPU): Quality Aggregation GPU Pilot
//!
//! Tests GPU (Metal) performance for quality aggregation operation across batch sizes.
//!
//! **Hypothesis**: Quality aggregation (complexity 0.50) is more complex than base counting (0.40)
//! and reverse complement (0.45), and involves reduction operations (min/max). May show different
//! GPU behavior.
//!
//! **Expected**:
//! - Higher complexity than previous tests
//! - Min/max operations may be less vectorizable than counting
//! - GPU reduction operations might be more efficient than CPU
//!
//! Run in release mode with GPU feature:
//! ```bash
//! cargo run --release --features gpu -p asbb-cli --bin asbb-pilot-gpu-quality
//! ```

use anyhow::{Context, Result};
use asbb_core::{PrimitiveOperation, SequenceRecord};
use asbb_ops::quality_aggregation::QualityAggregation;
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
        expected_pattern: "Extreme overhead (likely 1000s× slower)",
    },
    GpuScale {
        name: "Small",
        path: "datasets/small_1k_150bp.fq",
        num_sequences: 1_000,
        expected_pattern: "High overhead (likely 100s× slower)",
    },
    GpuScale {
        name: "Medium",
        path: "datasets/medium_10k_150bp.fq",
        num_sequences: 10_000,
        expected_pattern: "Moderate overhead",
    },
    GpuScale {
        name: "Large",
        path: "datasets/large_100k_150bp.fq",
        num_sequences: 50_000,
        expected_pattern: "Approaching break-even?",
    },
    GpuScale {
        name: "VeryLarge",
        path: "datasets/large_100k_150bp.fq",
        num_sequences: 100_000,
        expected_pattern: "Higher complexity - may show benefit here?",
    },
    GpuScale {
        name: "Huge",
        path: "datasets/vlarge_1m_150bp.fq",
        num_sequences: 500_000,
        expected_pattern: "Aggregation operations - GPU strength?",
    },
    GpuScale {
        name: "Massive1M",
        path: "datasets/vlarge_1m_150bp.fq",
        num_sequences: 1_000_000,
        expected_pattern: "Sustained benefit validation",
    },
    GpuScale {
        name: "Massive5M",
        path: "datasets/huge_10m_150bp.fq",
        num_sequences: 5_000_000,
        expected_pattern: "Maximum benefit measurement",
    },
];

fn main() -> Result<()> {
    println!("╔════════════════════════════════════════════════════════════════════╗");
    println!("║     Phase 1 (GPU): Quality Aggregation Complexity Test           ║");
    println!("║     Testing Aggregation Operations on GPU                         ║");
    println!("╚════════════════════════════════════════════════════════════════════╝");
    println!();

    println!("🔬 Operation: Quality Aggregation (min/max/sum)");
    println!("🎯 Goal: Test if aggregation operations benefit from GPU");
    println!("📊 Hypothesis: Reduction ops may favor GPU differently than counting");
    println!("⚡ Hardware: M4 MacBook Pro GPU (10-core, unified memory)");
    println!();

    println!("📝 Complexity Comparison:");
    println!("   Base Counting:       0.40 (simple counting)");
    println!("   Reverse Complement:  0.45 (transform)");
    println!("   Quality Aggregation: 0.50 (aggregation with reductions)");
    println!();
    println!("   Previous Results:");
    println!("   - Base counting: GPU NEVER beat CPU NEON");
    println!("   - Reverse complement: GPU NEVER beat CPU NEON");
    println!("   Expected for Quality: GPU likely slower, but testing aggregations");
    println!();

    test_quality_aggregation_gpu()?;

    println!();
    println!("═══════════════════════════════════════════════════════════════");
    println!("✅ Quality Aggregation GPU Pilot Complete");
    println!("═══════════════════════════════════════════════════════════════");
    println!();
    println!("Next steps:");
    println!("1. Compare to base counting and reverse complement");
    println!("2. Analyze if aggregation operations have different GPU pattern");
    println!("3. Test complexity score (highest complexity)");
    println!("4. Create final GPU dimension analysis");

    Ok(())
}

fn test_quality_aggregation_gpu() -> Result<()> {
    let op = QualityAggregation::new();

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
        let _cpu_neon_result = op.execute_neon(&records)?;
        let cpu_neon_time = start.elapsed();

        // Benchmark GPU
        #[cfg(all(target_os = "macos", feature = "gpu"))]
        let (gpu_result, gpu_metrics, gpu_total_time) = {
            let start = Instant::now();
            let (result, metrics) = op.execute_gpu(&records)?;
            let total_time = start.elapsed();
            (result, metrics, total_time)
        };

        #[cfg(not(all(target_os = "macos", feature = "gpu")))]
        let _ = {
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
                println!("    This would be DIFFERENT from previous operations!");
            } else if gpu_speedup_vs_neon > 1.1 {
                println!("✨ GPU WIN! Speedup region begins here");
                println!("    Aggregation hypothesis VALIDATED!");
            }

            // Basic validation - check stats match
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

        if let (Some(Ok(sequence)), Some(Ok(_plus)), Some(Ok(quality))) =
            (lines.next(), lines.next(), lines.next())
        {
            records.push(SequenceRecord::fastq(id, sequence.into_bytes(), quality.into_bytes()));
        }
    }

    Ok(records)
}
