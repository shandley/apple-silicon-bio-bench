// Sequence Length Multi-Scale Pilot Experiment (N=6 Validation)
//
// Tests the lower bound of NEON benefit with an extremely simple operation.
//
// **Hypothesis**:
// - Very simple ops may show MINIMAL NEON speedup (overhead dominates)
// - Expected: 1-3× NEON (operation too simple for SIMD benefit)
// - Parallel: High threshold (10K+) due to overhead
//
// **Goal**: Reach N=6, test lower bound of complexity spectrum.
//
// Run in release mode:
// ```bash
// cargo run --release -p asbb-cli --bin asbb-pilot-seqlen
// ```

use anyhow::{Context, Result};
use asbb_core::{HardwareConfig, SequenceRecord};
use asbb_explorer::benchmark_operation;
use asbb_ops::sequence_length::SequenceLength;
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
    println!("║  Sequence Length Multi-Scale Pilot (N=6 Validation)              ║");
    println!("║  Testing Lower Bound of NEON Benefit                             ║");
    println!("╚════════════════════════════════════════════════════════════════════╝");
    println!();

    println!("🎯 Hypothesis: Very simple ops show MINIMAL NEON speedup");
    println!("   - NEON: 1-3× (operation too simple, overhead dominates)");
    println!("   - Parallel threshold: 10K+ sequences");
    println!("   - Complexity score: ~0.2 (lowest yet)");
    println!();

    println!("🔬 Question: What is the lower bound of NEON utility?");
    println!();

    // Create operation
    let operation = SequenceLength;

    // Benchmark parameters
    let warmup_runs = 2;
    let measured_runs = 5;

    // Results storage
    let mut all_results = Vec::new();

    // Run experiments for each scale
    for scale in SCALES {
        println!("╔════════════════════════════════════════════════════════════════════╗");
        println!(
            "║  Scale: {} ({} sequences, ~{:.1} MB)                    ",
            scale.name, scale.num_sequences, scale.expected_size_mb
        );
        println!("╚════════════════════════════════════════════════════════════════════╝");
        println!();

        // Load data
        print!("📂 Loading data... ");
        let data = load_fastq(scale.path).context("Failed to load FASTQ data")?;
        println!("loaded {} sequences", data.len());
        println!();

        // Configuration 1: Naive (baseline)
        let naive_config = HardwareConfig::naive();

        print!("  1️⃣  Naive (baseline)... ");
        let naive_result = benchmark_operation(
            &operation,
            &data,
            &naive_config,
            warmup_runs,
            measured_runs,
        )?;
        println!(
            "✓ {:.2} Mseqs/sec",
            naive_result.throughput_seqs_per_sec / 1_000_000.0
        );

        // Configuration 2: NEON SIMD
        #[cfg(target_arch = "aarch64")]
        {
            let mut neon_config = HardwareConfig::naive();
            neon_config.use_neon = true;

            print!("  2️⃣  NEON SIMD...        ");
            let neon_result = benchmark_operation(
                &operation,
                &data,
                &neon_config,
                warmup_runs,
                measured_runs,
            )?;
            let neon_speedup =
                neon_result.throughput_seqs_per_sec / naive_result.throughput_seqs_per_sec;
            println!(
                "✓ {:.2} Mseqs/sec ({:.2}×)",
                neon_result.throughput_seqs_per_sec / 1_000_000.0,
                neon_speedup
            );

            all_results.push(("neon", scale.name, neon_speedup));
        }

        #[cfg(not(target_arch = "aarch64"))]
        {
            println!("  2️⃣  NEON SIMD...        ⊘ Not available on this platform");
        }

        // Configuration 3: Parallel (4 threads)
        let mut parallel_config = HardwareConfig::naive();
        parallel_config.num_threads = 4;

        print!("  3️⃣  Parallel (4T)...    ");
        let parallel_result = benchmark_operation(
            &operation,
            &data,
            &parallel_config,
            warmup_runs,
            measured_runs,
        )?;
        let parallel_speedup =
            parallel_result.throughput_seqs_per_sec / naive_result.throughput_seqs_per_sec;
        println!(
            "✓ {:.2} Mseqs/sec ({:.2}×)",
            parallel_result.throughput_seqs_per_sec / 1_000_000.0,
            parallel_speedup
        );

        all_results.push(("parallel", scale.name, parallel_speedup));

        // Configuration 4: Combined (NEON + Parallel)
        #[cfg(target_arch = "aarch64")]
        {
            let mut combined_config = HardwareConfig::naive();
            combined_config.use_neon = true;
            combined_config.num_threads = 4;

            print!("  4️⃣  NEON + Parallel... ");
            let combined_result = benchmark_operation(
                &operation,
                &data,
                &combined_config,
                warmup_runs,
                measured_runs,
            )?;
            let combined_speedup =
                combined_result.throughput_seqs_per_sec / naive_result.throughput_seqs_per_sec;
            println!(
                "✓ {:.2} Mseqs/sec ({:.2}×)",
                combined_result.throughput_seqs_per_sec / 1_000_000.0,
                combined_speedup
            );

            all_results.push(("combined", scale.name, combined_speedup));
        }

        #[cfg(not(target_arch = "aarch64"))]
        {
            println!("  4️⃣  NEON + Parallel... ⊘ Not available on this platform");
        }

        println!();
    }

    // Print summary
    println!();
    println!("╔════════════════════════════════════════════════════════════════════╗");
    println!("║  Performance Summary: Sequence Length Across Scales               ║");
    println!("╚════════════════════════════════════════════════════════════════════╝");
    println!();

    println!("📊 N=6 Validation:");
    println!();
    println!("   Operation         | Complexity | NEON (tiny) | NEON (large) | Parallel (1K) | Parallel (100K)");
    println!("   ------------------|------------|-------------|--------------|---------------|----------------");
    println!("   Sequence length   | 0.20       | ???         | ???          | ???           | ???");
    println!("   GC content        | 0.315      | 35×         | 14×          | 13.42×        | 43.77×");
    println!("   Base counting     | 0.39       | 53-65×      | 16-18×       | 7.33×         | 56.61×");
    println!("   N-content         | 0.48       | 8×          | 3-6×         | 1.27×         | 13.90×");
    println!("   Quality aggr.     | 0.61       | 16-23×      | 7-8×         | 1.28×         | 18-25×");
    println!();

    println!("   N=6 Assessment:");
    println!("   ✅ If seq_length shows 1-3× NEON → Lower bound confirmed");
    println!("   ✅ If parallel threshold high (10K+) → Overhead hypothesis confirmed");
    println!("   ✅ Fills out low-complexity end of gradient");
    println!();

    println!();
    println!("╔════════════════════════════════════════════════════════════════════╗");
    println!("║  Sequence Length Pilot Complete (N=6)                            ║");
    println!("╚════════════════════════════════════════════════════════════════════╝");
    println!();

    Ok(())
}

/// Load FASTQ data from file
fn load_fastq(path: impl AsRef<Path>) -> Result<Vec<SequenceRecord>> {
    let file = File::open(path.as_ref())
        .with_context(|| format!("Failed to open FASTQ file: {:?}", path.as_ref()))?;
    let reader = BufReader::new(file);

    let mut records = Vec::new();
    let mut lines = reader.lines();

    while let Some(header) = lines.next() {
        let header = header?;
        if !header.starts_with('@') {
            continue;
        }

        let id = header[1..].split_whitespace().next().unwrap_or("").to_string();

        let sequence = lines
            .next()
            .context("Missing sequence line")??
            .into_bytes();

        let _plus = lines.next().context("Missing + line")??;

        let quality = lines.next().context("Missing quality line")??.into_bytes();

        records.push(SequenceRecord::fastq(id, sequence, quality));
    }

    Ok(records)
}
