//! Phase 1 (GPU): Complexity Score GPU Pilot
//!
//! Tests GPU for our MOST COMPLEX operation (0.61).
//! If any operation shows GPU benefit, it should be this one.

use anyhow::{Context, Result};
use asbb_core::{PrimitiveOperation, SequenceRecord};
use asbb_ops::complexity_score::ComplexityScore;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;
use std::time::Instant;

struct GpuScale {
    name: &'static str,
    path: &'static str,
    num_sequences: usize,
}

const SCALES: &[GpuScale] = &[
    GpuScale { name: "Tiny", path: "datasets/tiny_100_150bp.fq", num_sequences: 100 },
    GpuScale { name: "Small", path: "datasets/small_1k_150bp.fq", num_sequences: 1_000 },
    GpuScale { name: "Medium", path: "datasets/medium_10k_150bp.fq", num_sequences: 10_000 },
    GpuScale { name: "Large", path: "datasets/large_100k_150bp.fq", num_sequences: 50_000 },
    GpuScale { name: "VeryLarge", path: "datasets/large_100k_150bp.fq", num_sequences: 100_000 },
    GpuScale { name: "Huge", path: "datasets/vlarge_1m_150bp.fq", num_sequences: 500_000 },
    GpuScale { name: "Massive1M", path: "datasets/vlarge_1m_150bp.fq", num_sequences: 1_000_000 },
    GpuScale { name: "Massive5M", path: "datasets/huge_10m_150bp.fq", num_sequences: 5_000_000 },
];

fn main() -> Result<()> {
    println!("╔════════════════════════════════════════════════════════════════════╗");
    println!("║   Phase 1 (GPU): Complexity Score - FINAL GPU TEST                ║");
    println!("║   Most Complex Operation (0.61) - Last Chance for GPU             ║");
    println!("╚════════════════════════════════════════════════════════════════════╝\n");

    println!("🔬 Operation: Complexity Score (character diversity)");
    println!("📊 Complexity: 0.61 (HIGHEST we have)");
    println!("🎯 Previous Results: 0.40, 0.45, 0.50 all showed NO GPU benefit\n");

    let op = ComplexityScore::new();

    for scale in SCALES {
        println!("═══════════════════════════════════════════════════════════════");
        println!("📦 {} ({} sequences)", scale.name, scale.num_sequences);

        let mut records = load_fastq(scale.path)?;
        if records.is_empty() {
            println!("   ⚠️  Skipping (file not found)\n");
            continue;
        }
        if records.len() > scale.num_sequences {
            records.truncate(scale.num_sequences);
        }

        let start = Instant::now();
        let _cpu_naive = op.execute_naive(&records)?;
        let cpu_naive_time = start.elapsed();

        let start = Instant::now();
        let _cpu_neon = op.execute_neon(&records)?;
        let cpu_neon_time = start.elapsed();

        #[cfg(all(target_os = "macos", feature = "gpu"))]
        let (_, gpu_metrics, gpu_total_time) = {
            let start = Instant::now();
            let (result, metrics) = op.execute_gpu(&records)?;
            (result, metrics, start.elapsed())
        };

        #[cfg(not(all(target_os = "macos", feature = "gpu")))]
        continue;

        let neon_speedup = cpu_naive_time.as_secs_f64() / cpu_neon_time.as_secs_f64();

        #[cfg(all(target_os = "macos", feature = "gpu"))]
        {
            let gpu_vs_neon = cpu_neon_time.as_secs_f64() / gpu_total_time.as_secs_f64();
            let total_ms = gpu_total_time.as_secs_f64() * 1000.0;

            println!("⏱️  CPU Naive:  {:>10.3} ms", cpu_naive_time.as_secs_f64() * 1000.0);
            println!("⏱️  CPU NEON:   {:>10.3} ms  ({:.2}× vs naive)", cpu_neon_time.as_secs_f64() * 1000.0, neon_speedup);
            println!("⏱️  GPU Total:  {:>10.3} ms", total_ms);
            println!("    └─ Kernel:  {:>10.3} ms  ({:.1}% of total)", gpu_metrics.kernel_time_ms, gpu_metrics.kernel_time_ms / total_ms * 100.0);
            println!("🎯 GPU vs NEON: {:.2}× {}\n", gpu_vs_neon, if gpu_vs_neon > 1.0 { "FASTER ✅" } else { "slower" });
        }
    }

    println!("═══════════════════════════════════════════════════════════════");
    println!("✅ Complexity Score GPU Pilot Complete");
    println!("═══════════════════════════════════════════════════════════════\n");
    println!("Next: Final GPU dimension analysis document");

    Ok(())
}

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

        if let (Some(Ok(sequence)), Some(Ok(_plus)), Some(Ok(_quality))) =
            (lines.next(), lines.next(), lines.next())
        {
            records.push(SequenceRecord::fasta(id, sequence.into_bytes()));
        }
    }

    Ok(records)
}
