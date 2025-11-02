//! Memory Footprint Pilot
//!
//! Characterizes memory usage of bioinformatics operations across scales.
//! Tests "load-all" pattern to establish baseline memory requirements.
//!
//! Part of Data Access pillar experiments (fourth democratization pillar).

use anyhow::Result;
use asbb_core::SequenceRecord;
use asbb_ops::*;
use std::time::Instant;

/// Memory tracker using system RSS
struct MemoryTracker {
    pid: u32,
}

impl MemoryTracker {
    fn new() -> Self {
        Self {
            pid: std::process::id(),
        }
    }

    /// Get current RSS in bytes (macOS-specific using ps)
    fn current_rss_bytes(&self) -> Result<usize> {
        let output = std::process::Command::new("ps")
            .args(&["-o", "rss=", "-p", &self.pid.to_string()])
            .output()?;

        let rss_kb = String::from_utf8(output.stdout)?
            .trim()
            .parse::<usize>()?;

        Ok(rss_kb * 1024) // Convert KB to bytes
    }

    /// Get current RSS in MB
    fn current_rss_mb(&self) -> Result<f64> {
        Ok(self.current_rss_bytes()? as f64 / 1_048_576.0)
    }
}

#[derive(Debug, Clone, Copy)]
enum Scale {
    Tiny,      // 100
    Small,     // 1,000
    Medium,    // 10,000
    Large,     // 100,000
    VeryLarge, // 1,000,000
}

impl Scale {
    fn num_sequences(&self) -> usize {
        match self {
            Scale::Tiny => 100,
            Scale::Small => 1_000,
            Scale::Medium => 10_000,
            Scale::Large => 100_000,
            Scale::VeryLarge => 1_000_000,
        }
    }

    fn name(&self) -> &'static str {
        match self {
            Scale::Tiny => "Tiny",
            Scale::Small => "Small",
            Scale::Medium => "Medium",
            Scale::Large => "Large",
            Scale::VeryLarge => "VeryLarge",
        }
    }

    fn all() -> Vec<Scale> {
        vec![
            Scale::Tiny,
            Scale::Small,
            Scale::Medium,
            Scale::Large,
            Scale::VeryLarge,
        ]
    }
}

/// Generate synthetic sequences
fn generate_sequences(num_sequences: usize, seq_length: usize) -> Vec<SequenceRecord> {
    use rand::{Rng, SeedableRng};
    use rand::rngs::StdRng;

    let mut rng = StdRng::seed_from_u64(42);
    let bases = b"ACGT";
    let qualities = b"IIIIIIIIII"; // High quality

    (0..num_sequences)
        .map(|i| {
            let seq: Vec<u8> = (0..seq_length)
                .map(|_| bases[rng.gen_range(0..4)])
                .collect();

            let qual: Vec<u8> = (0..seq_length)
                .map(|_| qualities[rng.gen_range(0..qualities.len())])
                .collect();

            SequenceRecord {
                id: format!("seq_{}", i),
                sequence: seq,
                quality: Some(qual),
            }
        })
        .collect()
}

/// Run single memory experiment
fn run_experiment(
    operation_name: &str,
    scale: Scale,
    seq_length: usize,
) -> Result<()> {
    let num_sequences = scale.num_sequences();

    eprintln!(
        "Running: {} @ {} ({} sequences)",
        operation_name,
        scale.name(),
        num_sequences
    );

    let tracker = MemoryTracker::new();

    // Measure baseline memory
    let baseline_mb = tracker.current_rss_mb()?;

    // Generate data and measure memory
    let start = Instant::now();
    let sequences = generate_sequences(num_sequences, seq_length);
    let after_generation_mb = tracker.current_rss_mb()?;

    // Run operation and measure peak memory
    let op_start = Instant::now();
    let peak_mb = match operation_name {
        "base_counting" => {
            let op = base_counting::BaseCounting::new();
            let _result = op.execute_neon(&sequences)?;
            tracker.current_rss_mb()?
        }
        "gc_content" => {
            let op = gc_content::GcContent;
            let _result = op.execute_neon(&sequences)?;
            tracker.current_rss_mb()?
        }
        "quality_filter" => {
            let op = quality_filter::QualityFilter::new(20);
            let _result = op.execute_naive(&sequences)?;
            tracker.current_rss_mb()?
        }
        "sequence_length" => {
            let op = sequence_length::SequenceLength;
            let _result = op.execute_naive(&sequences)?;
            tracker.current_rss_mb()?
        }
        "reverse_complement" => {
            let op = reverse_complement::ReverseComplement::new();
            let _result = op.execute_neon(&sequences)?;
            tracker.current_rss_mb()?
        }
        _ => {
            anyhow::bail!("Unknown operation: {}", operation_name);
        }
    };

    let elapsed = op_start.elapsed();
    let total_time = start.elapsed();

    // Output CSV row
    println!(
        "{},{},{},{},{:.2},{:.2},{:.2},{:.2},{:.0}",
        operation_name,
        scale.name(),
        num_sequences,
        seq_length,
        baseline_mb,
        after_generation_mb,
        peak_mb,
        peak_mb - baseline_mb, // Memory used by operation
        elapsed.as_secs_f64() * 1000.0  // Operation time in ms
    );

    eprintln!(
        "  Memory: baseline={:.1}MB, after_gen={:.1}MB, peak={:.1}MB, used={:.1}MB, time={:.0}ms",
        baseline_mb,
        after_generation_mb,
        peak_mb,
        peak_mb - baseline_mb,
        elapsed.as_secs_f64() * 1000.0
    );

    Ok(())
}

fn main() -> Result<()> {
    eprintln!("=== Memory Footprint Pilot ===");
    eprintln!("Characterizing memory usage of load-all pattern");
    eprintln!("");

    // CSV header
    println!("operation,scale,num_sequences,seq_length,baseline_mb,after_generation_mb,peak_mb,operation_memory_mb,time_ms");

    let operations = vec![
        "base_counting",
        "gc_content",
        "quality_filter",
        "sequence_length",
        "reverse_complement",
    ];

    let seq_length = 150; // Typical read length

    for operation in &operations {
        for scale in Scale::all() {
            if let Err(e) = run_experiment(operation, scale, seq_length) {
                eprintln!("ERROR: {} @ {}: {}", operation, scale.name(), e);
            }

            // Give system time to stabilize
            std::thread::sleep(std::time::Duration::from_secs(1));
        }
    }

    eprintln!("");
    eprintln!("=== Pilot Complete ===");

    Ok(())
}
