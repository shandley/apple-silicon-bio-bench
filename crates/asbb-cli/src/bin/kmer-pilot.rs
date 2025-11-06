//! K-mer Operations Pilot Benchmark
//!
//! Quick validation (N=3) before full DAG experiments (N=30).
//! Tests 3 k-mer operations: minimizers, kmer_counting (spectrum), kmer_extraction.
//!
//! Lab Notebook: Entry 034, Day 2 (November 7, 2025)

use anyhow::Result;
use asbb_core::{PrimitiveOperation, SequenceRecord};
use asbb_ops::{kmer_counting::KmerCounting, kmer_extraction::KmerExtraction, minimizers::Minimizers};
use rand::prelude::*;
use rand_chacha::ChaCha8Rng;
use std::time::Instant;

/// Generate test sequences with fixed seed for reproducibility
fn generate_sequences(num_sequences: usize, seq_length: usize) -> Vec<SequenceRecord> {
    let mut rng = ChaCha8Rng::seed_from_u64(42); // Fixed seed
    let bases = [b'A', b'C', b'G', b'T'];

    (0..num_sequences)
        .map(|i| {
            let sequence: Vec<u8> = (0..seq_length)
                .map(|_| bases[rng.gen_range(0..4)])
                .collect();

            let quality: Vec<u8> = (0..seq_length)
                .map(|_| rng.gen_range(30..=40))
                .collect();

            SequenceRecord {
                id: format!("seq_{}", i),
                sequence,
                quality: Some(quality),
            }
        })
        .collect()
}

/// Benchmark an operation with repetitions
fn benchmark_operation<T: PrimitiveOperation>(
    op: &T,
    sequences: &[SequenceRecord],
    config: &str,
    repetitions: usize,
) -> Result<(f64, f64)> {
    let mut times = Vec::with_capacity(repetitions);

    for _ in 0..repetitions {
        let start = Instant::now();

        let _result = match config {
            "naive" => op.execute_naive(sequences)?,
            "neon" => {
                #[cfg(target_arch = "aarch64")]
                {
                    op.execute_neon(sequences)?
                }
                #[cfg(not(target_arch = "aarch64"))]
                {
                    println!("Warning: NEON not available on this platform");
                    op.execute_naive(sequences)?
                }
            }
            "parallel-2t" => op.execute_parallel(sequences, 2)?,
            "parallel-4t" => op.execute_parallel(sequences, 4)?,
            "neon-parallel-2t" => {
                #[cfg(target_arch = "aarch64")]
                {
                    // Parallel calls NEON internally on ARM
                    op.execute_parallel(sequences, 2)?
                }
                #[cfg(not(target_arch = "aarch64"))]
                {
                    op.execute_parallel(sequences, 2)?
                }
            }
            "neon-parallel-4t" => {
                #[cfg(target_arch = "aarch64")]
                {
                    // Parallel calls NEON internally on ARM
                    op.execute_parallel(sequences, 4)?
                }
                #[cfg(not(target_arch = "aarch64"))]
                {
                    op.execute_parallel(sequences, 4)?
                }
            }
            _ => panic!("Unknown config: {}", config),
        };

        let elapsed = start.elapsed();
        times.push(elapsed.as_secs_f64());
    }

    // Calculate median and mean
    times.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let median = times[times.len() / 2];
    let mean = times.iter().sum::<f64>() / times.len() as f64;

    Ok((median, mean))
}

fn main() -> Result<()> {
    println!("=== K-mer Operations Pilot Benchmark ===");
    println!("Lab Notebook: Entry 034, Day 2");
    println!("Purpose: Quick validation (N=3) before full experiments\n");

    // Test parameters
    let scales = vec![
        ("Small", 1_000, 150),
        ("Medium", 10_000, 150),
    ];
    let k_values = vec![6, 21];
    let repetitions = 3; // Quick pilot (N=3 instead of N=30)

    println!("Parameters:");
    println!("  Scales: {:?}", scales.iter().map(|(n, _, _)| n).collect::<Vec<_>>());
    println!("  K-mer sizes: {:?}", k_values);
    println!("  Repetitions: {} (pilot)\n", repetitions);

    for (scale_name, num_sequences, seq_length) in &scales {
        println!("\n=== Scale: {} ({} sequences, {}bp) ===", scale_name, num_sequences, seq_length);

        // Generate sequences once per scale
        println!("Generating {} sequences...", num_sequences);
        let sequences = generate_sequences(*num_sequences, *seq_length);
        println!("Generated {} sequences ({:.2} MB)\n", sequences.len(),
            (sequences.len() * seq_length) as f64 / 1_000_000.0);

        for k in &k_values {
            println!("\n--- K-mer size: k={} ---", k);

            // Test 1: Minimizers
            {
                let op = Minimizers::new(*k, 5); // w=5 (typical window size)
                println!("\n1. Minimizers (k={}, w=5):", k);

                let (naive_median, _) = benchmark_operation(&op, &sequences, "naive", repetitions)?;
                println!("  Naive (1t):      {:.4}s", naive_median);

                #[cfg(target_arch = "aarch64")]
                {
                    let (neon_median, _) = benchmark_operation(&op, &sequences, "neon", repetitions)?;
                    let neon_speedup = naive_median / neon_median;
                    println!("  NEON (1t):       {:.4}s  ({:.2}× vs naive)", neon_median, neon_speedup);
                }

                let (par2_median, _) = benchmark_operation(&op, &sequences, "neon-parallel-2t", repetitions)?;
                let par2_speedup = naive_median / par2_median;
                println!("  NEON+Parallel 2t: {:.4}s  ({:.2}× vs naive)", par2_median, par2_speedup);

                let (par4_median, _) = benchmark_operation(&op, &sequences, "neon-parallel-4t", repetitions)?;
                let par4_speedup = naive_median / par4_median;
                println!("  NEON+Parallel 4t: {:.4}s  ({:.2}× vs naive)", par4_median, par4_speedup);

                // Expected: Parallel should help even if NEON doesn't (4-21× from DAG)
                if par4_speedup >= 5.0 {
                    println!("  ✅ MEETS THRESHOLD (≥5×) - Implement parallel in biometal");
                } else if par4_speedup >= 2.0 {
                    println!("  ⚠️  MODERATE speedup (2-5×) - Consider parallel");
                } else {
                    println!("  ❌ BELOW THRESHOLD (<2×) - Limited benefit");
                }
            }

            // Test 2: K-mer Spectrum (Counting)
            {
                let op = KmerCounting::new(*k, false); // Non-canonical
                println!("\n2. K-mer Spectrum (k={}):", k);

                let (naive_median, _) = benchmark_operation(&op, &sequences, "naive", repetitions)?;
                println!("  Naive (1t):      {:.4}s", naive_median);

                #[cfg(target_arch = "aarch64")]
                {
                    let (neon_median, _) = benchmark_operation(&op, &sequences, "neon", repetitions)?;
                    let neon_speedup = naive_median / neon_median;
                    println!("  NEON (1t):       {:.4}s  ({:.2}× vs naive)", neon_median, neon_speedup);
                }

                let (par2_median, _) = benchmark_operation(&op, &sequences, "neon-parallel-2t", repetitions)?;
                let par2_speedup = naive_median / par2_median;
                println!("  NEON+Parallel 2t: {:.4}s  ({:.2}× vs naive)", par2_median, par2_speedup);

                let (par4_median, _) = benchmark_operation(&op, &sequences, "neon-parallel-4t", repetitions)?;
                let par4_speedup = naive_median / par4_median;
                println!("  NEON+Parallel 4t: {:.4}s  ({:.2}× vs naive)", par4_median, par4_speedup);

                if par4_speedup >= 5.0 {
                    println!("  ✅ MEETS THRESHOLD (≥5×) - Implement parallel in biometal");
                } else if par4_speedup >= 2.0 {
                    println!("  ⚠️  MODERATE speedup (2-5×) - Consider parallel");
                } else {
                    println!("  ❌ BELOW THRESHOLD (<2×) - Limited benefit");
                }
            }

            // Test 3: Simple K-mer Extraction
            {
                let op = KmerExtraction::new(*k, false); // No deduplication
                println!("\n3. K-mer Extraction (k={}):", k);

                let (naive_median, _) = benchmark_operation(&op, &sequences, "naive", repetitions)?;
                println!("  Naive (1t):      {:.4}s", naive_median);

                #[cfg(target_arch = "aarch64")]
                {
                    let (neon_median, _) = benchmark_operation(&op, &sequences, "neon", repetitions)?;
                    let neon_speedup = naive_median / neon_median;
                    println!("  NEON (1t):       {:.4}s  ({:.2}× vs naive)", neon_median, neon_speedup);
                }

                let (par2_median, _) = benchmark_operation(&op, &sequences, "neon-parallel-2t", repetitions)?;
                let par2_speedup = naive_median / par2_median;
                println!("  NEON+Parallel 2t: {:.4}s  ({:.2}× vs naive)", par2_median, par2_speedup);

                let (par4_median, _) = benchmark_operation(&op, &sequences, "neon-parallel-4t", repetitions)?;
                let par4_speedup = naive_median / par4_median;
                println!("  NEON+Parallel 4t: {:.4}s  ({:.2}× vs naive)", par4_median, par4_speedup);

                if par4_speedup >= 5.0 {
                    println!("  ✅ MEETS THRESHOLD (≥5×) - Implement parallel in biometal");
                } else if par4_speedup >= 2.0 {
                    println!("  ⚠️  MODERATE speedup (2-5×) - Consider parallel");
                } else {
                    println!("  ❌ BELOW THRESHOLD (<2×) - Limited benefit");
                }
            }
        }
    }

    println!("\n\n=== Pilot Complete ===");
    println!("Next steps:");
    println!("  1. Review results above");
    println!("  2. If speedups match predictions, proceed to Day 3 (DAG integration)");
    println!("  3. If speedups are low, optimize NEON implementations");
    println!("  4. Full experiments: 36 × N=30 = 1,080 measurements");

    Ok(())
}
