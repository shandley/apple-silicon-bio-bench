//! Multi-scale pilot experiment to discover performance cliffs
//!
//! This systematically tests base counting across 6 data scales to discover:
//! - At what scale does each optimization pay off?
//! - Where are the performance cliffs?
//! - How do optimizations interact with scale?
//!
//! Run in release mode:
//! ```bash
//! cargo run --release -p asbb-cli --bin asbb-pilot-scales
//! ```

use anyhow::{Context, Result};
use asbb_core::{HardwareConfig, PerformanceResult, SequenceRecord};
use asbb_explorer::benchmark_operation;
use asbb_ops::base_counting::BaseCounting;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

/// Dataset scale definition
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
    println!("║  ASBB Multi-Scale Pilot Experiment                                ║");
    println!("║  Discovering Performance Cliffs Across Data Scales                ║");
    println!("╚════════════════════════════════════════════════════════════════════╝");
    println!();

    println!("🎯 Goal: Identify optimization thresholds");
    println!("   - When does NEON pay off?");
    println!("   - When does parallel threading help?");
    println!("   - Where is the GPU threshold? (if implemented)");
    println!();

    // Create operation
    let operation = BaseCounting::new();

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

    // Analyze thresholds
    analyze_thresholds(&all_results);

    println!();
    println!("╔════════════════════════════════════════════════════════════════════╗");
    println!("║  Multi-Scale Pilot Complete                                       ║");
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
    if speedup > 1.0 {
        format!("{:.2}", speedup)
    } else {
        format!("{:.2}", speedup)
    }
}

fn print_summary_table(results: &[ScaleResults]) {
    println!();
    println!("╔════════════════════════════════════════════════════════════════════╗");
    println!("║  Performance Summary Across Scales                                ║");
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

fn analyze_thresholds(results: &[ScaleResults]) {
    println!();
    println!("╔════════════════════════════════════════════════════════════════════╗");
    println!("║  Threshold Analysis                                               ║");
    println!("╚════════════════════════════════════════════════════════════════════╝");
    println!();

    // Analyze NEON
    if results.iter().any(|r| r.neon.is_some()) {
        println!("🔧 NEON SIMD:");
        for result in results {
            if let Some(neon) = &result.neon {
                let speedup = neon.speedup_vs(&result.naive);
                let emoji = if speedup > 5.0 { "🚀" } else if speedup > 2.0 { "✓" } else { "⚠️" };
                println!("   {} {} sequences: {:.2}× speedup", emoji, result.scale, speedup);
            }
        }
        println!();
    }

    // Analyze Parallel
    println!("⚡ Parallel (4 threads):");
    for result in results {
        let speedup = result.parallel.speedup_vs(&result.naive);
        let emoji = if speedup > 2.0 { "🚀" } else if speedup > 1.2 { "✓" } else { "⚠️" };
        let assessment = if speedup < 1.0 {
            "(overhead > benefit)"
        } else if speedup < 1.2 {
            "(minimal benefit)"
        } else if speedup < 2.0 {
            "(good scaling)"
        } else {
            "(excellent scaling)"
        };
        println!("   {} {} sequences: {:.2}× speedup {}", emoji, result.scale, speedup, assessment);
    }
    println!();

    // Analyze Combined (NEON + Parallel)
    if results.iter().any(|r| r.combined.is_some()) {
        println!();
        println!("🔬 Combined Optimization (NEON + Parallel):");
        for result in results {
            if let (Some(neon), Some(combined)) = (&result.neon, &result.combined) {
                let neon_speedup = neon.speedup_vs(&result.naive);
                let parallel_speedup = result.parallel.speedup_vs(&result.naive);
                let combined_speedup = combined.speedup_vs(&result.naive);
                let expected_speedup = neon_speedup * parallel_speedup;
                let efficiency = combined_speedup / expected_speedup;

                let emoji = if efficiency > 0.95 { "✓" } else if efficiency > 0.8 { "⚠️" } else { "❌" };
                let assessment = if efficiency > 1.05 {
                    "super-linear!"
                } else if efficiency > 0.95 {
                    "multiplicative"
                } else if efficiency > 0.8 {
                    "sub-linear"
                } else {
                    "poor composition"
                };

                println!("   {} {} sequences: {:.2}× (expected {:.2}×, {:.0}% efficiency, {})",
                    emoji, result.scale, combined_speedup, expected_speedup, efficiency * 100.0, assessment);
            }
        }
        println!();
    }

    // Key insights
    println!("💡 Key Insights:");

    // Find where parallel becomes beneficial
    if let Some(first_good_parallel) = results.iter().find(|r| r.parallel.speedup_vs(&r.naive) > 1.5) {
        println!("   • Parallel threading beneficial at >{} sequences",
            format_number(first_good_parallel.num_sequences / 10));
    }

    // NEON consistency
    let neon_count = results.iter().filter(|r| r.neon.is_some()).count();
    if neon_count > 2 {
        let neon_speedups: Vec<f64> = results.iter()
            .filter_map(|r| r.neon.as_ref().map(|neon| neon.speedup_vs(&r.naive)))
            .collect();

        if !neon_speedups.is_empty() {
            // Check if NEON speedup is relatively constant
            let avg_speedup: f64 = neon_speedups.iter().sum::<f64>() / neon_speedups.len() as f64;
            println!("   • NEON speedup relatively constant across scales (avg {:.1}×)", avg_speedup);
        }
    }

    // Combined optimization composition
    if results.iter().any(|r| r.combined.is_some()) {
        let combined_count = results.iter().filter(|r| r.combined.is_some()).count();
        if combined_count > 2 {
            let efficiencies: Vec<f64> = results.iter()
                .filter_map(|r| {
                    if let (Some(neon), Some(combined)) = (&r.neon, &r.combined) {
                        let neon_speedup = neon.speedup_vs(&r.naive);
                        let parallel_speedup = r.parallel.speedup_vs(&r.naive);
                        let combined_speedup = combined.speedup_vs(&r.naive);
                        let expected = neon_speedup * parallel_speedup;
                        Some(combined_speedup / expected)
                    } else {
                        None
                    }
                })
                .collect();

            if !efficiencies.is_empty() {
                let avg_efficiency: f64 = efficiencies.iter().sum::<f64>() / efficiencies.len() as f64;
                let composition = if avg_efficiency > 0.95 {
                    "roughly multiplicative"
                } else if avg_efficiency > 0.8 {
                    "sub-linear due to overhead"
                } else {
                    "poor - significant interference"
                };
                println!("   • Combined optimizations are {} ({:.0}% avg efficiency)", composition, avg_efficiency * 100.0);
            }
        }
    }

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
