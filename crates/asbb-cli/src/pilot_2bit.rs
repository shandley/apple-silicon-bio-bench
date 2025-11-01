//! Phase 2: 2-Bit Encoding Pilot Experiment
//!
//! Compares ASCII vs 2-bit encoding performance across operations and scales.
//!
//! **Research Questions**:
//! 1. What is the actual encoding benefit (2-bit speedup / ASCII speedup)?
//! 2. Does encoding benefit vary by operation category?
//! 3. Does encoding benefit vary by data scale?
//!
//! **Operations Tested**:
//! - Reverse complement (transform operation - expected high benefit)
//! - Base counting (counting operation - expected modest benefit)
//!
//! **Approach**: Agnostic observation, no anchoring to prior results.
//!
//! Run in release mode:
//! ```bash
//! cargo run --release -p asbb-cli --bin asbb-pilot-2bit
//! ```

use anyhow::{Context, Result};
use asbb_core::{encoding::BitSeq, PrimitiveOperation, SequenceRecord};
use asbb_ops::{
    at_content::ATContent,
    base_counting::BaseCounting,
    gc_content::GcContent,
    reverse_complement::ReverseComplement,
    sequence_length::SequenceLength,
};
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;
use std::time::Instant;

/// Dataset scale definition
#[derive(Debug, Clone)]
struct DatasetScale {
    name: &'static str,
    path: &'static str,
    num_sequences: usize,
}

const SCALES: &[DatasetScale] = &[
    DatasetScale {
        name: "Tiny",
        path: "datasets/tiny_100_150bp.fq",
        num_sequences: 100,
    },
    DatasetScale {
        name: "Small",
        path: "datasets/small_1k_150bp.fq",
        num_sequences: 1_000,
    },
    DatasetScale {
        name: "Medium",
        path: "datasets/medium_10k_150bp.fq",
        num_sequences: 10_000,
    },
    DatasetScale {
        name: "Large",
        path: "datasets/large_100k_150bp.fq",
        num_sequences: 100_000,
    },
    DatasetScale {
        name: "VeryLarge",
        path: "datasets/vlarge_1m_150bp.fq",
        num_sequences: 1_000_000,
    },
    DatasetScale {
        name: "Huge",
        path: "datasets/huge_10m_150bp.fq",
        num_sequences: 10_000_000,
    },
];

fn main() -> Result<()> {
    println!("╔════════════════════════════════════════════════════════════════════╗");
    println!("║        Phase 2: 2-Bit Encoding Experiments                        ║");
    println!("║        Comparing ASCII vs 2-Bit Performance                        ║");
    println!("╚════════════════════════════════════════════════════════════════════╝");
    println!();

    println!("🔬 Approach: Objective observation, no prior expectations");
    println!("📊 Operations: 5 compatible operations across categories");
    println!("   • Transform: reverse complement");
    println!("   • Counting (all): base counting");
    println!("   • Counting (GC): gc content");
    println!("   • Counting (AT): at content");
    println!("   • Simple: sequence length");
    println!("📏 Scales: 100 → 10M sequences");
    println!();

    // Test reverse complement (expected to show encoding benefit)
    println!("═══════════════════════════════════════════════════════════════");
    println!("🧬 Reverse Complement (Transform Operation)");
    println!("═══════════════════════════════════════════════════════════════");
    println!();

    test_reverse_complement_encoding()?;

    println!();
    println!("═══════════════════════════════════════════════════════════════");
    println!("🔢 Base Counting (Counting Operation)");
    println!("═══════════════════════════════════════════════════════════════");
    println!();

    test_base_counting_encoding()?;

    println!();
    println!("═══════════════════════════════════════════════════════════════");
    println!("🧬 GC Content (Counting Operation - Specific Bases)");
    println!("═══════════════════════════════════════════════════════════════");
    println!();

    test_gc_content_encoding()?;

    println!();
    println!("═══════════════════════════════════════════════════════════════");
    println!("🧬 AT Content (Counting Operation - Specific Bases)");
    println!("═══════════════════════════════════════════════════════════════");
    println!();

    test_at_content_encoding()?;

    println!();
    println!("═══════════════════════════════════════════════════════════════");
    println!("📏 Sequence Length (Simple Operation)");
    println!("═══════════════════════════════════════════════════════════════");
    println!();

    test_sequence_length_encoding()?;

    println!();
    println!("═══════════════════════════════════════════════════════════════");
    println!("✅ Phase 2 Encoding Experiments Complete");
    println!("═══════════════════════════════════════════════════════════════");
    println!();
    println!("📊 Operations tested: 5 (2-bit compatible)");
    println!("⚠️  Note: 3 operations skipped (incompatible with 2-bit):");
    println!("   • quality_aggregation (requires quality scores)");
    println!("   • n_content (N bases encoded as A in 2-bit)");
    println!("   • complexity_score (character diversity changes)");
    println!();
    println!("Next steps:");
    println!("1. Analyze encoding benefit patterns across operation types");
    println!("2. Document which operations benefit from 2-bit encoding");
    println!("3. Update Phase 2 findings with complete dataset");

    Ok(())
}

fn test_reverse_complement_encoding() -> Result<()> {
    let op = ReverseComplement::new();

    println!("Testing ASCII vs 2-bit encoding across scales...");
    println!();

    for scale in SCALES {
        println!("📦 Scale: {} ({} sequences)", scale.name, scale.num_sequences);

        // Load ASCII data
        let ascii_records = load_fastq(scale.path)
            .with_context(|| format!("Failed to load {}", scale.path))?;

        if ascii_records.is_empty() {
            println!("   ⚠️  Skipping {} (file not found or empty)", scale.name);
            println!();
            continue;
        }

        // Convert to 2-bit
        let bitseqs: Vec<BitSeq> = ascii_records
            .iter()
            .map(|r| BitSeq::from_ascii(&r.sequence))
            .collect();

        // Benchmark ASCII naive
        let start = Instant::now();
        let _ = op.execute_naive(&ascii_records)?;
        let ascii_naive_time = start.elapsed();

        // Benchmark ASCII NEON
        let start = Instant::now();
        let _ = op.execute_neon(&ascii_records)?;
        let ascii_neon_time = start.elapsed();

        // Benchmark 2-bit naive
        let start = Instant::now();
        let _ = op.execute_2bit_naive(&bitseqs)?;
        let bit_naive_time = start.elapsed();

        // Benchmark 2-bit NEON
        let start = Instant::now();
        let _ = op.execute_2bit_neon(&bitseqs)?;
        let bit_neon_time = start.elapsed();

        // Calculate speedups
        let ascii_neon_speedup = ascii_naive_time.as_secs_f64() / ascii_neon_time.as_secs_f64();
        let bit_neon_speedup = bit_naive_time.as_secs_f64() / bit_neon_time.as_secs_f64();
        let encoding_benefit = bit_neon_time.as_secs_f64() / ascii_neon_time.as_secs_f64();
        let encoding_benefit_naive = bit_naive_time.as_secs_f64() / ascii_naive_time.as_secs_f64();

        println!("   ASCII naive:      {:>8.3} ms", ascii_naive_time.as_secs_f64() * 1000.0);
        println!("   ASCII NEON:       {:>8.3} ms  ({:.2}× vs naive)",
                 ascii_neon_time.as_secs_f64() * 1000.0, ascii_neon_speedup);
        println!("   2-bit naive:      {:>8.3} ms  ({:.2}× vs ASCII naive)",
                 bit_naive_time.as_secs_f64() * 1000.0, 1.0 / encoding_benefit_naive);
        println!("   2-bit NEON:       {:>8.3} ms  ({:.2}× vs 2-bit naive)",
                 bit_neon_time.as_secs_f64() * 1000.0, bit_neon_speedup);
        println!();
        println!("   🎯 Encoding benefit: {:.2}× (2-bit NEON vs ASCII NEON)",
                 1.0 / encoding_benefit);
        println!();
    }

    Ok(())
}

fn test_base_counting_encoding() -> Result<()> {
    let op = BaseCounting::new();

    println!("Testing ASCII vs 2-bit encoding across scales...");
    println!();

    for scale in SCALES {
        println!("📦 Scale: {} ({} sequences)", scale.name, scale.num_sequences);

        // Load ASCII data
        let ascii_records = load_fastq(scale.path)
            .with_context(|| format!("Failed to load {}", scale.path))?;

        if ascii_records.is_empty() {
            println!("   ⚠️  Skipping {} (file not found or empty)", scale.name);
            println!();
            continue;
        }

        // Convert to 2-bit
        let bitseqs: Vec<BitSeq> = ascii_records
            .iter()
            .map(|r| BitSeq::from_ascii(&r.sequence))
            .collect();

        // Benchmark ASCII naive
        let start = Instant::now();
        let _ = op.execute_naive(&ascii_records)?;
        let ascii_naive_time = start.elapsed();

        // Benchmark ASCII NEON
        let start = Instant::now();
        let _ = op.execute_neon(&ascii_records)?;
        let ascii_neon_time = start.elapsed();

        // Benchmark 2-bit naive
        let start = Instant::now();
        let _ = op.execute_2bit_naive(&bitseqs)?;
        let bit_naive_time = start.elapsed();

        // Benchmark 2-bit NEON
        let start = Instant::now();
        let _ = op.execute_2bit_neon(&bitseqs)?;
        let bit_neon_time = start.elapsed();

        // Calculate speedups
        let ascii_neon_speedup = ascii_naive_time.as_secs_f64() / ascii_neon_time.as_secs_f64();
        let bit_neon_speedup = bit_naive_time.as_secs_f64() / bit_neon_time.as_secs_f64();
        let encoding_benefit = bit_neon_time.as_secs_f64() / ascii_neon_time.as_secs_f64();
        let encoding_benefit_naive = bit_naive_time.as_secs_f64() / ascii_naive_time.as_secs_f64();

        println!("   ASCII naive:      {:>8.3} ms", ascii_naive_time.as_secs_f64() * 1000.0);
        println!("   ASCII NEON:       {:>8.3} ms  ({:.2}× vs naive)",
                 ascii_neon_time.as_secs_f64() * 1000.0, ascii_neon_speedup);
        println!("   2-bit naive:      {:>8.3} ms  ({:.2}× vs ASCII naive)",
                 bit_naive_time.as_secs_f64() * 1000.0, 1.0 / encoding_benefit_naive);
        println!("   2-bit NEON:       {:>8.3} ms  ({:.2}× vs 2-bit naive)",
                 bit_neon_time.as_secs_f64() * 1000.0, bit_neon_speedup);
        println!();
        println!("   🎯 Encoding benefit: {:.2}× (2-bit NEON vs ASCII NEON)",
                 1.0 / encoding_benefit);
        println!();
    }

    Ok(())
}

fn test_gc_content_encoding() -> Result<()> {
    let op = GcContent;

    println!("Testing ASCII vs 2-bit encoding across scales...");
    println!();

    for scale in SCALES {
        println!("📦 Scale: {} ({} sequences)", scale.name, scale.num_sequences);

        // Load ASCII data
        let ascii_records = load_fastq(scale.path)
            .with_context(|| format!("Failed to load {}", scale.path))?;

        if ascii_records.is_empty() {
            println!("   ⚠️  Skipping {} (file not found or empty)", scale.name);
            println!();
            continue;
        }

        // Convert to 2-bit
        let bitseqs: Vec<BitSeq> = ascii_records
            .iter()
            .map(|r| BitSeq::from_ascii(&r.sequence))
            .collect();

        // Benchmark ASCII naive
        let start = Instant::now();
        let _ = op.execute_naive(&ascii_records)?;
        let ascii_naive_time = start.elapsed();

        // Benchmark ASCII NEON
        let start = Instant::now();
        let _ = op.execute_neon(&ascii_records)?;
        let ascii_neon_time = start.elapsed();

        // Benchmark 2-bit naive
        let start = Instant::now();
        let _ = op.execute_2bit_naive(&bitseqs)?;
        let bit_naive_time = start.elapsed();

        // Benchmark 2-bit NEON
        let start = Instant::now();
        let _ = op.execute_2bit_neon(&bitseqs)?;
        let bit_neon_time = start.elapsed();

        // Calculate speedups
        let ascii_neon_speedup = ascii_naive_time.as_secs_f64() / ascii_neon_time.as_secs_f64();
        let bit_neon_speedup = bit_naive_time.as_secs_f64() / bit_neon_time.as_secs_f64();
        let encoding_benefit = bit_neon_time.as_secs_f64() / ascii_neon_time.as_secs_f64();
        let encoding_benefit_naive = bit_naive_time.as_secs_f64() / ascii_naive_time.as_secs_f64();

        println!("   ASCII naive:      {:>8.3} ms", ascii_naive_time.as_secs_f64() * 1000.0);
        println!("   ASCII NEON:       {:>8.3} ms  ({:.2}× vs naive)",
                 ascii_neon_time.as_secs_f64() * 1000.0, ascii_neon_speedup);
        println!("   2-bit naive:      {:>8.3} ms  ({:.2}× vs ASCII naive)",
                 bit_naive_time.as_secs_f64() * 1000.0, 1.0 / encoding_benefit_naive);
        println!("   2-bit NEON:       {:>8.3} ms  ({:.2}× vs 2-bit naive)",
                 bit_neon_time.as_secs_f64() * 1000.0, bit_neon_speedup);
        println!();
        println!("   🎯 Encoding benefit: {:.2}× (2-bit NEON vs ASCII NEON)",
                 1.0 / encoding_benefit);
        println!();
    }

    Ok(())
}

fn test_at_content_encoding() -> Result<()> {
    let op = ATContent;

    println!("Testing ASCII vs 2-bit encoding across scales...");
    println!();

    for scale in SCALES {
        println!("📦 Scale: {} ({} sequences)", scale.name, scale.num_sequences);

        // Load ASCII data
        let ascii_records = load_fastq(scale.path)
            .with_context(|| format!("Failed to load {}", scale.path))?;

        if ascii_records.is_empty() {
            println!("   ⚠️  Skipping {} (file not found or empty)", scale.name);
            println!();
            continue;
        }

        // Convert to 2-bit
        let bitseqs: Vec<BitSeq> = ascii_records
            .iter()
            .map(|r| BitSeq::from_ascii(&r.sequence))
            .collect();

        // Benchmark ASCII naive
        let start = Instant::now();
        let _ = op.execute_naive(&ascii_records)?;
        let ascii_naive_time = start.elapsed();

        // Benchmark ASCII NEON
        let start = Instant::now();
        let _ = op.execute_neon(&ascii_records)?;
        let ascii_neon_time = start.elapsed();

        // Benchmark 2-bit naive
        let start = Instant::now();
        let _ = op.execute_2bit_naive(&bitseqs)?;
        let bit_naive_time = start.elapsed();

        // Benchmark 2-bit NEON
        let start = Instant::now();
        let _ = op.execute_2bit_neon(&bitseqs)?;
        let bit_neon_time = start.elapsed();

        // Calculate speedups
        let ascii_neon_speedup = ascii_naive_time.as_secs_f64() / ascii_neon_time.as_secs_f64();
        let bit_neon_speedup = bit_naive_time.as_secs_f64() / bit_neon_time.as_secs_f64();
        let encoding_benefit = bit_neon_time.as_secs_f64() / ascii_neon_time.as_secs_f64();
        let encoding_benefit_naive = bit_naive_time.as_secs_f64() / ascii_naive_time.as_secs_f64();

        println!("   ASCII naive:      {:>8.3} ms", ascii_naive_time.as_secs_f64() * 1000.0);
        println!("   ASCII NEON:       {:>8.3} ms  ({:.2}× vs naive)",
                 ascii_neon_time.as_secs_f64() * 1000.0, ascii_neon_speedup);
        println!("   2-bit naive:      {:>8.3} ms  ({:.2}× vs ASCII naive)",
                 bit_naive_time.as_secs_f64() * 1000.0, 1.0 / encoding_benefit_naive);
        println!("   2-bit NEON:       {:>8.3} ms  ({:.2}× vs 2-bit naive)",
                 bit_neon_time.as_secs_f64() * 1000.0, bit_neon_speedup);
        println!();
        println!("   🎯 Encoding benefit: {:.2}× (2-bit NEON vs ASCII NEON)",
                 1.0 / encoding_benefit);
        println!();
    }

    Ok(())
}

fn test_sequence_length_encoding() -> Result<()> {
    let op = SequenceLength;

    println!("Testing ASCII vs 2-bit encoding across scales...");
    println!();

    for scale in SCALES {
        println!("📦 Scale: {} ({} sequences)", scale.name, scale.num_sequences);

        // Load ASCII data
        let ascii_records = load_fastq(scale.path)
            .with_context(|| format!("Failed to load {}", scale.path))?;

        if ascii_records.is_empty() {
            println!("   ⚠️  Skipping {} (file not found or empty)", scale.name);
            println!();
            continue;
        }

        // Convert to 2-bit
        let bitseqs: Vec<BitSeq> = ascii_records
            .iter()
            .map(|r| BitSeq::from_ascii(&r.sequence))
            .collect();

        // Benchmark ASCII naive
        let start = Instant::now();
        let _ = op.execute_naive(&ascii_records)?;
        let ascii_naive_time = start.elapsed();

        // Benchmark ASCII NEON
        let start = Instant::now();
        let _ = op.execute_neon(&ascii_records)?;
        let ascii_neon_time = start.elapsed();

        // Benchmark 2-bit naive
        let start = Instant::now();
        let _ = op.execute_2bit_naive(&bitseqs)?;
        let bit_naive_time = start.elapsed();

        // Benchmark 2-bit NEON
        let start = Instant::now();
        let _ = op.execute_2bit_neon(&bitseqs)?;
        let bit_neon_time = start.elapsed();

        // Calculate speedups
        let ascii_neon_speedup = ascii_naive_time.as_secs_f64() / ascii_neon_time.as_secs_f64();
        let bit_neon_speedup = bit_naive_time.as_secs_f64() / bit_neon_time.as_secs_f64();
        let encoding_benefit = bit_neon_time.as_secs_f64() / ascii_neon_time.as_secs_f64();
        let encoding_benefit_naive = bit_naive_time.as_secs_f64() / ascii_naive_time.as_secs_f64();

        println!("   ASCII naive:      {:>8.3} ms", ascii_naive_time.as_secs_f64() * 1000.0);
        println!("   ASCII NEON:       {:>8.3} ms  ({:.2}× vs naive)",
                 ascii_neon_time.as_secs_f64() * 1000.0, ascii_neon_speedup);
        println!("   2-bit naive:      {:>8.3} ms  ({:.2}× vs ASCII naive)",
                 bit_naive_time.as_secs_f64() * 1000.0, 1.0 / encoding_benefit_naive);
        println!("   2-bit NEON:       {:>8.3} ms  ({:.2}× vs 2-bit naive)",
                 bit_neon_time.as_secs_f64() * 1000.0, bit_neon_speedup);
        println!();
        println!("   🎯 Encoding benefit: {:.2}× (2-bit NEON vs ASCII NEON)",
                 1.0 / encoding_benefit);
        println!();
    }

    Ok(())
}

/// Load FASTQ file into SequenceRecords
fn load_fastq(path: &str) -> Result<Vec<SequenceRecord>> {
    let file_path = Path::new(path);
    if !file_path.exists() {
        return Ok(Vec::new()); // Return empty if file doesn't exist
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
