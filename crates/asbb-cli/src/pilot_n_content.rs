//! N-Content Multi-Scale Pilot Experiment (N=5 Validation)
//!
//! Tests if simple counting patterns hold with N-content calculation.
//!
//! **Hypothesis** (from N=4 validation):
//! - Simple counting ops (base, GC): 14-65× NEON, 40-75× parallel at scale
//! - Complex aggregation (quality): 7-23× NEON, 20-25× parallel at scale
//! - N-content is simple counting → should match base/GC patterns
//!
//! **Goal**: Reach N=5 for VERY HIGH confidence on counting sub-category.
//!
//! Run in release mode:
//! ```bash
//! cargo run --release -p asbb-cli --bin asbb-pilot-ncontent
//! ```

use anyhow::{Context, Result};
use asbb_core::{HardwareConfig, SequenceRecord};
use asbb_explorer::benchmark_operation;
use asbb_ops::n_content::NContent;
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
    println!("║  N-Content Multi-Scale Pilot (N=5 Validation)                    ║");
    println!("║  Confirming Simple Counting Sub-Category Patterns                ║");
    println!("╚════════════════════════════════════════════════════════════════════╝");
    println!();

    println!("🎯 Hypothesis: N-content shows same patterns as simple counting ops");
    println!("   - NEON: 14-65× speedup (scale-dependent, like base/GC)");
    println!("   - Parallel threshold: 1,000 sequences");
    println!("   - Combined: 40-75× at large scale");
    println!();

    println!("🔬 Question: Confirm counting sub-category with N=5 confidence");
    println!();

    // Create operation
    let operation = NContent;

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
    println!("║  Performance Summary: N-Content Across Scales                     ║");
    println!("╚════════════════════════════════════════════════════════════════════╝");
    println!();

    println!();
    println!("╔════════════════════════════════════════════════════════════════════╗");
    println!("║  N=5 Validation: Simple Counting Sub-Category                    ║");
    println!("╚════════════════════════════════════════════════════════════════════╝");
    println!();

    println!("📊 Expected Patterns (from N=4):");
    println!("   Simple counting (base, GC):");
    println!("   • NEON (tiny): 35-65× speedup");
    println!("   • NEON (large): 14-18× speedup");
    println!("   • Parallel (1K): 7-13× speedup");
    println!("   • Parallel (100K+): 40-75× speedup");
    println!();

    println!("   Complex aggregation (quality):");
    println!("   • NEON (tiny): 16-23× speedup (lower)");
    println!("   • NEON (large): 7-8× speedup (lower)");
    println!("   • Parallel (1K): 1.28× speedup (much lower)");
    println!("   • Parallel (100K+): 18-25× speedup (lower)");
    println!();

    println!("🔬 N-Content Results:");
    println!("   Compare to simple counting patterns above...");
    println!();

    println!("💡 N=5 Validation:");
    println!();
    println!("   Operation         | NEON (tiny) | NEON (large) | Parallel (1K) | Parallel (100K)");
    println!("   ------------------|-------------|--------------|---------------|----------------");
    println!("   Base counting     | 53-65×      | 16-18×       | 7.33×         | 56.61×");
    println!("   GC content        | 35×         | 14×          | 13.42×        | 43.77×");
    println!("   N-content         | ???         | ???          | ???           | ???");
    println!("   Quality aggr.     | 16-23×      | 7-8×         | 1.28×         | 18-25× (complex)");
    println!("   Reverse comp.     | 1×          | 1×           | 1.69×         | 3.68× (transform)");
    println!();

    println!("   N=5 Assessment:");
    println!("   ✅ If N-content matches base/GC → Simple counting sub-category CONFIRMED");
    println!("   ✅ VERY HIGH confidence (N=5) for simple counting rules");
    println!("   ✅ Ready for publication and Phase 2 (2-bit encoding)");
    println!();

    println!();
    println!("╔════════════════════════════════════════════════════════════════════╗");
    println!("║  N-Content Pilot Complete (N=5 Validation)                       ║");
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
