/// Parallel bgzip Decompression Benchmark
///
/// Tests if parallel decompression of bgzip blocks can overcome I/O bottleneck.
///
/// bgzip format:
/// - Standard gzip with extra field (BSIZE) indicating block size
/// - Each block is independently decompressible
/// - Typical block size: 64KB compressed → ~64KB uncompressed
///
/// Hypothesis: Parallel decompression of N blocks on CPU (or GPU) could provide
/// N× speedup over sequential decompression.

use std::fs::File;
use std::io::{self, BufReader, Read, Seek, SeekFrom};
use std::path::Path;
use std::time::Instant;
use flate2::read::GzDecoder;
use rayon::prelude::*;

/// Represents a single bgzip block
#[derive(Debug, Clone)]
struct BgzipBlock {
    offset: u64,      // File offset of this block
    csize: u16,       // Compressed size (from BSIZE field)
    data: Vec<u8>,    // Compressed data
}

/// Parse bgzip file and extract all block boundaries
fn parse_bgzip_blocks(path: &Path) -> io::Result<Vec<BgzipBlock>> {
    let mut file = File::open(path)?;
    let file_size = file.metadata()?.len();
    let mut blocks = Vec::new();

    let mut offset = 0u64;

    while offset < file_size {
        file.seek(SeekFrom::Start(offset))?;

        // Read gzip header (minimum 10 bytes)
        let mut header = [0u8; 10];
        if file.read_exact(&mut header).is_err() {
            break;  // End of file
        }

        // Check gzip magic number
        if header[0] != 0x1f || header[1] != 0x8b {
            eprintln!("Invalid gzip magic at offset {}", offset);
            break;
        }

        // Check if this is bgzip (has extra field)
        let flags = header[3];
        let has_extra = (flags & 0x04) != 0;

        if !has_extra {
            eprintln!("Warning: gzip block at {} doesn't have extra field (not bgzip?)", offset);
            // Try to skip this block by reading until we find next header
            offset += 1;
            continue;
        }

        // Read extra field length
        let mut xlen_bytes = [0u8; 2];
        file.read_exact(&mut xlen_bytes)?;
        let xlen = u16::from_le_bytes(xlen_bytes);

        // Read extra subfields to find BSIZE
        let mut bsize: Option<u16> = None;
        let mut extra_read = 0;

        while extra_read < xlen {
            let mut subfield_header = [0u8; 4];
            if file.read_exact(&mut subfield_header).is_err() {
                break;
            }

            let si1 = subfield_header[0];
            let si2 = subfield_header[1];
            let slen = u16::from_le_bytes([subfield_header[2], subfield_header[3]]);

            extra_read += 4;

            // Check if this is the BC subfield (BSIZE)
            if si1 == b'B' && si2 == b'C' && slen == 2 {
                let mut bsize_bytes = [0u8; 2];
                file.read_exact(&mut bsize_bytes)?;
                bsize = Some(u16::from_le_bytes(bsize_bytes));
                extra_read += 2;
            } else {
                // Skip this subfield
                file.seek(SeekFrom::Current(slen as i64))?;
                extra_read += slen;
            }
        }

        let bsize = match bsize {
            Some(b) => b,
            None => {
                eprintln!("Warning: No BSIZE found in extra field at offset {}", offset);
                offset += 1;
                continue;
            }
        };

        // BSIZE is the total block size minus 1
        let block_size = (bsize + 1) as u64;

        // Read entire block
        file.seek(SeekFrom::Start(offset))?;
        let mut block_data = vec![0u8; block_size as usize];
        file.read_exact(&mut block_data)?;

        blocks.push(BgzipBlock {
            offset,
            csize: bsize,
            data: block_data,
        });

        offset += block_size;
    }

    Ok(blocks)
}

/// Decompress a single bgzip block
fn decompress_block(block: &BgzipBlock) -> io::Result<Vec<u8>> {
    let cursor = io::Cursor::new(&block.data);
    let mut decoder = GzDecoder::new(cursor);
    let mut decompressed = Vec::new();
    decoder.read_to_end(&mut decompressed)?;
    Ok(decompressed)
}

/// Benchmark sequential decompression
fn benchmark_sequential(blocks: &[BgzipBlock], reps: usize) -> io::Result<(f64, usize)> {
    let start = Instant::now();
    let mut total_bytes = 0;

    for _ in 0..reps {
        for block in blocks {
            let decompressed = decompress_block(block)?;
            total_bytes += decompressed.len();
        }
    }

    let elapsed = start.elapsed().as_secs_f64();
    Ok((elapsed, total_bytes / reps))
}

/// Benchmark parallel decompression using Rayon
fn benchmark_parallel(blocks: &[BgzipBlock], reps: usize) -> io::Result<(f64, usize)> {
    let start = Instant::now();
    let mut total_bytes = 0;

    for _ in 0..reps {
        let results: Vec<_> = blocks
            .par_iter()
            .map(|block| decompress_block(block))
            .collect::<io::Result<Vec<_>>>()?;

        total_bytes = results.iter().map(|v| v.len()).sum();
    }

    let elapsed = start.elapsed().as_secs_f64();
    Ok((elapsed, total_bytes))
}

fn main() -> io::Result<()> {
    println!("================================================================================");
    println!("PARALLEL BGZIP DECOMPRESSION BENCHMARK");
    println!("================================================================================");
    println!();
    println!("Testing hypothesis: Parallel decompression can overcome I/O bottleneck");
    println!();

    // Use bgzip-compressed file (try both sizes)
    let test_files = vec![
        ("Medium (10K seqs)", Path::new("datasets/medium_10k_150bp.fq.bgz")),
        ("Large (100K seqs)", Path::new("datasets/large_100k_150bp.fq.bgz")),
    ];

    for (label, test_file) in &test_files {
        println!("================================================================================");
        println!("Testing: {}", label);
        println!("================================================================================");
        println!();

        if !test_file.exists() {
            eprintln!("Warning: Test file not found: {:?}", test_file);
            eprintln!("Skipping...");
            println!();
            continue;
        }

        println!("Parsing bgzip blocks from {:?}...", test_file);
        let blocks = parse_bgzip_blocks(test_file)?;

        if blocks.is_empty() {
            eprintln!("Error: No valid bgzip blocks found!");
            eprintln!("Note: Regular gzip files are not bgzip (no block boundaries)");
            eprintln!();
            continue;
        }

        println!("Found {} bgzip blocks", blocks.len());

        let total_compressed: u64 = blocks.iter().map(|b| b.data.len() as u64).sum();
        println!("Total compressed size: {:.2} MB", total_compressed as f64 / 1_000_000.0);

        let avg_block_size = total_compressed / blocks.len() as u64;
        println!("Average block size: {:.2} KB", avg_block_size as f64 / 1_000.0);
        println!();

        // Determine repetitions based on file size
        let reps = if blocks.len() < 100 {
            30
        } else if blocks.len() < 1000 {
            10
        } else {
            3
        };

        println!("Running benchmark with {} repetitions...", reps);
        println!();

        // Benchmark sequential
        print!("Sequential decompression... ");
        io::Write::flush(&mut io::stdout())?;
        let (seq_time, seq_bytes) = benchmark_sequential(&blocks, reps)?;
        let seq_throughput = (seq_bytes * reps) as f64 / seq_time / 1_000_000.0;
        println!("{:.2} MB/s ({:.3}s for {} reps)", seq_throughput, seq_time, reps);

        // Benchmark parallel
        print!("Parallel decompression... ");
        io::Write::flush(&mut io::stdout())?;
        let (par_time, par_bytes) = benchmark_parallel(&blocks, reps)?;
        let par_throughput = (par_bytes * reps) as f64 / par_time / 1_000_000.0;
        println!("{:.2} MB/s ({:.3}s for {} reps)", par_throughput, par_time, reps);

        println!();
        println!("Sequential: {:.2} MB/s", seq_throughput);
        println!("Parallel:   {:.2} MB/s", par_throughput);
        println!("Speedup:    {:.2}×", par_throughput / seq_throughput);
        println!();

        if par_throughput > seq_throughput * 1.5 {
            println!("✅ Parallel decompression shows significant benefit!");
            println!("   GPU implementation would likely provide even more speedup.");
        } else {
            println!("⚠️  Parallel decompression shows limited benefit.");
            println!("   This may be due to:");
            println!("   - Small number of blocks (limited parallelism)");
            println!("   - Memory bandwidth bottleneck");
            println!("   - File is not bgzip (regular gzip is sequential)");
        }
        println!();
    }

    println!();
    println!("================================================================================");
    println!("SUMMARY");
    println!("================================================================================");
    println!();
    println!("Note: Standard gzip files (.gz) are NOT bgzip!");
    println!("bgzip is used in genomics for BAM/CRAM files.");
    println!("To create bgzip file: bgzip -c input.fastq > output.fastq.gz");

    Ok(())
}
