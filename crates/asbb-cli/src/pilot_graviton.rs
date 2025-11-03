#!/usr/bin/env rust
//! Graviton Portability Pilot
//!
//! Cross-platform validation: AWS Graviton 3 vs Mac M4
//! Tests ARM NEON portability across platforms
//!
//! Lab Notebook: Entry 021
//! Operations: 5 (base_counting, gc_content, quality_aggregation, adapter_trimming, kmer_counting)
//! Configurations: 3 (naive, neon, neon_4t)
//! Scales: 3 (Small 1K, Medium 10K, Large 100K)
//! Total: 45 experiments

use bio::io::fastq::{Reader, Record};
use std::fs::File;
use std::io::{BufWriter, Write};
use std::time::{Duration, Instant};
use std::path::Path;
use chrono::Local;

// Operation implementations
mod operations {
    use bio::io::fastq::Record;

    pub fn base_counting_naive(records: &[Record]) -> (usize, usize, usize, usize) {
        let mut counts = (0, 0, 0, 0); // A, C, G, T
        for record in records {
            for &base in record.seq() {
                match base {
                    b'A' | b'a' => counts.0 += 1,
                    b'C' | b'c' => counts.1 += 1,
                    b'G' | b'g' => counts.2 += 1,
                    b'T' | b't' => counts.3 += 1,
                    _ => {}
                }
            }
        }
        counts
    }

    #[cfg(target_arch = "aarch64")]
    pub fn base_counting_neon(records: &[Record]) -> (usize, usize, usize, usize) {
        use std::arch::aarch64::*;

        let mut counts = (0usize, 0usize, 0usize, 0usize);

        unsafe {
            let mut a_total = vdupq_n_u32(0);
            let mut c_total = vdupq_n_u32(0);
            let mut g_total = vdupq_n_u32(0);
            let mut t_total = vdupq_n_u32(0);

            for record in records {
                let seq = record.seq();
                let chunks = seq.chunks_exact(16);
                let remainder = chunks.remainder();

                for chunk in chunks {
                    let data = vld1q_u8(chunk.as_ptr());

                    let a_match = vceqq_u8(data, vdupq_n_u8(b'A'));
                    let c_match = vceqq_u8(data, vdupq_n_u8(b'C'));
                    let g_match = vceqq_u8(data, vdupq_n_u8(b'G'));
                    let t_match = vceqq_u8(data, vdupq_n_u8(b'T'));

                    a_total = vaddq_u32(a_total, vpaddlq_u16(vpaddlq_u8(a_match)));
                    c_total = vaddq_u32(c_total, vpaddlq_u16(vpaddlq_u8(c_match)));
                    g_total = vaddq_u32(g_total, vpaddlq_u16(vpaddlq_u8(g_match)));
                    t_total = vaddq_u32(t_total, vpaddlq_u16(vpaddlq_u8(t_match)));
                }

                // Handle remainder
                for &base in remainder {
                    match base {
                        b'A' => counts.0 += 1,
                        b'C' => counts.1 += 1,
                        b'G' => counts.2 += 1,
                        b'T' => counts.3 += 1,
                        _ => {}
                    }
                }
            }

            // Sum NEON accumulators
            counts.0 += vaddvq_u32(a_total) as usize;
            counts.1 += vaddvq_u32(c_total) as usize;
            counts.2 += vaddvq_u32(g_total) as usize;
            counts.3 += vaddvq_u32(t_total) as usize;
        }

        counts
    }

    pub fn gc_content_naive(records: &[Record]) -> f64 {
        let mut gc_count = 0usize;
        let mut total = 0usize;

        for record in records {
            for &base in record.seq() {
                total += 1;
                if base == b'G' || base == b'C' {
                    gc_count += 1;
                }
            }
        }

        if total > 0 {
            (gc_count as f64) / (total as f64)
        } else {
            0.0
        }
    }

    #[cfg(target_arch = "aarch64")]
    pub fn gc_content_neon(records: &[Record]) -> f64 {
        use std::arch::aarch64::*;

        let mut gc_count = 0usize;
        let mut total = 0usize;

        unsafe {
            let mut gc_total = vdupq_n_u32(0);

            for record in records {
                let seq = record.seq();
                total += seq.len();

                let chunks = seq.chunks_exact(16);
                let remainder = chunks.remainder();

                for chunk in chunks {
                    let data = vld1q_u8(chunk.as_ptr());
                    let g_match = vceqq_u8(data, vdupq_n_u8(b'G'));
                    let c_match = vceqq_u8(data, vdupq_n_u8(b'C'));
                    let gc_match = vorrq_u8(g_match, c_match);
                    gc_total = vaddq_u32(gc_total, vpaddlq_u16(vpaddlq_u8(gc_match)));
                }

                for &base in remainder {
                    if base == b'G' || base == b'C' {
                        gc_count += 1;
                    }
                }
            }

            gc_count += vaddvq_u32(gc_total) as usize;
        }

        if total > 0 {
            (gc_count as f64) / (total as f64)
        } else {
            0.0
        }
    }

    pub fn quality_aggregation_naive(records: &[Record]) -> (f64, f64, u8, u8) {
        let mut sum = 0u64;
        let mut count = 0usize;
        let mut min_q = u8::MAX;
        let mut max_q = u8::MIN;

        for record in records {
            for &q in record.qual() {
                let quality = q.saturating_sub(33);
                sum += quality as u64;
                count += 1;
                min_q = min_q.min(quality);
                max_q = max_q.max(quality);
            }
        }

        let mean = if count > 0 { sum as f64 / count as f64 } else { 0.0 };
        (mean, 0.0, min_q, max_q)
    }

    #[cfg(target_arch = "aarch64")]
    pub fn quality_aggregation_neon(records: &[Record]) -> (f64, f64, u8, u8) {
        use std::arch::aarch64::*;

        let mut sum = 0u64;
        let mut count = 0usize;
        let mut min_q = u8::MAX;
        let mut max_q = u8::MIN;

        unsafe {
            let offset = vdupq_n_u8(33);
            let mut sum_vec = vdupq_n_u32(0);
            let mut min_vec = vdupq_n_u8(255);
            let mut max_vec = vdupq_n_u8(0);

            for record in records {
                let qual = record.qual();
                count += qual.len();

                let chunks = qual.chunks_exact(16);
                let remainder = chunks.remainder();

                for chunk in chunks {
                    let data = vld1q_u8(chunk.as_ptr());
                    let quality = vsubq_u8(data, offset);

                    sum_vec = vaddq_u32(sum_vec, vpaddlq_u16(vpaddlq_u8(quality)));
                    min_vec = vminq_u8(min_vec, quality);
                    max_vec = vmaxq_u8(max_vec, quality);
                }

                for &q in remainder {
                    let quality = q.saturating_sub(33);
                    sum += quality as u64;
                    min_q = min_q.min(quality);
                    max_q = max_q.max(quality);
                }
            }

            sum += vaddvq_u32(sum_vec) as u64;
            min_q = min_q.min(vminvq_u8(min_vec));
            max_q = max_q.max(vmaxvq_u8(max_vec));
        }

        let mean = if count > 0 { sum as f64 / count as f64 } else { 0.0 };
        (mean, 0.0, min_q, max_q)
    }

    pub fn adapter_trimming_naive(records: &[Record], adapter: &[u8]) -> usize {
        let mut trimmed = 0usize;

        for record in records {
            let seq = record.seq();
            if seq.len() >= adapter.len() {
                if seq.windows(adapter.len()).any(|window| window == adapter) {
                    trimmed += 1;
                }
            }
        }

        trimmed
    }

    #[cfg(target_arch = "aarch64")]
    pub fn adapter_trimming_neon(records: &[Record], adapter: &[u8]) -> usize {
        // For now, use naive (NEON string matching is complex)
        adapter_trimming_naive(records, adapter)
    }

    pub fn kmer_counting_naive(records: &[Record], k: usize) -> usize {
        use std::collections::HashSet;

        let mut kmers = HashSet::new();

        for record in records {
            let seq = record.seq();
            if seq.len() >= k {
                for window in seq.windows(k) {
                    kmers.insert(window.to_vec());
                }
            }
        }

        kmers.len()
    }

    #[cfg(target_arch = "aarch64")]
    pub fn kmer_counting_neon(records: &[Record], k: usize) -> usize {
        // For now, use naive (NEON hashing is complex)
        kmer_counting_naive(records, k)
    }
}

use operations::*;

fn generate_synthetic_sequences(num_sequences: usize, _scale_name: &str) -> Vec<Record> {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    let mut sequences = Vec::with_capacity(num_sequences);

    for i in 0..num_sequences {
        let id = format!("seq_{}", i);
        let seq_len = 150;

        let seq: Vec<u8> = (0..seq_len)
            .map(|_| match rng.gen_range(0..4) {
                0 => b'A',
                1 => b'C',
                2 => b'G',
                _ => b'T',
            })
            .collect();

        let qual: Vec<u8> = (0..seq_len)
            .map(|_| rng.gen_range(20..40) + 33)
            .collect();

        sequences.push(Record::with_attrs(&id, None, &seq, &qual));
    }

    sequences
}

fn run_experiment(
    operation: &str,
    config: &str,
    scale: &str,
    num_sequences: usize,
    loop_duration: Duration,
) -> (usize, usize, f64) {
    eprintln!("  Running: {} / {} / {} ({} sequences)", operation, config, scale, num_sequences);

    // Generate synthetic data
    let sequences = generate_synthetic_sequences(num_sequences, scale);

    // Warmup
    match (operation, config) {
        ("base_counting", "naive") => { base_counting_naive(&sequences); },
        ("base_counting", "neon") => {
            #[cfg(target_arch = "aarch64")]
            { base_counting_neon(&sequences); }
        },
        ("gc_content", "naive") => { gc_content_naive(&sequences); },
        ("gc_content", "neon") => {
            #[cfg(target_arch = "aarch64")]
            { gc_content_neon(&sequences); }
        },
        ("quality_aggregation", "naive") => { quality_aggregation_naive(&sequences); },
        ("quality_aggregation", "neon") => {
            #[cfg(target_arch = "aarch64")]
            { quality_aggregation_neon(&sequences); }
        },
        ("adapter_trimming", "naive") => { adapter_trimming_naive(&sequences, b"AGATCGGAAGAG"); },
        ("adapter_trimming", "neon") => {
            #[cfg(target_arch = "aarch64")]
            { adapter_trimming_neon(&sequences, b"AGATCGGAAGAG"); }
        },
        ("kmer_counting", "naive") => { kmer_counting_naive(&sequences, 5); },
        ("kmer_counting", "neon") => {
            #[cfg(target_arch = "aarch64")]
            { kmer_counting_neon(&sequences, 5); }
        },
        _ => unreachable!(),
    }

    // Main loop
    let start = Instant::now();
    let mut iterations = 0usize;

    if config.contains("4t") {
        // Parallel execution with 4 threads
        use std::sync::Arc;
        use rayon::prelude::*;

        let seq_arc = Arc::new(sequences);
        rayon::ThreadPoolBuilder::new().num_threads(4).build().unwrap().install(|| {
            while start.elapsed() < loop_duration {
                let seq_clone = Arc::clone(&seq_arc);
                let _result = match operation {
                    "base_counting" => {
                        #[cfg(target_arch = "aarch64")]
                        { base_counting_neon(&seq_clone) }
                        #[cfg(not(target_arch = "aarch64"))]
                        { base_counting_naive(&seq_clone) }
                    },
                    "gc_content" => {
                        #[cfg(target_arch = "aarch64")]
                        { gc_content_neon(&seq_clone); 0 }
                        #[cfg(not(target_arch = "aarch64"))]
                        { gc_content_naive(&seq_clone); 0 }
                    },
                    "quality_aggregation" => {
                        #[cfg(target_arch = "aarch64")]
                        { quality_aggregation_neon(&seq_clone); 0 }
                        #[cfg(not(target_arch = "aarch64"))]
                        { quality_aggregation_naive(&seq_clone); 0 }
                    },
                    "adapter_trimming" => {
                        #[cfg(target_arch = "aarch64")]
                        { adapter_trimming_neon(&seq_clone, b"AGATCGGAAGAG") }
                        #[cfg(not(target_arch = "aarch64"))]
                        { adapter_trimming_naive(&seq_clone, b"AGATCGGAAGAG") }
                    },
                    "kmer_counting" => {
                        #[cfg(target_arch = "aarch64")]
                        { kmer_counting_neon(&seq_clone, 5) }
                        #[cfg(not(target_arch = "aarch64"))]
                        { kmer_counting_naive(&seq_clone, 5) }
                    },
                    _ => 0,
                };
                iterations += 1;
            }
        });
    } else {
        // Single-threaded execution
        while start.elapsed() < loop_duration {
            let _result = match (operation, config) {
                ("base_counting", "naive") => { base_counting_naive(&sequences); 0 },
                ("base_counting", "neon") => {
                    #[cfg(target_arch = "aarch64")]
                    { base_counting_neon(&sequences); 0 }
                    #[cfg(not(target_arch = "aarch64"))]
                    { base_counting_naive(&sequences); 0 }
                },
                ("gc_content", "naive") => { gc_content_naive(&sequences); 0 },
                ("gc_content", "neon") => {
                    #[cfg(target_arch = "aarch64")]
                    { gc_content_neon(&sequences); 0 }
                    #[cfg(not(target_arch = "aarch64"))]
                    { gc_content_naive(&sequences); 0 }
                },
                ("quality_aggregation", "naive") => { quality_aggregation_naive(&sequences); 0 },
                ("quality_aggregation", "neon") => {
                    #[cfg(target_arch = "aarch64")]
                    { quality_aggregation_neon(&sequences); 0 }
                    #[cfg(not(target_arch = "aarch64"))]
                    { quality_aggregation_naive(&sequences); 0 }
                },
                ("adapter_trimming", "naive") => adapter_trimming_naive(&sequences, b"AGATCGGAAGAG"),
                ("adapter_trimming", "neon") => {
                    #[cfg(target_arch = "aarch64")]
                    { adapter_trimming_neon(&sequences, b"AGATCGGAAGAG") }
                    #[cfg(not(target_arch = "aarch64"))]
                    { adapter_trimming_naive(&sequences, b"AGATCGGAAGAG") }
                },
                ("kmer_counting", "naive") => kmer_counting_naive(&sequences, 5),
                ("kmer_counting", "neon") => {
                    #[cfg(target_arch = "aarch64")]
                    { kmer_counting_neon(&sequences, 5) }
                    #[cfg(not(target_arch = "aarch64"))]
                    { kmer_counting_naive(&sequences, 5) }
                },
                _ => 0,
            };
            iterations += 1;
        }
    }

    let elapsed = start.elapsed();
    let sequences_processed = iterations * num_sequences;
    let throughput = sequences_processed as f64 / elapsed.as_secs_f64();

    (iterations, sequences_processed, throughput)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    eprintln!("=== Graviton Portability Pilot ===");
    eprintln!("Lab Notebook: Entry 021");
    eprintln!("Platform: AWS Graviton 3 (c7g.xlarge)");
    eprintln!("Operations: 5 | Configs: 3 | Scales: 3");
    eprintln!("Total experiments: 45");
    eprintln!();

    // Create results directory
    let results_dir = Path::new("results/cross_platform_graviton");
    std::fs::create_dir_all(results_dir)?;

    // Generate timestamp
    let timestamp = Local::now().format("%Y%m%d_%H%M%S").to_string();
    let csv_path = results_dir.join(format!("graviton_raw_{}.csv", timestamp));

    // Open CSV file
    let csv_file = File::create(&csv_path)?;
    let mut csv_writer = BufWriter::new(csv_file);

    // Write CSV header
    writeln!(csv_writer, "operation,config,scale,num_sequences,loop_duration_s,iterations,sequences_processed,throughput_seqs_per_sec,timestamp")?;

    // Experiment parameters
    let operations = vec![
        "base_counting",
        "gc_content",
        "quality_aggregation",
        "adapter_trimming",
        "kmer_counting",
    ];

    let configs = vec!["naive", "neon", "neon_4t"];

    let scales = vec![
        ("Small", 1_000),
        ("Medium", 10_000),
        ("Large", 100_000),
    ];

    let loop_duration = Duration::from_secs(60);

    eprintln!("Starting in 3 seconds...");
    std::thread::sleep(Duration::from_secs(3));
    eprintln!();

    let total_experiments = operations.len() * configs.len() * scales.len();
    let mut experiment_num = 0;

    for operation in &operations {
        for config in &configs {
            for (scale_name, num_sequences) in &scales {
                experiment_num += 1;
                eprintln!("Experiment {}/{}: {} / {} / {}",
                    experiment_num, total_experiments, operation, config, scale_name);

                let (iterations, sequences_processed, throughput) = run_experiment(
                    operation,
                    config,
                    scale_name,
                    *num_sequences,
                    loop_duration,
                );

                let experiment_timestamp = Local::now().format("%Y-%m-%dT%H:%M:%S").to_string();

                writeln!(
                    csv_writer,
                    "{},{},{},{},{:.3},{},{},{:.2},{}",
                    operation,
                    config,
                    scale_name,
                    num_sequences,
                    loop_duration.as_secs_f64(),
                    iterations,
                    sequences_processed,
                    throughput,
                    experiment_timestamp
                )?;
                csv_writer.flush()?;

                eprintln!("  → {} iterations, {:.0} seqs/sec", iterations, throughput);
                eprintln!();

                // Cooldown
                std::thread::sleep(Duration::from_secs(5));
            }
        }
    }

    eprintln!("✅ All experiments complete!");
    eprintln!("Results: {}", csv_path.display());

    Ok(())
}
