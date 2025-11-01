//! Quick benchmark for sequence_masking operation
//!
//! Tests naive, NEON, and parallel implementations to validate:
//! - Correctness (all produce same results)
//! - Performance (NEON should be 20-40× faster)
//! - Scalability (parallel should add 2-4× on top)

use anyhow::Result;
use asbb_core::SequenceRecord;
use asbb_ops::sequence_masking::SequenceMasking;
use asbb_ops::PrimitiveOperation;
use rand::prelude::*;
use std::time::Instant;

fn generate_test_data(num_sequences: usize, seq_length: usize, seed: u64) -> Vec<SequenceRecord> {
    let mut rng = rand::rngs::StdRng::seed_from_u64(seed);

    (0..num_sequences)
        .map(|i| {
            let id = format!("seq_{}", i);

            // Generate random DNA sequence
            let sequence: Vec<u8> = (0..seq_length)
                .map(|_| match rng.gen_range(0..4) {
                    0 => b'A',
                    1 => b'C',
                    2 => b'G',
                    _ => b'T',
                })
                .collect();

            // Generate quality scores with realistic distribution
            // Mix of high-quality (Q30-Q40) and low-quality (Q5-Q15) bases
            let quality: Vec<u8> = (0..seq_length)
                .map(|_| {
                    if rng.gen_bool(0.8) {
                        // 80% high quality (Q30-Q40)
                        rng.gen_range(63..74) // 30+33 to 40+33
                    } else {
                        // 20% low quality (Q5-Q15) - will be masked
                        rng.gen_range(38..48) // 5+33 to 15+33
                    }
                })
                .collect();

            SequenceRecord::fastq(id, sequence, quality)
        })
        .collect()
}

fn benchmark_operation(
    op: &SequenceMasking,
    data: &[SequenceRecord],
    backend: &str,
    warmup: usize,
    runs: usize,
) -> Result<(f64, usize)> {
    // Warmup
    for _ in 0..warmup {
        match backend {
            "naive" => { let _ = op.execute_naive(data)?; }
            "neon" => { let _ = op.execute_neon(data)?; }
            "parallel_2" => { let _ = op.execute_parallel(data, 2)?; }
            "parallel_4" => { let _ = op.execute_parallel(data, 4)?; }
            _ => panic!("Unknown backend: {}", backend),
        }
    }

    // Measured runs
    let mut durations = Vec::with_capacity(runs);
    let mut masked_count = 0;

    for _ in 0..runs {
        let start = Instant::now();

        let result = match backend {
            "naive" => op.execute_naive(data)?,
            "neon" => op.execute_neon(data)?,
            "parallel_2" => op.execute_parallel(data, 2)?,
            "parallel_4" => op.execute_parallel(data, 4)?,
            _ => panic!("Unknown backend: {}", backend),
        };

        let duration = start.elapsed();
        durations.push(duration);

        // Count masked bases (first run only)
        if masked_count == 0 {
            if let asbb_core::OperationOutput::Records(records) = result {
                for record in &records {
                    masked_count += record.sequence.iter().filter(|&&b| b == b'N').count();
                }
            }
        }
    }

    // Calculate median time
    durations.sort();
    let median = durations[runs / 2];
    let median_secs = median.as_secs_f64();

    Ok((median_secs, masked_count))
}

fn main() -> Result<()> {
    println!("╔════════════════════════════════════════════════════════════════╗");
    println!("║        Sequence Masking Operation - Quick Benchmark           ║");
    println!("╚════════════════════════════════════════════════════════════════╝");
    println!();

    let op = SequenceMasking::new();

    // Test scales
    let scales = vec![
        ("Tiny", 100),
        ("Small", 1_000),
        ("Medium", 10_000),
        ("Large", 100_000),
    ];

    let seq_length = 150;
    let warmup = 2;
    let runs = 5;

    println!("Configuration:");
    println!("  Sequence length: {} bp", seq_length);
    println!("  Quality threshold: Q20 (mask bases with Q < 20)");
    println!("  Warmup runs: {}", warmup);
    println!("  Measured runs: {}", runs);
    println!();

    for (name, num_seqs) in scales {
        println!("═══════════════════════════════════════════════════════════════");
        println!("📊 Scale: {} ({} sequences, {} total bases)",
                 name, num_seqs, num_seqs * seq_length);
        println!("═══════════════════════════════════════════════════════════════");

        // Generate test data
        let data = generate_test_data(num_seqs, seq_length, 42);

        println!("Generating test data... done");

        // Benchmark backends
        let backends = vec!["naive", "neon", "parallel_2", "parallel_4"];
        let mut results = Vec::new();

        for backend in &backends {
            print!("  Testing {} backend... ", backend);
            std::io::Write::flush(&mut std::io::stdout()).unwrap();

            let (time, masked) = benchmark_operation(&op, &data, backend, warmup, runs)?;
            results.push((backend, time, masked));

            println!("done ({:.3}s, {} masked bases)", time, masked);
        }

        println!();
        println!("Results:");

        let naive_time = results[0].1;
        let masked_count = results[0].2;
        let total_bases = num_seqs * seq_length;
        let masked_pct = (masked_count as f64 / total_bases as f64) * 100.0;

        println!("  Masked bases: {}/{} ({:.1}%)", masked_count, total_bases, masked_pct);
        println!();

        // Print results table
        println!("  Backend         Time (s)    Throughput (seqs/s)    Speedup");
        println!("  ─────────────────────────────────────────────────────────────");

        for (backend, time, _) in &results {
            let throughput = num_seqs as f64 / time;
            let speedup = naive_time / time;
            println!("  {:12}    {:8.4}    {:16.0}     {:6.2}×",
                     backend, time, throughput, speedup);
        }

        println!();

        // Analysis
        let neon_speedup = naive_time / results[1].1;
        let parallel_2_speedup = naive_time / results[2].1;
        let parallel_4_speedup = naive_time / results[3].1;

        println!("Analysis:");
        println!("  NEON speedup: {:.2}× (predicted: 20-40×)", neon_speedup);

        if neon_speedup >= 20.0 {
            println!("    ✓ NEON performing as expected (>20×)");
        } else if neon_speedup >= 10.0 {
            println!("    ⚠ NEON speedup lower than predicted (10-20×)");
        } else if neon_speedup >= 2.0 {
            println!("    ⚠ NEON speedup marginal (2-10×)");
        } else {
            println!("    ✗ NEON not providing benefit (<2×)");
        }

        println!("  Parallel (2 threads): {:.2}× over naive", parallel_2_speedup);
        println!("  Parallel (4 threads): {:.2}× over naive", parallel_4_speedup);

        let parallel_efficiency_4 = (parallel_4_speedup / 4.0) * 100.0;
        println!("  Parallel efficiency (4 threads): {:.1}%", parallel_efficiency_4);

        println!();
    }

    println!("═══════════════════════════════════════════════════════════════");
    println!("Benchmark complete!");
    println!();

    // Correctness validation
    println!("Correctness Validation:");
    println!("  Testing that all backends produce identical results...");

    let test_data = generate_test_data(1000, 150, 12345);

    let naive_result = op.execute_naive(&test_data)?;
    let neon_result = op.execute_neon(&test_data)?;
    let parallel_result = op.execute_parallel(&test_data, 4)?;

    // Extract sequences from results
    let naive_seqs = match naive_result {
        asbb_core::OperationOutput::Records(r) => r,
        _ => panic!("Expected Records"),
    };

    let neon_seqs = match neon_result {
        asbb_core::OperationOutput::Records(r) => r,
        _ => panic!("Expected Records"),
    };

    let parallel_seqs = match parallel_result {
        asbb_core::OperationOutput::Records(r) => r,
        _ => panic!("Expected Records"),
    };

    // Compare
    let mut all_match = true;

    for i in 0..test_data.len() {
        if naive_seqs[i].sequence != neon_seqs[i].sequence {
            println!("  ✗ NEON mismatch at sequence {}", i);
            all_match = false;
            break;
        }
        if naive_seqs[i].sequence != parallel_seqs[i].sequence {
            println!("  ✗ Parallel mismatch at sequence {}", i);
            all_match = false;
            break;
        }
    }

    if all_match {
        println!("  ✓ All backends produce identical results");
    }

    println!();

    Ok(())
}
