// Length Filter Pilot (N=9) - Simple filtering, complexity ~0.40

use anyhow::{Context, Result};
use asbb_core::{HardwareConfig, SequenceRecord};
use asbb_explorer::benchmark_operation;
use asbb_ops::length_filter::LengthFilter;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

const SCALES: &[(&str, &str)] = &[
    ("Tiny", "datasets/tiny_100_150bp.fq"),
    ("Small", "datasets/small_1k_150bp.fq"),
    ("Medium", "datasets/medium_10k_150bp.fq"),
    ("Large", "datasets/large_100k_150bp.fq"),
    ("VeryLarge", "datasets/vlarge_1m_150bp.fq"),
    ("Huge", "datasets/huge_10m_150bp.fq"),
];

fn main() -> Result<()> {
    println!("Length Filter Pilot (N=9) - Simple filtering\n");
    let operation = LengthFilter::new(100);

    for (name, path) in SCALES {
        print!("{}:", name);
        let data = load_fastq(path)?;

        let naive = benchmark_operation(&operation, &data, &HardwareConfig::naive(), 2, 5)?;
        print!(" Naive {:.2}M", naive.throughput_seqs_per_sec / 1e6);

        let mut parallel_cfg = HardwareConfig::naive();
        parallel_cfg.num_threads = 4;
        let parallel = benchmark_operation(&operation, &data, &parallel_cfg, 2, 5)?;
        println!(" | Parallel {:.2}×", parallel.throughput_seqs_per_sec / naive.throughput_seqs_per_sec);
    }
    println!("\nLength Filter Complete (N=9)");
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
