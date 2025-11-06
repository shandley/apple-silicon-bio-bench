// Entry 035: K-mer 2-bit + ntHash Pilot Benchmark
// Tests non-traditional approaches to k-mer optimization

use asbb_core::SequenceRecord;
use asbb_ops::kmer_2bit;
use asbb_ops::minimizers::Minimizers; // For FNV-1a baseline
use rand::Rng;
use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;
use std::time::Instant;

/// Generate random DNA sequences for benchmarking
fn generate_sequences(num_sequences: usize, seq_length: usize) -> Vec<SequenceRecord> {
    let mut rng = ChaCha8Rng::seed_from_u64(42);
    let bases = [b'A', b'C', b'G', b'T'];

    (0..num_sequences)
        .map(|i| {
            let sequence: Vec<u8> = (0..seq_length)
                .map(|_| bases[rng.gen_range(0..4)])
                .collect();

            let quality: Vec<u8> = (0..seq_length).map(|_| b'I').collect();

            SequenceRecord {
                id: format!("seq_{}", i),
                sequence,
                quality: Some(quality),
            }
        })
        .collect()
}

/// Baseline: ASCII + FNV-1a hash (Entry 034 approach)
fn extract_kmers_baseline(sequences: &[SequenceRecord], k: usize) -> Vec<u64> {
    let mut hashes = Vec::new();

    for record in sequences {
        for i in 0..=(record.sequence.len().saturating_sub(k)) {
            let kmer = &record.sequence[i..i + k];
            if kmer.iter().all(|&b| matches!(b, b'A' | b'C' | b'G' | b'T')) {
                hashes.push(fnv1a_hash(kmer));
            }
        }
    }

    hashes
}

/// FNV-1a hash (baseline from Entry 034)
fn fnv1a_hash(data: &[u8]) -> u64 {
    const FNV_OFFSET: u64 = 0xcbf29ce484222325;
    const FNV_PRIME: u64 = 0x100000001b3;

    let mut hash = FNV_OFFSET;
    for &byte in data {
        hash ^= byte as u64;
        hash = hash.wrapping_mul(FNV_PRIME);
    }
    hash
}

/// Variant 1: 2-bit + Wang hash
fn extract_kmers_2bit_wang(sequences: &[SequenceRecord], k: usize) -> Vec<u64> {
    let mut hashes = Vec::new();

    for record in sequences {
        hashes.extend(kmer_2bit::extract_kmers_2bit_wang(&record.sequence, k));
    }

    hashes
}

/// Variant 2: 2-bit + ntHash (scalar)
fn extract_kmers_nthash_scalar(sequences: &[SequenceRecord], k: usize) -> Vec<u64> {
    let mut hashes = Vec::new();

    for record in sequences {
        hashes.extend(kmer_2bit::extract_kmers_nthash_scalar(&record.sequence, k));
    }

    hashes
}

/// Variant 3: 2-bit + ntHash + NEON
fn extract_kmers_nthash_neon(sequences: &[SequenceRecord], k: usize) -> Vec<u64> {
    let mut hashes = Vec::new();

    for record in sequences {
        hashes.extend(kmer_2bit::extract_kmers_nthash_neon(&record.sequence, k));
    }

    hashes
}

/// Benchmark a single variant
fn benchmark_variant<F>(
    name: &str,
    sequences: &[SequenceRecord],
    k: usize,
    repetitions: usize,
    func: F,
) -> (f64, f64, usize)
where
    F: Fn(&[SequenceRecord], usize) -> Vec<u64>,
{
    let mut times = Vec::with_capacity(repetitions);

    // Warmup
    let _ = func(sequences, k);

    // Measure
    for _ in 0..repetitions {
        let start = Instant::now();
        let result = func(sequences, k);
        let elapsed = start.elapsed().as_secs_f64();
        times.push(elapsed);

        // Store result size from first run
        if times.len() == 1 {
            println!("  {} produced {} k-mer hashes", name, result.len());
        }
    }

    let mean = times.iter().sum::<f64>() / times.len() as f64;
    let min = times.iter().cloned().fold(f64::INFINITY, f64::min);
    let max = times.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let num_kmers = func(sequences, k).len();

    println!("  {}: {:.4}s ± {:.4}s (min={:.4}s, max={:.4}s)",
             name, mean, max - min, min, max);

    (mean, min, num_kmers)
}

fn main() {
    println!("===========================================");
    println!("Entry 035: K-mer 2-bit + ntHash Pilot");
    println!("===========================================\n");

    let k_values = vec![15, 21];
    let scales = vec![
        ("Small", 10_000, 150),
        ("Large", 100_000, 150),
    ];

    let repetitions = 3; // Pilot: N=3

    for k in k_values {
        println!("\n*** K = {} ***\n", k);

        for (scale_name, num_seqs, seq_len) in &scales {
            println!("Scale: {} ({} sequences × {} bp)", scale_name, num_seqs, seq_len);

            // Generate sequences
            let sequences = generate_sequences(*num_seqs, *seq_len);

            // Benchmark baseline
            let (baseline_mean, baseline_min, baseline_kmers) = benchmark_variant(
                "Baseline (ASCII + FNV-1a)",
                &sequences,
                k,
                repetitions,
                extract_kmers_baseline,
            );

            // Benchmark Variant 1: 2-bit + Wang
            let (wang_mean, wang_min, wang_kmers) = benchmark_variant(
                "Variant 1 (2-bit + Wang)",
                &sequences,
                k,
                repetitions,
                extract_kmers_2bit_wang,
            );

            // Benchmark Variant 2: ntHash scalar
            let (nthash_mean, nthash_min, nthash_kmers) = benchmark_variant(
                "Variant 2 (ntHash scalar)",
                &sequences,
                k,
                repetitions,
                extract_kmers_nthash_scalar,
            );

            // Benchmark Variant 3: ntHash NEON
            let (neon_mean, neon_min, neon_kmers) = benchmark_variant(
                "Variant 3 (ntHash NEON)",
                &sequences,
                k,
                repetitions,
                extract_kmers_nthash_neon,
            );

            // Verify all variants produce same number of k-mers
            assert_eq!(baseline_kmers, wang_kmers, "Wang produced different k-mer count!");
            assert_eq!(baseline_kmers, nthash_kmers, "ntHash scalar produced different k-mer count!");
            assert_eq!(baseline_kmers, neon_kmers, "ntHash NEON produced different k-mer count!");

            println!("\n  Speedups (vs baseline):");
            println!("    2-bit + Wang:    {:.2}×", baseline_mean / wang_mean);
            println!("    ntHash scalar:   {:.2}×", baseline_mean / nthash_mean);
            println!("    ntHash NEON:     {:.2}×", baseline_mean / neon_mean);

            println!();
        }
    }

    println!("\n===========================================");
    println!("Pilot Complete");
    println!("===========================================");
    println!("\nNext Steps:");
    println!("  - If any variant ≥5× speedup: Run full N=30 validation");
    println!("  - If 2-5× speedup: Consider implementation (moderate gain)");
    println!("  - If <2× speedup: Confirms Entry 034 findings");
}
