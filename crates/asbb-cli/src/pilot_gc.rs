//! GC Content Multi-Scale Pilot Experiment
//!
//! Tests if base counting patterns generalize to GC content calculation.
//!
//! **Hypothesis**: GC content should show similar patterns to base counting:
//! - NEON: 16-65× speedup (scale-dependent)
//! - Parallel threshold: 1,000 sequences
//! - Combined: 40-60× at large scale
//!
//! Run in release mode:
//! ```bash
//! cargo run --release -p asbb-cli --bin asbb-pilot-gc
//! ```

use anyhow::{Context, Result};
use asbb_core::{HardwareConfig, PerformanceResult, SequenceRecord};
use asbb_explorer::benchmark_operation;
use asbb_ops::gc_content::GcContent;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

/// Dataset scale definition (reuse from base counting)
#[derive(Debug)]
struct DatasetScale {
    name: &'static str,
    path: &'static str,
    num_sequences: usize,
    expected_size_mb: f64,
}

const SCALES: &[DatasetScale] = &[
    DatasetScale {
        name: "Tiny",
        path: "datasets/tiny_100_150bp.fq",
        num_sequences: 100,
        expected_size_mb: 0.03,
    },
    DatasetScale {
        name: "Small",
        path: "datasets/small_1k_150bp.fq",
        num_sequences: 1_000,
        expected_size_mb: 0.3,
    },
    DatasetScale {
        name: "Medium",
        path: "datasets/medium_10k_150bp.fq",
        num_sequences: 10_000,
        expected_size_mb: 3.0,
    },
    DatasetScale {
        name: "Large",
        path: "datasets/large_100k_150bp.fq",
        num_sequences: 100_000,
        expected_size_mb: 30.0,
    },
    DatasetScale {
        name: "VeryLarge",
        path: "datasets/vlarge_1m_150bp.fq",
        num_sequences: 1_000_000,
        expected_size_mb: 300.0,
    },
    DatasetScale {
        name: "Huge",
        path: "datasets/huge_10m_150bp.fq",
        num_sequences: 10_000_000,
        expected_size_mb: 3000.0,
    },
];

fn main() -> Result<()> {
    println!("╔════════════════════════════════════════════════════════════════════╗");
    println!("║  GC Content Multi-Scale Pilot Experiment                         ║");
    println!("║  Validating Base Counting Patterns                               ║");
    println!("╚════════════════════════════════════════════════════════════════════╝");
    println!();

    println!("🎯 Hypothesis: GC content shows same patterns as base counting");
    println!("   - NEON: 16-65× speedup (scale-dependent)");
    println!("   - Parallel threshold: 1,000 sequences");
    println!("   - Combined: 40-60× at large scale");
    println!();

    // Create operation
    let operation = GcContent::new();

    // Benchmark parameters
    let warmup_runs = 2;
    let measured_runs = 5;

    // Results storage
    let mut all_results = Vec::new();

    // Run experiments for each scale
    for scale in SCALES {
        println!("╔════════════════════════════════════════════════════════════════════╗");
        println!("║  Scale: {} ({} sequences, ~{:.1} MB)                    ", scale.name, scale.num_sequences, scale.expected_size_mb);
        println!("╚════════════════════════════════════════════════════════════════════╝");
        println!();

        // Load data
        print!("📂 Loading data...");
        let data = load_fastq(scale.path)
            .with_context(|| format!("Failed to load {}", scale.path))?;
        println!(" loaded {} sequences", data.len());

        // Validate data size
        if data.len() != scale.num_sequences {
            eprintln!("⚠️  Warning: Expected {} sequences, got {}", scale.num_sequences, data.len());
        }

        println!();

        // Experiment 1: Naive
        print!("  1️⃣  Naive (baseline)... ");
        let naive_config = HardwareConfig::naive();
        let naive_result = benchmark_operation(&operation, &data, &naive_config, warmup_runs, measured_runs)?;
        println!("✓ {:.2} Mseqs/sec", naive_result.throughput_seqs_per_sec / 1_000_000.0);

        // Experiment 2: NEON
        #[cfg(target_arch = "aarch64")]
        let neon_result = {
            print!("  2️⃣  NEON SIMD...        ");
            let mut neon_config = HardwareConfig::naive();
            neon_config.use_neon = true;
            let result = benchmark_operation(&operation, &data, &neon_config, warmup_runs, measured_runs)?;
            println!("✓ {:.2} Mseqs/sec ({}×)",
                result.throughput_seqs_per_sec / 1_000_000.0,
                format_speedup(result.speedup_vs(&naive_result))
            );
            Some(result)
        };

        #[cfg(not(target_arch = "aarch64"))]
        let neon_result: Option<PerformanceResult> = None;

        // Experiment 3: Parallel (4 threads)
        print!("  3️⃣  Parallel (4T)...    ");
        let mut parallel_config = HardwareConfig::naive();
        parallel_config.num_threads = 4;
        let parallel_result = benchmark_operation(&operation, &data, &parallel_config, warmup_runs, measured_runs)?;
        println!("✓ {:.2} Mseqs/sec ({}×)",
            parallel_result.throughput_seqs_per_sec / 1_000_000.0,
            format_speedup(parallel_result.speedup_vs(&naive_result))
        );

        // Experiment 4: NEON + Parallel Combined
        #[cfg(target_arch = "aarch64")]
        let combined_result = {
            print!("  4️⃣  NEON + Parallel... ");
            let mut combined_config = HardwareConfig::naive();
            combined_config.use_neon = true;
            combined_config.num_threads = 4;
            let result = benchmark_operation(&operation, &data, &combined_config, warmup_runs, measured_runs)?;
            println!("✓ {:.2} Mseqs/sec ({}×)",
                result.throughput_seqs_per_sec / 1_000_000.0,
                format_speedup(result.speedup_vs(&naive_result))
            );
            Some(result)
        };

        #[cfg(not(target_arch = "aarch64"))]
        let combined_result: Option<PerformanceResult> = None;

        // Store results
        all_results.push(ScaleResults {
            scale: scale.name,
            num_sequences: scale.num_sequences,
            naive: naive_result,
            neon: neon_result,
            parallel: parallel_result,
            combined: combined_result,
        });

        println!();
    }

    // Print summary table
    print_summary_table(&all_results);

    // Compare to base counting patterns
    compare_to_base_counting(&all_results);

    println!();
    println!("╔════════════════════════════════════════════════════════════════════╗");
    println!("║  GC Content Pilot Complete                                        ║");
    println!("╚════════════════════════════════════════════════════════════════════╝");
    println!();

    Ok(())
}

/// Results for one scale
struct ScaleResults {
    scale: &'static str,
    num_sequences: usize,
    naive: PerformanceResult,
    neon: Option<PerformanceResult>,
    parallel: PerformanceResult,
    combined: Option<PerformanceResult>,
}

/// Load FASTQ file (simple parser for benchmarking)
fn load_fastq<P: AsRef<Path>>(path: P) -> Result<Vec<SequenceRecord>> {
    let file = File::open(path.as_ref())?;
    let reader = BufReader::new(file);
    let mut lines = reader.lines();
    let mut records = Vec::new();

    loop {
        // Line 1: Header
        let header = match lines.next() {
            Some(Ok(line)) => line,
            Some(Err(e)) => return Err(e.into()),
            None => break,
        };

        // Line 2: Sequence
        let sequence = lines.next()
            .ok_or_else(|| anyhow::anyhow!("Incomplete FASTQ record"))??;

        // Line 3: Plus line
        let _plus = lines.next()
            .ok_or_else(|| anyhow::anyhow!("Incomplete FASTQ record"))??;

        // Line 4: Quality
        let quality = lines.next()
            .ok_or_else(|| anyhow::anyhow!("Incomplete FASTQ record"))??;

        records.push(SequenceRecord::fastq(
            header[1..].to_string(), // Remove '@'
            sequence.into_bytes(),
            quality.into_bytes(),
        ));
    }

    Ok(records)
}

fn format_speedup(speedup: f64) -> String {
    format!("{:.2}", speedup)
}

fn print_summary_table(results: &[ScaleResults]) {
    println!();
    println!("╔════════════════════════════════════════════════════════════════════╗");
    println!("║  Performance Summary: GC Content Across Scales                    ║");
    println!("╚════════════════════════════════════════════════════════════════════╝");
    println!();

    // Header
    println!("┌────────────┬─────────────┬────────────────┬────────────────┬────────────────┬────────────────┐");
    println!("│ Scale      │ Sequences   │ Naive          │ NEON           │ Parallel (4T)  │ NEON+Parallel  │");
    println!("│            │             │ (Mseqs/sec)    │ (speedup)      │ (speedup)      │ (speedup)      │");
    println!("├────────────┼─────────────┼────────────────┼────────────────┼────────────────┼────────────────┤");

    for result in results {
        let naive_throughput = result.naive.throughput_seqs_per_sec / 1_000_000.0;

        let neon_speedup = result.neon.as_ref()
            .map(|n| format!("{:.2}×", n.speedup_vs(&result.naive)))
            .unwrap_or_else(|| "N/A".to_string());

        let parallel_speedup = format!("{:.2}×", result.parallel.speedup_vs(&result.naive));

        let combined_speedup = result.combined.as_ref()
            .map(|c| format!("{:.2}×", c.speedup_vs(&result.naive)))
            .unwrap_or_else(|| "N/A".to_string());

        println!("│ {:<10} │ {:>11} │ {:>14.2} │ {:>14} │ {:>14} │ {:>14} │",
            result.scale,
            format_number(result.num_sequences),
            naive_throughput,
            neon_speedup,
            parallel_speedup,
            combined_speedup,
        );
    }

    println!("└────────────┴─────────────┴────────────────┴────────────────┴────────────────┴────────────────┘");
    println!();
}

fn compare_to_base_counting(results: &[ScaleResults]) {
    println!();
    println!("╔════════════════════════════════════════════════════════════════════╗");
    println!("║  Pattern Validation: Compare to Base Counting                    ║");
    println!("╚════════════════════════════════════════════════════════════════════╝");
    println!();

    println!("📊 Expected Patterns (from Base Counting):");
    println!("   • NEON: 53-65× (tiny), 16-18× (large)");
    println!("   • Parallel: <1× (tiny), 7-60× (small-huge)");
    println!("   • Parallel threshold: 1,000 sequences");
    println!();

    println!("🔬 GC Content Actual Results:");

    // Find tiny, small, and large results
    let tiny = results.iter().find(|r| r.scale == "Tiny");
    let small = results.iter().find(|r| r.scale == "Small");
    let large = results.iter().find(|r| r.scale == "Large");

    if let Some(tiny_r) = tiny {
        if let Some(neon) = &tiny_r.neon {
            let speedup = neon.speedup_vs(&tiny_r.naive);
            println!("   • NEON (tiny): {:.2}× [expected: 53-65×]", speedup);
        }
        let par_speedup = tiny_r.parallel.speedup_vs(&tiny_r.naive);
        println!("   • Parallel (tiny): {:.2}× [expected: <1×]", par_speedup);
    }

    if let Some(small_r) = small {
        let par_speedup = small_r.parallel.speedup_vs(&small_r.naive);
        println!("   • Parallel (small 1K): {:.2}× [expected: 7-10×]", par_speedup);
    }

    if let Some(large_r) = large {
        if let Some(neon) = &large_r.neon {
            let speedup = neon.speedup_vs(&large_r.naive);
            println!("   • NEON (large): {:.2}× [expected: 16-18×]", speedup);
        }
        let par_speedup = large_r.parallel.speedup_vs(&large_r.naive);
        println!("   • Parallel (large 100K): {:.2}× [expected: 56-60×]", par_speedup);
    }

    println!();
    println!("💡 Conclusion:");
    println!("   Pattern holds? Check if GC content shows similar trends");
    println!("   If yes → Element-wise category rule validated");
    println!("   If no → Need to understand why GC differs from base counting");
    println!();
}

fn format_number(n: usize) -> String {
    if n >= 1_000_000 {
        format!("{}M", n / 1_000_000)
    } else if n >= 1_000 {
        format!("{}K", n / 1_000)
    } else {
        n.to_string()
    }
}
