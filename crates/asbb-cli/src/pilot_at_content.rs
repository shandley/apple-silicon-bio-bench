// AT Content Multi-Scale Pilot Experiment (N=7 Validation)
//
// Tests if AT content shows same pattern as GC content (simple counting).
//
// Expected: 20-40× NEON at tiny, 12-16× at large, threshold at 1K
// Complexity: ~0.35 (simple, like GC)

use anyhow::{Context, Result};
use asbb_core::{HardwareConfig, SequenceRecord};
use asbb_explorer::benchmark_operation;
use asbb_ops::at_content::ATContent;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

#[derive(Debug)]
struct DatasetScale {
    name: &'static str,
    path: &'static str,
}

const SCALES: &[DatasetScale] = &[
    DatasetScale { name: "Tiny", path: "datasets/tiny_100_150bp.fq" },
    DatasetScale { name: "Small", path: "datasets/small_1k_150bp.fq" },
    DatasetScale { name: "Medium", path: "datasets/medium_10k_150bp.fq" },
    DatasetScale { name: "Large", path: "datasets/large_100k_150bp.fq" },
    DatasetScale { name: "VeryLarge", path: "datasets/vlarge_1m_150bp.fq" },
    DatasetScale { name: "Huge", path: "datasets/huge_10m_150bp.fq" },
];

fn main() -> Result<()> {
    println!("AT Content Multi-Scale Pilot (N=7 Validation)");
    println!("Expected: Similar to GC content (simple counting pattern)\n");

    let operation = ATContent;
    let warmup_runs = 2;
    let measured_runs = 5;

    for scale in SCALES {
        println!("Scale: {}", scale.name);
        let data = load_fastq(scale.path)?;

        let naive_config = HardwareConfig::naive();
        let naive_result = benchmark_operation(&operation, &data, &naive_config, warmup_runs, measured_runs)?;
        let naive_throughput = naive_result.throughput_seqs_per_sec / 1_000_000.0;
        print!("  Naive: {:.2} Mseqs/sec", naive_throughput);

        #[cfg(target_arch = "aarch64")]
        {
            let mut neon_config = HardwareConfig::naive();
            neon_config.use_neon = true;
            let neon_result = benchmark_operation(&operation, &data, &neon_config, warmup_runs, measured_runs)?;
            let neon_speedup = neon_result.throughput_seqs_per_sec / naive_result.throughput_seqs_per_sec;
            print!(" | NEON: {:.2}×", neon_speedup);
        }

        let mut parallel_config = HardwareConfig::naive();
        parallel_config.num_threads = 4;
        let parallel_result = benchmark_operation(&operation, &data, &parallel_config, warmup_runs, measured_runs)?;
        let parallel_speedup = parallel_result.throughput_seqs_per_sec / naive_result.throughput_seqs_per_sec;
        print!(" | Parallel: {:.2}×", parallel_speedup);

        #[cfg(target_arch = "aarch64")]
        {
            let mut combined_config = HardwareConfig::naive();
            combined_config.use_neon = true;
            combined_config.num_threads = 4;
            let combined_result = benchmark_operation(&operation, &data, &combined_config, warmup_runs, measured_runs)?;
            let combined_speedup = combined_result.throughput_seqs_per_sec / naive_result.throughput_seqs_per_sec;
            println!(" | Combined: {:.2}×", combined_speedup);
        }

        #[cfg(not(target_arch = "aarch64"))]
        println!();
    }

    println!("\nAT Content Pilot Complete (N=7)");
    Ok(())
}

fn load_fastq(path: impl AsRef<Path>) -> Result<Vec<SequenceRecord>> {
    let file = File::open(path.as_ref())?;
    let reader = BufReader::new(file);
    let mut records = Vec::new();
    let mut lines = reader.lines();

    while let Some(header) = lines.next() {
        let header = header?;
        if !header.starts_with('@') { continue; }
        let id = header[1..].split_whitespace().next().unwrap_or("").to_string();
        let sequence = lines.next().context("Missing sequence")??.into_bytes();
        let _plus = lines.next().context("Missing +")?;
        let quality = lines.next().context("Missing quality")??.into_bytes();
        records.push(SequenceRecord::fastq(id, sequence, quality));
    }
    Ok(records)
}
