//! Phase 1 (GPU): Reverse Complement GPU Pilot
//!
//! Tests GPU (Metal) performance for reverse complement operation across batch sizes.
//!
//! **Hypothesis**: Reverse complement is more complex than base counting (~10-15 ops/byte vs ~6),
//! so it may show GPU benefit at lower batch sizes than base counting (which showed no benefit).
//!
//! **Expected**:
//! - Higher complexity operations should favor GPU more
//! - May see cliff threshold at 100K-500K sequences
//! - GPU kernel does more work per byte, better amortizes overhead
//!
//! Run in release mode with GPU feature:
//! ```bash
//! cargo run --release --features gpu -p asbb-cli --bin asbb-pilot-gpu-revcomp
//! ```

use anyhow::{Context, Result};
use asbb_core::{PrimitiveOperation, SequenceRecord};
use asbb_ops::reverse_complement::ReverseComplement;
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
        expected_pattern: "Moderate overhead (maybe 10× slower?)",
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
        expected_pattern: "POTENTIAL CLIFF (more complex than base counting)",
    },
    GpuScale {
        name: "Huge",
        path: "datasets/vlarge_1m_150bp.fq",
        num_sequences: 500_000,
        expected_pattern: "Post-cliff benefit? (if cliff exists)",
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
    println!("║     Phase 1 (GPU): Reverse Complement Complexity Test            ║");
    println!("║     Testing Operation Complexity Hypothesis                       ║");
    println!("╚════════════════════════════════════════════════════════════════════╝");
    println!();

    println!("🔬 Operation: Reverse Complement (transform operation)");
    println!("🎯 Goal: Test if higher complexity shows GPU benefit");
    println!("📊 Hypothesis: More complex ops favor GPU at lower batch sizes");
    println!("⚡ Hardware: M4 MacBook Pro GPU (10-core, unified memory)");
    println!();

    println!("📝 Comparison to Base Counting:");
    println!("   Base Counting:       ~6 ops/byte (simple)");
    println!("   Reverse Complement: ~10-15 ops/byte (complex)");
    println!();
    println!("   Base Counting Result: GPU NEVER beat CPU NEON (1.3-30,630× slower)");
    println!("   Expected for RevComp: GPU may show cliff at 100K-500K sequences");
    println!();

    test_reverse_complement_gpu()?;

    println!();
    println!("═══════════════════════════════════════════════════════════════");
    println!("✅ Reverse Complement GPU Pilot Complete");
    println!("═══════════════════════════════════════════════════════════════");
    println!();
    println!("Next steps:");
    println!("1. Compare to base counting results");
    println!("2. Analyze if complexity affects GPU benefit");
    println!("3. Identify complexity threshold for GPU effectiveness");
    println!("4. Document findings and update GPU decision rules");

    Ok(())
}

fn test_reverse_complement_gpu() -> Result<()> {
    let op = ReverseComplement::new();

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
                println!("    This is DIFFERENT from base counting (no cliff found)!");
            } else if gpu_speedup_vs_neon > 1.1 {
                println!("✨ GPU WIN! Speedup region begins here");
                println!("    Complexity hypothesis VALIDATED!");
            }

            // Basic validation - check first sequence
            if !gpu_result.is_empty() {
                // Extract sequences from OperationOutput
                let cpu_sequences = match cpu_naive_result {
                    asbb_core::OperationOutput::Records(ref seqs) => seqs,
                    _ => {
                        println!("⚠️  Unexpected output type from CPU");
                        continue;
                    }
                };

                if !cpu_sequences.is_empty() {
                    let cpu_seq = &cpu_sequences[0].sequence;
                    let gpu_seq = &gpu_result[0].sequence;
                    if cpu_seq == gpu_seq {
                        println!("✅ Validation: GPU output matches CPU (first sequence checked)");
                    } else {
                        println!("❌ WARNING: GPU output differs from CPU!");
                        println!("    CPU length: {}, GPU length: {}", cpu_seq.len(), gpu_seq.len());
                    }
                }
            }
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
