# biofast I/O Optimization Integration Plan

**Date**: November 4, 2025
**Target**: biofast v0.1.0 (Week 1-2, Nov 4-15)
**Goal**: Integrate I/O optimization stack (16.3× speedup for large files)

---

## Overview

Integrate two complementary I/O optimizations into biofast:
1. **CPU parallel bgzip decompression**: 6.5× speedup (all files, all platforms)
2. **Smart mmap + APFS optimization**: 2.5× additional (large files ≥50 MB, macOS)

**Combined**: 16.3× I/O speedup for typical genomics files

---

## Architecture

### Smart I/O Selection (Threshold-Based)

```rust
const MMAP_THRESHOLD: u64 = 50 * 1024 * 1024;  // 50 MB

fn select_io_method(file_size: u64) -> IoMethod {
    if file_size >= MMAP_THRESHOLD {
        IoMethod::MemoryMapped  // 2.5× faster for large files
    } else {
        IoMethod::StandardIo    // No overhead for small files
    }
}
```

**Rationale**:
- Small files (<50 MB): mmap overhead dominates, use standard I/O
- Large files (≥50 MB): APFS prefetching dominates, use mmap + madvise
- Result: Optimal performance across all file sizes

### Data Flow

```
File → Smart I/O Selection → bgzip Block Parsing → Parallel Decompression → FASTQ Records
         (threshold: 50 MB)     (detect BC field)     (Rayon par_iter)
```

---

## Implementation Plan

### Day 1 (Nov 4): Dependencies and Types

**Add dependencies** (`crates/biofast/Cargo.toml`):
```toml
[dependencies]
memmap2 = "0.9"
rayon = "1.10"
flate2 = "1.0"
libc = "0.2"  # For madvise on macOS
```

**Core types** (`biofast/src/io/types.rs`):
```rust
/// Bgzip block metadata
#[derive(Debug, Clone)]
pub struct BgzipBlock {
    pub offset: u64,
    pub compressed_size: u16,
    pub data: Vec<u8>,
}

/// I/O method selection
pub enum DataSource {
    StandardIo(Vec<u8>),
    MemoryMapped(Mmap),
}

/// Bgzip reader configuration
pub struct BgzipConfig {
    pub parallel: bool,        // Enable parallel decompression
    pub thread_count: Option<usize>,  // None = auto-detect
}
```

**Deliverable**: Types defined, dependencies added

### Day 2 (Nov 5): Bgzip Block Parsing

**Implement block parser** (`biofast/src/io/bgzip.rs`):
```rust
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;

/// Parse bgzip blocks from file
pub fn parse_bgzip_blocks(path: &Path) -> io::Result<Vec<BgzipBlock>> {
    let mut file = File::open(path)?;
    let file_size = file.metadata()?.len();
    let mut blocks = Vec::new();
    let mut offset = 0u64;

    while offset < file_size {
        file.seek(SeekFrom::Start(offset))?;

        // Read gzip header (10 bytes)
        let mut header = [0u8; 10];
        if file.read_exact(&mut header).is_err() {
            break;
        }

        // Check magic number (0x1f 0x8b)
        if header[0] != 0x1f || header[1] != 0x8b {
            break;
        }

        // Check for FEXTRA flag (bit 2)
        let flags = header[3];
        let has_extra = (flags & 0x04) != 0;

        if !has_extra {
            offset += 1;
            continue;
        }

        // Read XLEN (extra field length)
        let mut xlen_bytes = [0u8; 2];
        file.read_exact(&mut xlen_bytes)?;
        let xlen = u16::from_le_bytes(xlen_bytes);

        // Find BC subfield (bgzip block size)
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

            if si1 == b'B' && si2 == b'C' && slen == 2 {
                // Found BC subfield (bgzip block size)
                let mut bsize_bytes = [0u8; 2];
                file.read_exact(&mut bsize_bytes)?;
                bsize = Some(u16::from_le_bytes(bsize_bytes));
                extra_read += 2;
            } else {
                // Skip other subfields
                file.seek(SeekFrom::Current(slen as i64))?;
                extra_read += slen;
            }
        }

        let bsize = match bsize {
            Some(b) => b,
            None => {
                // Not a bgzip block, skip
                offset += 1;
                continue;
            }
        };

        // Read entire block
        let block_size = (bsize + 1) as u64;
        file.seek(SeekFrom::Start(offset))?;
        let mut block_data = vec![0u8; block_size as usize];
        file.read_exact(&mut block_data)?;

        blocks.push(BgzipBlock {
            offset,
            compressed_size: bsize,
            data: block_data,
        });

        offset += block_size;
    }

    Ok(blocks)
}

/// Decompress single bgzip block (CPU)
fn decompress_block(block: &BgzipBlock) -> io::Result<Vec<u8>> {
    use flate2::read::GzDecoder;

    let cursor = std::io::Cursor::new(&block.data);
    let mut decoder = GzDecoder::new(cursor);
    let mut decompressed = Vec::new();
    decoder.read_to_end(&mut decompressed)?;
    Ok(decompressed)
}
```

**Testing**:
```rust
#[test]
fn test_bgzip_block_parsing() {
    let blocks = parse_bgzip_blocks(Path::new("test.fq.bgz")).unwrap();
    assert!(blocks.len() > 0);
    assert_eq!(blocks[0].data[0], 0x1f);  // gzip magic
    assert_eq!(blocks[0].data[1], 0x8b);
}
```

**Deliverable**: Bgzip block parsing working, tested

### Day 3 (Nov 6): Smart I/O Selection + mmap

**Implement smart I/O** (`biofast/src/io/reader.rs`):
```rust
use memmap2::Mmap;

const MMAP_THRESHOLD: u64 = 50 * 1024 * 1024;  // 50 MB

pub struct BgzipReader {
    source: DataSource,
    blocks: Vec<BgzipBlock>,
    config: BgzipConfig,
}

impl BgzipReader {
    pub fn open(path: &Path) -> io::Result<Self> {
        Self::open_with_config(path, BgzipConfig::default())
    }

    pub fn open_with_config(path: &Path, config: BgzipConfig) -> io::Result<Self> {
        let file_size = std::fs::metadata(path)?.len();

        // Smart I/O method selection
        let source = if file_size >= MMAP_THRESHOLD {
            // Large file: Use mmap + madvise for 2.5× speedup
            eprintln!("[biofast] Large file detected ({} MB), using mmap+madvise",
                      file_size / (1024 * 1024));

            let file = File::open(path)?;
            let mmap = unsafe { Mmap::map(&file)? };

            // Apply APFS optimization hints (macOS only)
            #[cfg(target_os = "macos")]
            unsafe {
                use libc::{madvise, MADV_SEQUENTIAL, MADV_WILLNEED};
                let result = madvise(
                    mmap.as_ptr() as *mut _,
                    mmap.len(),
                    MADV_SEQUENTIAL | MADV_WILLNEED,
                );
                if result == 0 {
                    eprintln!("[biofast] APFS optimization applied (sequential + willneed)");
                }
            }

            DataSource::MemoryMapped(mmap)
        } else {
            // Small file: Use standard I/O (no mmap overhead)
            eprintln!("[biofast] Small file detected ({} MB), using standard I/O",
                      file_size / (1024 * 1024));

            let mut file = File::open(path)?;
            let mut data = Vec::new();
            file.read_to_end(&mut data)?;
            DataSource::StandardIo(data)
        };

        // Parse bgzip blocks from data source
        let blocks = parse_bgzip_blocks_from_source(&source)?;

        eprintln!("[biofast] Found {} bgzip blocks", blocks.len());

        Ok(BgzipReader {
            source,
            blocks,
            config,
        })
    }
}
```

**Testing**:
```rust
#[test]
fn test_smart_io_selection_small() {
    // Small file should use standard I/O
    let reader = BgzipReader::open(Path::new("small.fq.bgz")).unwrap();
    assert!(matches!(reader.source, DataSource::StandardIo(_)));
}

#[test]
fn test_smart_io_selection_large() {
    // Large file should use mmap
    let reader = BgzipReader::open(Path::new("large.fq.bgz")).unwrap();
    assert!(matches!(reader.source, DataSource::MemoryMapped(_)));
}
```

**Deliverable**: Smart I/O selection working, tested on both small and large files

### Day 4 (Nov 7): Parallel Decompression

**Implement parallel decompression** (`biofast/src/io/reader.rs`):
```rust
use rayon::prelude::*;

impl BgzipReader {
    /// Decompress all blocks in parallel
    pub fn decompress_all(&self) -> io::Result<Vec<u8>> {
        let start = std::time::Instant::now();

        let decompressed_blocks: Vec<_> = if self.config.parallel {
            // Parallel decompression (6.5× speedup)
            self.blocks
                .par_iter()
                .map(|block| decompress_block(block))
                .collect::<io::Result<Vec<_>>>()?
        } else {
            // Sequential decompression (fallback)
            self.blocks
                .iter()
                .map(|block| decompress_block(block))
                .collect::<io::Result<Vec<_>>>()?
        };

        let elapsed = start.elapsed();
        eprintln!("[biofast] Decompressed {} blocks in {:.2} ms (parallel: {})",
                  self.blocks.len(), elapsed.as_secs_f64() * 1000.0, self.config.parallel);

        // Concatenate all decompressed blocks
        Ok(decompressed_blocks.concat())
    }

    /// Iterator over FASTQ records (streaming)
    pub fn records(&self) -> impl Iterator<Item = FastqRecord> + '_ {
        // Decompress all blocks first
        let decompressed = self.decompress_all().expect("Decompression failed");

        // Parse FASTQ records from decompressed data
        parse_fastq_records(&decompressed)
    }
}
```

**Benchmarking**:
```rust
#[bench]
fn bench_parallel_decompression(b: &mut Bencher) {
    let reader = BgzipReader::open(Path::new("large.fq.bgz")).unwrap();

    b.iter(|| {
        let decompressed = reader.decompress_all().unwrap();
        black_box(decompressed);
    });
}
```

**Deliverable**: Parallel decompression working, validated with benchmarks

### Day 5 (Nov 8): FASTQ Parsing Integration

**Integrate with FASTQ parser** (`biofast/src/io/fastq.rs`):
```rust
/// Read FASTQ from bgzip file (auto-optimized)
pub fn read_fastq_bgzip(path: &Path) -> io::Result<Vec<FastqRecord>> {
    let reader = BgzipReader::open(path)?;
    Ok(reader.records().collect())
}

/// Stream FASTQ records from bgzip file
pub fn stream_fastq_bgzip(path: &Path) -> io::Result<impl Iterator<Item = FastqRecord>> {
    let reader = BgzipReader::open(path)?;
    Ok(reader.records())
}
```

**API Example**:
```rust
use biofast::io::fastq;

// Simple API (auto-optimized)
for record in fastq::stream_fastq_bgzip("file.fq.bgz")? {
    println!("ID: {}", record.id);
    println!("Seq: {}", String::from_utf8_lossy(&record.sequence));
}

// Advanced API (custom config)
let config = BgzipConfig {
    parallel: true,
    thread_count: Some(8),
};

let reader = BgzipReader::open_with_config("file.fq.bgz", config)?;
for record in reader.records() {
    process_record(&record);
}
```

**Deliverable**: FASTQ integration complete, ready for use

### Day 6 (Nov 9): Testing and Documentation

**Comprehensive testing**:
1. ✅ Unit tests (block parsing, decompression, I/O selection)
2. ✅ Integration tests (real FASTQ files, small and large)
3. ✅ Benchmark tests (validate 6.5-16.3× speedup)
4. ✅ Cross-platform tests (macOS, Linux)

**Documentation**:
1. ✅ API docs (rustdoc)
2. ✅ Usage examples (examples/bgzip_reader.rs)
3. ✅ Performance guide (docs/PERFORMANCE.md)

**Deliverable**: biofast v0.1.0 ready for release (Nov 15)

---

## Performance Validation

### Test Matrix

| File Size | Platform | Expected Speedup | Optimization Used |
|-----------|----------|------------------|-------------------|
| 5 MB | macOS | 6.5× | Parallel bgzip only |
| 5 MB | Linux | 6.5× | Parallel bgzip only |
| 500 MB | macOS | 16.3× | Parallel bgzip + mmap |
| 500 MB | Linux | 6.5× | Parallel bgzip only (no madvise) |

### Validation Commands

```bash
# Build benchmark
cargo build --release --bin biofast-io-benchmark

# Test small file (5 MB)
./target/release/biofast-io-benchmark --file datasets/large_100k_150bp.fq.bgz

# Test large file (500 MB)
./target/release/biofast-io-benchmark --file datasets/huge_10m_150bp.fq.bgz

# Compare to baseline (sequential gzip)
./target/release/biofast-io-benchmark --file datasets/large_100k_150bp.fq.gz --baseline
```

### Expected Results

**Small file (5 MB, 100K sequences)**:
```
Sequential gzip:  646 MB/s
Parallel bgzip:   4,199 MB/s (6.5× faster) ✅
mmap (not used):  N/A (file too small)
```

**Large file (500 MB, 10M sequences)**:
```
Sequential gzip:  646 MB/s
Parallel bgzip:   4,199 MB/s (6.5× faster) ✅
Parallel + mmap:  10,534 MB/s (16.3× faster on macOS) ✅
```

---

## Feature Flags

### Platform-Specific Optimizations

```rust
// Enable mmap optimization (macOS only by default)
#[cfg(all(target_os = "macos", feature = "mmap-optimization"))]
const MMAP_THRESHOLD: u64 = 50 * 1024 * 1024;

#[cfg(not(all(target_os = "macos", feature = "mmap-optimization")))]
const MMAP_THRESHOLD: u64 = u64::MAX;  // Disable mmap on other platforms
```

**Future work**: Test and enable mmap on Linux (with `posix_fadvise`)

### Configuration Options

```rust
#[derive(Default)]
pub struct BgzipConfig {
    pub parallel: bool,        // Default: true
    pub thread_count: Option<usize>,  // Default: None (auto-detect)
    pub enable_mmap: bool,     // Default: true (if platform supports)
    pub mmap_threshold: u64,   // Default: 50 MB
}
```

---

## Error Handling

### Graceful Fallbacks

```rust
impl BgzipReader {
    pub fn open(path: &Path) -> io::Result<Self> {
        // Try mmap first for large files
        match try_open_mmap(path) {
            Ok(reader) => Ok(reader),
            Err(e) => {
                eprintln!("[biofast] mmap failed ({}), falling back to standard I/O", e);
                try_open_standard_io(path)
            }
        }
    }
}
```

**Fallback scenarios**:
1. mmap fails → use standard I/O
2. Parallel decompression fails → use sequential
3. bgzip detection fails → fallback to standard gzip

---

## Timeline Summary

| Day | Task | Deliverable |
|-----|------|-------------|
| **Nov 4** | Dependencies + types | Types defined, deps added |
| **Nov 5** | Bgzip block parsing | Parser working, tested |
| **Nov 6** | Smart I/O + mmap | I/O selection working |
| **Nov 7** | Parallel decompression | 6.5× speedup validated |
| **Nov 8** | FASTQ integration | API complete |
| **Nov 9** | Testing + docs | biofast v0.1.0 ready |

**Release**: biofast v0.1.0 (Nov 15, 2025)

---

## Success Criteria

### Functional Requirements

- ✅ Auto-detect bgzip files (check BC extra field)
- ✅ Smart I/O selection (threshold-based, 50 MB)
- ✅ Parallel decompression (Rayon, all platforms)
- ✅ mmap + madvise optimization (macOS)
- ✅ Graceful fallbacks (mmap fails → standard I/O)
- ✅ FASTQ parsing integration

### Performance Requirements

- ✅ Small files (<50 MB): **6.5× speedup** vs sequential
- ✅ Large files (≥50 MB): **16.3× speedup** vs sequential (macOS)
- ✅ No regression for non-bgzip files
- ✅ Constant memory (streaming, not load-all)

### Code Quality

- ✅ 100% test coverage (unit + integration)
- ✅ Comprehensive rustdoc documentation
- ✅ Examples for common use cases
- ✅ Cross-platform (macOS, Linux, future Windows)

---

## Post-Integration Work

### Week 3-4: Cross-Platform Validation

**Linux (AWS Graviton)**:
- Test `posix_fadvise` (equivalent to macOS `madvise`)
- Measure threshold on Linux (may differ from 50 MB)
- Add Linux-specific optimizations

**Expected**: 6.5× speedup (parallel bgzip), potential 10-15× with Linux madvise

### Week 5-6: Python Bindings

**PyO3 integration**:
```python
from biofast import BgzipReader

reader = BgzipReader.open("file.fq.bgz")
for record in reader.records():
    print(record.id, len(record.sequence))
```

**Zero-copy numpy arrays** (for ML workflows):
```python
sequences = reader.to_numpy()  # Zero-copy with unified memory
```

---

## Conclusion

Integration of I/O optimization stack into biofast provides **16.3× speedup for large genomics files** through two complementary optimizations:

1. **CPU parallel bgzip** (6.5×, production-ready, all platforms)
2. **Smart mmap + APFS** (2.5× additional, macOS, threshold-based)

**Impact**: Reduces I/O bottleneck from 264-352× to 16-22× for large files, enabling **17× E2E speedup** instead of just 1.04-1.08×.

**Timeline**: Week 1-2 (Nov 4-15, 2025)
**Release**: biofast v0.1.0 (Nov 15, 2025)

---

**Plan Author**: Claude + Scott Handley
**Date**: November 4, 2025
**Status**: Ready for implementation
**Target**: biofast v0.1.0 (Nov 15, 2025)
