/// Memory-Mapped I/O Scale Benchmark
///
/// Tests whether mmap + APFS optimization scales across file sizes
/// Tests: 556 KB, 5.4 MB, 54 MB, 544 MB
///
/// Question: Does mmap benefit scale with file size?

use std::fs::File;
use std::io::{self, Read};
use std::path::Path;
use std::time::Instant;
use memmap2::Mmap;

#[cfg(target_os = "macos")]
use libc::{madvise, MADV_SEQUENTIAL, MADV_WILLNEED};

struct BenchmarkResult {
    method: String,
    file_size: u64,
    time_ms: f64,
    throughput_mbs: f64,
}

/// Standard file I/O (baseline)
fn benchmark_standard_io(path: &Path, reps: usize) -> io::Result<f64> {
    let start = Instant::now();

    for _ in 0..reps {
        let mut file = File::open(path)?;
        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer)?;
    }

    Ok(start.elapsed().as_secs_f64() / reps as f64)
}

/// Memory-mapped I/O with madvise hints
#[cfg(target_os = "macos")]
fn benchmark_mmap_optimized(path: &Path, reps: usize) -> io::Result<f64> {
    let start = Instant::now();

    for _ in 0..reps {
        let file = File::open(path)?;
        let mmap = unsafe { Mmap::map(&file)? };

        unsafe {
            madvise(
                mmap.as_ptr() as *mut _,
                mmap.len(),
                MADV_SEQUENTIAL | MADV_WILLNEED,
            );
        }

        // Force access
        let mut sum: u64 = 0;
        for &byte in mmap.iter() {
            sum = sum.wrapping_add(byte as u64);
        }

        if sum == 0 { println!("Unlikely"); }
    }

    Ok(start.elapsed().as_secs_f64() / reps as f64)
}

#[cfg(not(target_os = "macos"))]
fn benchmark_mmap_optimized(path: &Path, reps: usize) -> io::Result<f64> {
    benchmark_standard_io(path, reps)
}

fn run_scale_test(path: &Path, label: &str, reps: usize) -> io::Result<(BenchmarkResult, BenchmarkResult)> {
    let file_size = std::fs::metadata(path)?.len();

    println!("Testing: {} ({:.2} MB)", label, file_size as f64 / (1024.0 * 1024.0));

    // Standard I/O
    let time_std = benchmark_standard_io(path, reps)?;
    let throughput_std = (file_size as f64 / time_std) / (1024.0 * 1024.0);

    // mmap + madvise
    let time_mmap = benchmark_mmap_optimized(path, reps)?;
    let throughput_mmap = (file_size as f64 / time_mmap) / (1024.0 * 1024.0);

    let speedup = time_std / time_mmap;
    println!("  Standard I/O: {:.2} MB/s ({:.3} ms)", throughput_std, time_std * 1000.0);
    println!("  mmap+madvise: {:.2} MB/s ({:.3} ms) - {:.2}× speedup", throughput_mmap, time_mmap * 1000.0, speedup);
    println!();

    Ok((
        BenchmarkResult {
            method: "Standard I/O".to_string(),
            file_size,
            time_ms: time_std * 1000.0,
            throughput_mbs: throughput_std,
        },
        BenchmarkResult {
            method: "mmap+madvise".to_string(),
            file_size,
            time_ms: time_mmap * 1000.0,
            throughput_mbs: throughput_mmap,
        },
    ))
}

fn main() -> io::Result<()> {
    println!("================================================================================");
    println!("MEMORY-MAPPED I/O SCALE BENCHMARK");
    println!("================================================================================");
    println!();
    println!("Testing whether mmap + APFS optimization scales across file sizes");
    println!();

    let test_files = vec![
        ("datasets/medium_10k_150bp.fq.gz", "Medium (10K sequences)", 30),
        ("datasets/large_100k_150bp.fq.gz", "Large (100K sequences)", 10),
        ("datasets/vlarge_1m_150bp.fq.gz", "VeryLarge (1M sequences)", 5),
        ("datasets/huge_10m_150bp.fq.gz", "Huge (10M sequences)", 3),
    ];

    let mut all_results_std = Vec::new();
    let mut all_results_mmap = Vec::new();

    for (path_str, label, reps) in test_files {
        let path = Path::new(path_str);

        if !path.exists() {
            println!("⚠️  Skipping {}: File not found", label);
            println!();
            continue;
        }

        let (result_std, result_mmap) = run_scale_test(path, label, reps)?;
        all_results_std.push(result_std);
        all_results_mmap.push(result_mmap);
    }

    println!("================================================================================");
    println!("SUMMARY: mmap+madvise vs Standard I/O");
    println!("================================================================================");
    println!();
    println!("{:<20} {:>12} {:>15} {:>15} {:>10}",
             "File Size", "Std I/O", "mmap+madvise", "Speedup", "Benefit");
    println!("{}", "-".repeat(75));

    for (std, mmap) in all_results_std.iter().zip(all_results_mmap.iter()) {
        let speedup = std.throughput_mbs / mmap.throughput_mbs;
        let size_mb = std.file_size as f64 / (1024.0 * 1024.0);

        println!("{:<20} {:>10.2} MB/s {:>10.2} MB/s {:>9.2}× {:>9}",
                 format!("{:.0} MB", size_mb),
                 std.throughput_mbs,
                 mmap.throughput_mbs,
                 1.0 / speedup,
                 if speedup < 0.8 { "✅ Good" } else if speedup < 0.95 { "⚠️  OK" } else { "❌ None" });
    }

    println!();
    println!("================================================================================");
    println!("ANALYSIS");
    println!("================================================================================");
    println!();

    // Calculate average speedup
    let avg_speedup: f64 = all_results_std.iter()
        .zip(all_results_mmap.iter())
        .map(|(std, mmap)| mmap.throughput_mbs / std.throughput_mbs)
        .sum::<f64>() / all_results_std.len() as f64;

    println!("Average speedup: {:.2}×", avg_speedup);
    println!();

    if avg_speedup > 1.5 {
        println!("✅ STRONG RECOMMENDATION: Use mmap+madvise");
        println!("   Consistent {:.0}%+ improvement across all file sizes", (avg_speedup - 1.0) * 100.0);
        println!();
        println!("Combined with parallel bgzip:");
        println!("  - Parallel bgzip: 6.5×");
        println!("  - mmap+madvise: {:.2}×", avg_speedup);
        println!("  - Total I/O speedup: {:.1}×", 6.5 * avg_speedup);
        println!();
        println!("Impact on E2E performance:");
        println!("  - Original I/O bottleneck: 264-352× slower than compute");
        println!("  - With optimizations: {:.0}-{:.0}× slower than compute",
                 264.0 / (6.5 * avg_speedup), 352.0 / (6.5 * avg_speedup));
        println!("  - Improvement: {:.1}× better I/O performance", 6.5 * avg_speedup);
    } else if avg_speedup > 1.2 {
        println!("⚠️  MODERATE RECOMMENDATION: Consider mmap+madvise");
        println!("   {:.0}% improvement may justify implementation", (avg_speedup - 1.0) * 100.0);
    } else {
        println!("❌ NOT RECOMMENDED: Minimal benefit from mmap");
        println!("   <20% improvement doesn't justify additional complexity");
    }

    println!();
    println!("================================================================================");
    println!("NEXT STEPS");
    println!("================================================================================");
    println!();

    if avg_speedup > 1.2 {
        println!("1. ✅ Integrate mmap+madvise into biofast (Week 1-2)");
        println!("2. ✅ Combine with parallel bgzip (multiplicative benefit)");
        println!("3. ✅ Add feature flag for platforms without madvise");
        println!("4. ✅ Document in I/O optimization findings");
    } else {
        println!("1. ❌ Skip mmap optimization (insufficient benefit)");
        println!("2. ✅ Use parallel bgzip only (6.5× validated)");
        println!("3. ✅ Focus on biofast core implementation");
    }

    println!();

    Ok(())
}
