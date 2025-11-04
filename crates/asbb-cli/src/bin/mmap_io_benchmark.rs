/// Memory-Mapped I/O Benchmark
///
/// Tests whether memory-mapped files + APFS optimization can reduce I/O overhead
/// Compares: standard I/O vs mmap vs mmap+madvise
///
/// Question: Can mmap + sequential hints reduce I/O bottleneck?
/// Expected: 20-30% speedup from better prefetching

use std::fs::File;
use std::io::{self, Read};
use std::path::Path;
use std::time::Instant;
use memmap2::Mmap;

#[cfg(target_os = "macos")]
use libc::{madvise, MADV_SEQUENTIAL, MADV_WILLNEED};

/// Test 1: Standard file I/O (baseline)
fn benchmark_standard_io(path: &Path, reps: usize) -> io::Result<(f64, usize)> {
    let mut total_bytes = 0;
    let start = Instant::now();

    for _ in 0..reps {
        let mut file = File::open(path)?;
        let mut buffer = Vec::new();
        let bytes_read = file.read_to_end(&mut buffer)?;
        total_bytes += bytes_read;
    }

    let elapsed = start.elapsed().as_secs_f64();
    Ok((elapsed, total_bytes / reps))
}

/// Test 2: Memory-mapped I/O (no hints)
fn benchmark_mmap_basic(path: &Path, reps: usize) -> io::Result<(f64, usize)> {
    let mut total_bytes = 0;
    let start = Instant::now();

    for _ in 0..reps {
        let file = File::open(path)?;
        let mmap = unsafe { Mmap::map(&file)? };

        // Force access (otherwise lazy loading)
        let mut sum: u64 = 0;
        for &byte in mmap.iter() {
            sum = sum.wrapping_add(byte as u64);
        }

        total_bytes += mmap.len();

        // Prevent optimization
        if sum == 0 { println!("Unlikely"); }
    }

    let elapsed = start.elapsed().as_secs_f64();
    Ok((elapsed, total_bytes / reps))
}

/// Test 3: Memory-mapped I/O with madvise hints (APFS optimization)
#[cfg(target_os = "macos")]
fn benchmark_mmap_optimized(path: &Path, reps: usize) -> io::Result<(f64, usize)> {
    let mut total_bytes = 0;
    let start = Instant::now();

    for _ in 0..reps {
        let file = File::open(path)?;
        let mmap = unsafe { Mmap::map(&file)? };

        // Give kernel sequential access hints
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

        total_bytes += mmap.len();

        // Prevent optimization
        if sum == 0 { println!("Unlikely"); }
    }

    let elapsed = start.elapsed().as_secs_f64();
    Ok((elapsed, total_bytes / reps))
}

#[cfg(not(target_os = "macos"))]
fn benchmark_mmap_optimized(path: &Path, reps: usize) -> io::Result<(f64, usize)> {
    // Fallback for non-macOS (Linux has similar MADV_SEQUENTIAL)
    benchmark_mmap_basic(path, reps)
}

/// Test 4: Memory-mapped I/O with decompression (real-world scenario)
fn benchmark_mmap_with_decompression(path: &Path, reps: usize) -> io::Result<(f64, usize)> {
    use flate2::read::GzDecoder;

    let mut total_decompressed = 0;
    let start = Instant::now();

    for _ in 0..reps {
        let file = File::open(path)?;
        let mmap = unsafe { Mmap::map(&file)? };

        #[cfg(target_os = "macos")]
        unsafe {
            madvise(
                mmap.as_ptr() as *mut _,
                mmap.len(),
                MADV_SEQUENTIAL | MADV_WILLNEED,
            );
        }

        // Decompress from mmap
        let cursor = std::io::Cursor::new(&mmap[..]);
        let mut decoder = GzDecoder::new(cursor);
        let mut decompressed = Vec::new();
        decoder.read_to_end(&mut decompressed)?;

        total_decompressed += decompressed.len();
    }

    let elapsed = start.elapsed().as_secs_f64();
    Ok((elapsed, total_decompressed / reps))
}

fn format_throughput(bytes: usize, seconds: f64) -> String {
    let mb_per_sec = (bytes as f64 / seconds) / (1024.0 * 1024.0);
    format!("{:.2} MB/s", mb_per_sec)
}

fn main() -> io::Result<()> {
    println!("================================================================================");
    println!("MEMORY-MAPPED I/O BENCHMARK");
    println!("================================================================================");
    println!();
    println!("Testing whether mmap + APFS optimization can reduce I/O overhead");
    println!();

    // Test with compressed FASTQ file
    let test_file = Path::new("datasets/large_100k_150bp.fq.gz");

    if !test_file.exists() {
        println!("⚠️  Test file not found: {:?}", test_file);
        println!("   Please ensure the file exists.");
        return Ok(());
    }

    let file_size = std::fs::metadata(test_file)?.len();
    println!("Test file: {:?}", test_file);
    println!("File size: {:.2} MB", file_size as f64 / (1024.0 * 1024.0));
    println!();

    let reps = 10;
    println!("Repetitions: {}", reps);
    println!();

    println!("================================================================================");
    println!("Test 1: Standard File I/O (Baseline)");
    println!("================================================================================");

    let (time_std, bytes_std) = benchmark_standard_io(test_file, reps)?;
    let throughput_std = format_throughput(bytes_std, time_std / reps as f64);

    println!("Time: {:.3} ms (avg per iteration)", (time_std / reps as f64) * 1000.0);
    println!("Throughput: {}", throughput_std);
    println!();

    println!("================================================================================");
    println!("Test 2: Memory-Mapped I/O (Basic)");
    println!("================================================================================");

    let (time_mmap, bytes_mmap) = benchmark_mmap_basic(test_file, reps)?;
    let throughput_mmap = format_throughput(bytes_mmap, time_mmap / reps as f64);
    let speedup_mmap = time_std / time_mmap;

    println!("Time: {:.3} ms (avg per iteration)", (time_mmap / reps as f64) * 1000.0);
    println!("Throughput: {}", throughput_mmap);
    println!("Speedup vs standard I/O: {:.2}×", speedup_mmap);
    println!();

    println!("================================================================================");
    println!("Test 3: Memory-Mapped I/O + madvise (APFS Optimized)");
    println!("================================================================================");

    let (time_opt, bytes_opt) = benchmark_mmap_optimized(test_file, reps)?;
    let throughput_opt = format_throughput(bytes_opt, time_opt / reps as f64);
    let speedup_opt = time_std / time_opt;
    let speedup_vs_mmap = time_mmap / time_opt;

    println!("Time: {:.3} ms (avg per iteration)", (time_opt / reps as f64) * 1000.0);
    println!("Throughput: {}", throughput_opt);
    println!("Speedup vs standard I/O: {:.2}×", speedup_opt);
    println!("Speedup vs basic mmap: {:.2}×", speedup_vs_mmap);
    println!();

    println!("================================================================================");
    println!("Test 4: Memory-Mapped I/O + Decompression (Real-World)");
    println!("================================================================================");

    let (time_decomp, bytes_decomp) = benchmark_mmap_with_decompression(test_file, reps)?;
    let throughput_decomp = format_throughput(bytes_decomp, time_decomp / reps as f64);

    println!("Time: {:.3} ms (avg per iteration)", (time_decomp / reps as f64) * 1000.0);
    println!("Decompressed size: {:.2} MB", bytes_decomp as f64 / (1024.0 * 1024.0));
    println!("Throughput (decompressed): {}", throughput_decomp);
    println!();

    println!("================================================================================");
    println!("SUMMARY");
    println!("================================================================================");
    println!();
    println!("Standard I/O:          {} ({:.2}×)", throughput_std, 1.0);
    println!("mmap (basic):          {} ({:.2}×)", throughput_mmap, speedup_mmap);
    println!("mmap + madvise:        {} ({:.2}×)", throughput_opt, speedup_opt);
    println!();

    if speedup_opt > 1.15 {
        println!("✅ RECOMMENDATION: Use mmap + madvise (>{:.0}% improvement)", (speedup_opt - 1.0) * 100.0);
        println!("   APFS sequential hints provide measurable benefit");
    } else if speedup_opt > 1.05 {
        println!("⚠️  MARGINAL: mmap provides {:.0}% improvement", (speedup_opt - 1.0) * 100.0);
        println!("   Benefit exists but may not justify complexity");
    } else {
        println!("❌ NOT RECOMMENDED: No significant benefit from mmap");
        println!("   Standard I/O is sufficient");
    }
    println!();

    println!("================================================================================");
    println!("NEXT STEPS");
    println!("================================================================================");
    println!();

    if speedup_opt > 1.1 {
        println!("1. Combine with parallel bgzip (6.5× × {:.2}× = {:.2}× total)",
                 speedup_opt, 6.5 * speedup_opt);
        println!("2. Implement in biofast with feature flag");
        println!("3. Test on large files (>1GB) for validation");
    } else {
        println!("1. Skip mmap optimization (insufficient benefit)");
        println!("2. Focus on parallel bgzip (6.5× validated)");
        println!("3. Proceed to biofast implementation");
    }
    println!();

    Ok(())
}
