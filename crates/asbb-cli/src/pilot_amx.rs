//! AMX Matrix Engine Dimension Pilot
//!
//! Systematic testing of AMX (Apple Matrix Extension) performance for matrix-amenable operations.
//!
//! **Research Questions**:
//! 1. Which operations benefit from AMX matrix coprocessor?
//! 2. How does AMX compare to NEON SIMD (128-bit vs 512-bit)?
//! 3. What's the minimum data scale for AMX benefit (overhead threshold)?
//! 4. Do matrix-native algorithms see greater AMX speedup?
//!
//! **Operations Tested**:
//! - edit_distance: Wagner-Fischer DP (matrix-native)
//! - hamming_distance: XOR + popcount (not matrix-native)
//! - quality_statistics: Column-wise matrix statistics
//!
//! **Configurations**:
//! 1. Baseline (naive, no SIMD)
//! 2. NEON (128-bit SIMD)
//! 3. AMX (512-bit matrix operations via Accelerate)
//! 4. Parallel + AMX (multithreaded with AMX per thread)
//!
//! Run in release mode:
//! ```bash
//! cargo run --release -p asbb-cli --bin asbb-pilot-amx > results/phase1_amx_dimension_raw.csv
//! ```

use anyhow::Result;
use asbb_core::{OperationOutput, PrimitiveOperation, SequenceRecord};
use asbb_ops::{
    edit_distance::EditDistance,
    hamming_distance::HammingDistance,
    quality_statistics::QualityStatistics,
};
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;
use std::time::Instant;

/// Backend configuration for experiment
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Backend {
    /// Naive scalar implementation (baseline)
    Naive,
    /// NEON SIMD (128-bit)
    Neon,
    /// AMX matrix operations (512-bit, via Accelerate)
    Amx,
    /// Parallel with AMX per thread
    ParallelAmx,
}

impl Backend {
    fn name(&self) -> &'static str {
        match self {
            Backend::Naive => "naive",
            Backend::Neon => "neon",
            Backend::Amx => "amx",
            Backend::ParallelAmx => "parallel_amx",
        }
    }

    fn description(&self) -> &'static str {
        match self {
            Backend::Naive => "Scalar baseline (no SIMD)",
            Backend::Neon => "NEON 128-bit SIMD",
            Backend::Amx => "AMX 512-bit matrix ops",
            Backend::ParallelAmx => "Parallel + AMX (4 threads)",
        }
    }
}

/// Dataset scale definition
#[derive(Debug, Clone)]
struct Scale {
    name: &'static str,
    path: &'static str,
    num_sequences: usize,
}

const SCALES: &[Scale] = &[
    Scale { name: "Tiny", path: "datasets/tiny_100_150bp.fq", num_sequences: 100 },
    Scale { name: "Small", path: "datasets/small_1k_150bp.fq", num_sequences: 1_000 },
    Scale { name: "Medium", path: "datasets/medium_10k_150bp.fq", num_sequences: 10_000 },
    Scale { name: "Large", path: "datasets/large_100k_150bp.fq", num_sequences: 100_000 },
    Scale { name: "VeryLarge", path: "datasets/vlarge_1m_150bp.fq", num_sequences: 1_000_000 },
    Scale { name: "Huge", path: "datasets/huge_10m_150bp.fq", num_sequences: 10_000_000 },
];

const BACKENDS: &[Backend] = &[
    Backend::Naive,
    Backend::Neon,
    Backend::Amx,
    Backend::ParallelAmx,
];

/// Result of one experiment
#[derive(Debug)]
struct ExperimentResult {
    operation: String,
    complexity: f64,
    scale_name: String,
    num_sequences: usize,
    backend: Backend,
    time_ms: f64,
    speedup_vs_naive: f64,
    speedup_vs_neon: f64,
    throughput: f64,  // sequences/sec
}

impl ExperimentResult {
    fn to_csv_header() -> String {
        "operation,complexity,scale,num_sequences,backend,time_ms,speedup_vs_naive,speedup_vs_neon,throughput_seqs_per_sec".to_string()
    }

    fn to_csv(&self) -> String {
        format!(
            "{},{:.2},{},{},{},{:.6},{:.4},{:.4},{:.2}",
            self.operation,
            self.complexity,
            self.scale_name,
            self.num_sequences,
            self.backend.name(),
            self.time_ms,
            self.speedup_vs_naive,
            self.speedup_vs_neon,
            self.throughput
        )
    }
}

fn main() -> Result<()> {
    eprintln!("╔════════════════════════════════════════════════════════════════════╗");
    eprintln!("║          AMX Matrix Engine Dimension Pilot                         ║");
    eprintln!("║          Systematic Testing of Matrix Operations                   ║");
    eprintln!("╚════════════════════════════════════════════════════════════════════╝");
    eprintln!();

    eprintln!("🔬 Testing: 3 operations × 4 backends × 6 scales = 72 experiments");
    eprintln!("🎯 Goal: Quantify AMX benefit vs NEON for matrix-amenable operations");
    eprintln!("📊 Backends: {:?}", BACKENDS.iter().map(|b| b.name()).collect::<Vec<_>>());
    eprintln!("🖥️  AMX: 512-bit matrix coprocessor (via Accelerate framework)");
    eprintln!();

    eprintln!("💡 Operations:");
    eprintln!("   - edit_distance: Matrix-native (Wagner-Fischer DP)");
    eprintln!("   - hamming_distance: Not matrix-native (XOR + popcount)");
    eprintln!("   - quality_statistics: Column-wise matrix statistics");
    eprintln!();

    // Print CSV header
    println!("{}", ExperimentResult::to_csv_header());

    let mut results: Vec<ExperimentResult> = Vec::new();
    let mut experiment_num = 0;
    let total_experiments = 3 * BACKENDS.len() * SCALES.len();

    // Test edit_distance
    eprintln!("═══════════════════════════════════════════════════════════════════");
    eprintln!("Operation 1/3: edit_distance (complexity: 0.70, pairwise DP)");
    eprintln!("═══════════════════════════════════════════════════════════════════");

    let op = EditDistance::new(100);  // Limit to 100 sequences for N×N matrix
    run_operation_experiments(
        "edit_distance",
        0.70,
        &op,
        &mut results,
        &mut experiment_num,
        total_experiments,
    )?;

    // Test hamming_distance
    eprintln!("\n═══════════════════════════════════════════════════════════════════");
    eprintln!("Operation 2/3: hamming_distance (complexity: 0.35, pairwise XOR)");
    eprintln!("═══════════════════════════════════════════════════════════════════");

    let op = HammingDistance::new();
    run_operation_experiments(
        "hamming_distance",
        0.35,
        &op,
        &mut results,
        &mut experiment_num,
        total_experiments,
    )?;

    // Test quality_statistics
    eprintln!("\n═══════════════════════════════════════════════════════════════════");
    eprintln!("Operation 3/3: quality_statistics (complexity: 0.38, matrix column stats)");
    eprintln!("═══════════════════════════════════════════════════════════════════");

    let op = QualityStatistics::new();
    run_operation_experiments(
        "quality_statistics",
        0.38,
        &op,
        &mut results,
        &mut experiment_num,
        total_experiments,
    )?;

    eprintln!("\n╔════════════════════════════════════════════════════════════════════╗");
    eprintln!("║                    AMX Pilot Complete                              ║");
    eprintln!("╚════════════════════════════════════════════════════════════════════╝");
    eprintln!("✅ Total experiments: {}", results.len());
    eprintln!("📊 Results written to stdout (CSV format)");
    eprintln!("💾 Redirect to: results/phase1_amx_dimension_raw_YYYYMMDD_HHMMSS.csv");
    eprintln!();
    eprintln!("Next steps:");
    eprintln!("1. Analyze results: speedup_vs_naive, speedup_vs_neon");
    eprintln!("2. Identify scale thresholds for AMX benefit");
    eprintln!("3. Compare matrix-native (edit_distance) vs non-matrix (hamming) speedups");
    eprintln!("4. Document findings in results/phase1/phase1_amx_dimension_complete.md");

    Ok(())
}

fn run_operation_experiments(
    operation_name: &str,
    complexity: f64,
    op: &dyn PrimitiveOperation,
    results: &mut Vec<ExperimentResult>,
    experiment_num: &mut usize,
    total_experiments: usize,
) -> Result<()> {
    for scale in SCALES {
        eprintln!("\n  Scale: {} ({} sequences)", scale.name, scale.num_sequences);

        // Load data once for all backends
        let data = load_sequences(scale.path)?;

        // Store baseline times for speedup calculation
        let mut naive_time_ms = 0.0;
        let mut neon_time_ms = 0.0;

        for backend in BACKENDS {
            *experiment_num += 1;
            eprint!("  [{:3}/{:3}] {:12} ... ", experiment_num, total_experiments, backend.name());

            let start = Instant::now();
            let _result = match backend {
                Backend::Naive => op.execute_naive(&data)?,
                Backend::Neon => op.execute_neon(&data)?,
                Backend::Amx => op.execute_amx(&data)?,
                Backend::ParallelAmx => op.execute_parallel(&data, 4)?,
            };
            let elapsed = start.elapsed();
            let time_ms = elapsed.as_secs_f64() * 1000.0;

            // Store baseline times
            if matches!(backend, Backend::Naive) {
                naive_time_ms = time_ms;
            } else if matches!(backend, Backend::Neon) {
                neon_time_ms = time_ms;
            }

            let speedup_vs_naive = if naive_time_ms > 0.0 { naive_time_ms / time_ms } else { 1.0 };
            let speedup_vs_neon = if neon_time_ms > 0.0 { neon_time_ms / time_ms } else { 1.0 };
            let throughput = (scale.num_sequences as f64) / (time_ms / 1000.0);

            let result = ExperimentResult {
                operation: operation_name.to_string(),
                complexity,
                scale_name: scale.name.to_string(),
                num_sequences: scale.num_sequences,
                backend: *backend,
                time_ms,
                speedup_vs_naive,
                speedup_vs_neon,
                throughput,
            };

            eprintln!(
                "{:8.2}ms  ({:.2}× vs naive, {:.2}× vs NEON)",
                time_ms, speedup_vs_naive, speedup_vs_neon
            );

            // Print to stdout (CSV)
            println!("{}", result.to_csv());
            results.push(result);
        }
    }

    Ok(())
}

fn load_sequences<P: AsRef<Path>>(path: P) -> Result<Vec<SequenceRecord>> {
    let file = File::open(path.as_ref())?;
    let reader = BufReader::new(file);
    let mut sequences = Vec::new();
    let mut lines = reader.lines();

    while let Some(header) = lines.next() {
        let header = header?;
        if !header.starts_with('@') {
            continue;
        }

        let id = header[1..].to_string();

        let sequence = match lines.next() {
            Some(Ok(seq)) => seq.into_bytes(),
            _ => break,
        };

        let _plus = match lines.next() {
            Some(Ok(plus)) => plus,
            _ => break,
        };

        let quality = match lines.next() {
            Some(Ok(qual)) => Some(qual.into_bytes()),
            _ => None,
        };

        sequences.push(SequenceRecord { id, sequence, quality });
    }

    Ok(sequences)
}
