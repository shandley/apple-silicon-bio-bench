# Phase 1: Hardware Compression Dimension Pilot - Protocol

**Date**: November 2, 2025
**Status**: Design phase
**Pilot**: 7/9 dimension pilots
**Category**: Apple Silicon system-level optimization

---

## Objective

Systematically test Apple's hardware-accelerated compression for bioinformatics I/O operations to determine if zero-cost compression improves throughput for sequence data processing.

**Research Questions**:
1. Does hardware-accelerated compression provide throughput gains for sequence I/O?
2. Which compression algorithms (LZFSE, LZMA, zstd) perform best for sequence data?
3. What is the compression ratio for typical FASTQ/FASTA data?
4. Does streaming decompression enable faster processing than uncompressed data?
5. What is the minimum data scale for compression benefit?

**Hypothesis**: Hardware-accelerated compression/decompression is "free" (zero CPU cost), enabling faster I/O for bandwidth-limited operations at the cost of storage/memory.

---

## Background: Apple Hardware Compression

### Hardware Support

**AppleArchive Framework** (macOS 11.0+):
- Native Swift/Objective-C APIs
- Hardware acceleration on Apple Silicon
- Automatic dispatch to compression coprocessor
- Zero CPU overhead for compression/decompression

**Supported Algorithms**:
- **LZFSE** (Apple's algorithm): Fast, good compression ratio
- **LZMA**: Best compression ratio, slower
- **Zstandard (zstd)**: Balance of speed and compression
- **LZ4**: Fastest, lower compression ratio

**Hardware Acceleration**:
- Compression coprocessor on M-series chips
- Parallel compression streams
- DMA (Direct Memory Access) for zero-copy

### Prior Art

**iOS/macOS System**:
- APFS filesystem uses LZFSE compression
- Compressed memory pages (macOS VM)
- iCloud backup compression

**Bioinformatics**:
- FASTQ files: 50-60% compression (gzip)
- FASTA files: 60-70% compression (gzip)
- Quality scores: High entropy, harder to compress

---

## Challenge Analysis

### Why Hardware Compression is Different

**Traditional (Software) Compression**:
- CPU-intensive (gzip uses 100% of 1 core)
- Trade-off: Slower processing for smaller files
- I/O becomes compute-bound

**Hardware (AppleArchive) Compression**:
- Compression coprocessor (separate from CPU)
- Near-zero CPU overhead
- I/O remains I/O-bound (not compute-bound)
- "Free" compression if I/O is bottleneck

### Key Questions

1. **Is I/O the bottleneck?** If processing is fast, compression overhead may hurt
2. **Compression ratio**: Better compression = less I/O = faster (if I/O-bound)
3. **Decompression overhead**: Even with hardware, some overhead exists
4. **Streaming**: Can we stream decompress without materializing full dataset?

---

## Experimental Design

### Approach: I/O-Heavy Operations

Focus on operations where I/O is the bottleneck (reading/writing dominates compute):

#### Tier 1: Pure I/O Operations (Primary Focus)

**1. fastq_parsing**
- **Task**: Parse FASTQ file into sequence records
- **Bottleneck**: Reading from disk/memory
- **Compression Test**: Compressed FASTQ → decompress → parse vs uncompressed
- **Backends**:
  - Naive (uncompressed)
  - Hardware Compressed (LZFSE)
  - Hardware Compressed (zstd)
  - Software Compressed (gzip, baseline)

**2. sequence_length** (with I/O)
- **Task**: Compute length histogram of sequences
- **Bottleneck**: Reading sequences
- **Compression Test**: Stream from compressed vs uncompressed

**3. quality_aggregation** (with I/O)
- **Task**: Aggregate quality statistics across dataset
- **Bottleneck**: Reading quality scores
- **Compression Test**: Stream quality scores from compressed file

#### Tier 2: Intermediate Results Compression (Secondary)

**4. kmer_counting** (with output compression)
- **Task**: Count k-mers, write results
- **Bottleneck**: Writing large k-mer count tables
- **Compression Test**: Compress k-mer table before writing

**5. base_counting** (with output compression)
- **Task**: Count bases, write per-sequence results
- **Bottleneck**: Writing per-sequence counts
- **Compression Test**: Compress output stream

---

## Pilot Scope

### Conservative Start (Recommended)

**Operations**: 3 (fastq_parsing, sequence_length, quality_aggregation)
**Backends**: 4 per operation
- Naive (uncompressed read/write)
- LZFSE (Apple's algorithm)
- zstd (industry standard)
- gzip (software baseline for comparison)

**Scales**: 6 (100, 1K, 10K, 100K, 1M, 10M sequences)

**Total**: 3 operations × 4 backends × 6 scales = **72 experiments**

**Rationale**:
- Covers I/O-heavy operations (reading is bottleneck)
- Tests multiple compression algorithms
- Software baseline (gzip) for comparison
- Same scale as other pilots (tractable)

### Data Preparation

**Create Compressed Datasets**:
```bash
# For each dataset (tiny, small, medium, large, vlarge, huge)
for dataset in datasets/*.fq; do
    # LZFSE compression
    aa archive -d "${dataset}.lzfse" -i "$dataset" --compression lzfse

    # zstd compression
    zstd -o "${dataset}.zst" "$dataset"

    # gzip compression (baseline)
    gzip -k "$dataset"  # Creates ${dataset}.gz
done
```

**Result**: 6 scales × 4 formats = 24 dataset files
- `tiny_100_150bp.fq` (uncompressed)
- `tiny_100_150bp.fq.lzfse` (LZFSE)
- `tiny_100_150bp.fq.zst` (zstd)
- `tiny_100_150bp.fq.gz` (gzip)

---

## Implementation Plan

### Phase 1: AppleArchive Integration (Day 1)

**Approach**: Swift wrapper (like Neural Engine, but simpler)

**Swift Wrapper** (`crates/asbb-compression/swift/CompressionWrapper.swift`):
```swift
import Foundation
import AppleArchive

@objc public class CompressionWrapper: NSObject {
    @objc public static func decompressFile(
        sourcePath: String,
        destinationPath: String,
        algorithm: String
    ) -> Bool {
        // Decompress using AppleArchive
        // Returns true on success
    }

    @objc public static func compressData(
        data: Data,
        algorithm: String
    ) -> Data? {
        // Compress data buffer
        // Returns compressed data
    }
}
```

**Rust FFI** (`crates/asbb-compression/src/ffi.rs`):
```rust
#[link(name = "compression_bridge")]
extern "C" {
    fn compression_decompress_file(
        source: *const c_char,
        dest: *const c_char,
        algorithm: *const c_char,
    ) -> bool;
}

pub fn decompress_file(source: &str, algorithm: CompressionAlgorithm) -> Result<Vec<u8>> {
    // Decompress file to temp location
    // Read into memory
    // Return bytes
}
```

**Alternative: Use Existing Rust Crates** (Simpler)

**flate2** (gzip): Already available
**zstd**: Rust crate with C bindings (system zstd)
**LZFSE**: Rust wrapper around Apple's liblzfse

**Recommended**: Use Rust crates for portability, compare to hardware-accelerated via Swift

### Phase 2: Compression Backends (Day 1-2)

**Example: fastq_parsing with compression**

```rust
// crates/asbb-ops/src/fastq_parsing.rs

fn execute_compressed_lzfse(&self, compressed_path: &str) -> Result<OperationOutput> {
    // Decompress using AppleArchive (hardware-accelerated)
    let decompressed = decompress_file(compressed_path, CompressionAlgorithm::LZFSE)?;

    // Parse decompressed data
    let records = parse_fastq_from_bytes(&decompressed)?;

    Ok(OperationOutput::Records(records))
}

fn execute_compressed_zstd(&self, compressed_path: &str) -> Result<OperationOutput> {
    // Decompress using zstd
    let decompressed = decompress_file(compressed_path, CompressionAlgorithm::Zstd)?;

    let records = parse_fastq_from_bytes(&decompressed)?;

    Ok(OperationOutput::Records(records))
}

fn execute_compressed_gzip(&self, compressed_path: &str) -> Result<OperationOutput> {
    // Decompress using flate2 (software baseline)
    use flate2::read::GzDecoder;

    let file = File::open(compressed_path)?;
    let mut decoder = GzDecoder::new(file);
    let mut decompressed = Vec::new();
    decoder.read_to_end(&mut decompressed)?;

    let records = parse_fastq_from_bytes(&decompressed)?;

    Ok(OperationOutput::Records(records))
}
```

### Phase 3: Experiment Harness (Day 2)

**Binary**: `asbb-pilot-compression`

**Experiment Loop**:
```rust
for operation in OPERATIONS {
    for scale in SCALES {
        for compression in COMPRESSIONS {
            let start = Instant::now();

            match compression {
                Compression::None => {
                    operation.execute_naive(&load_sequences(scale.path))?;
                }
                Compression::LZFSE => {
                    operation.execute_compressed_lzfse(&scale.path_lzfse)?;
                }
                Compression::Zstd => {
                    operation.execute_compressed_zstd(&scale.path_zst)?;
                }
                Compression::Gzip => {
                    operation.execute_compressed_gzip(&scale.path_gz)?;
                }
            }

            let elapsed = start.elapsed();
            // Record: operation, scale, compression, time
        }
    }
}
```

### Phase 4: Execution & Analysis (Day 3)

**Metrics**:
- **Throughput**: Sequences/sec (higher is better)
- **Compression Ratio**: Compressed size / Uncompressed size
- **Speedup vs Uncompressed**: Time(uncompressed) / Time(compressed)
- **Speedup vs Software (gzip)**: Time(gzip) / Time(hardware)

**Analysis**:
- Does compression improve throughput? (Expected: Yes at large scale)
- Which algorithm is fastest? (Expected: LZFSE or zstd)
- Compression ratio vs speed trade-off
- Scale threshold for benefit

---

## Expected Outcomes

### Scenario 1: Hardware Compression Wins (Most Likely)

**Result**: LZFSE/zstd 1.5-2.5× faster than uncompressed at large scale
**Cause**: I/O bandwidth is bottleneck, compression reduces bytes read
**Compression Ratio**: FASTQ ~2× compression (50% size)
**Speedup Math**: 2× less data → ~2× faster (if I/O-bound)
**Conclusion**: Hardware compression beneficial for large-scale I/O

### Scenario 2: Compression Overhead Dominates (Possible at Small Scale)

**Result**: Compression 0.8-0.9× slower at tiny/small scales
**Cause**: Decompression overhead exceeds I/O savings
**Threshold**: Benefit starts at 10K-100K sequences
**Conclusion**: Compression beneficial only at scale

### Scenario 3: No Clear Winner (Unlikely)

**Result**: All compression methods ~1.0× vs uncompressed
**Cause**: Processing is not I/O-bound (compute dominates)
**Conclusion**: Compression irrelevant for sequence operations

---

## Success Criteria

**Minimum Viable Pilot**:
- ✅ 72 experiments complete (3 ops × 4 compressions × 6 scales)
- ✅ Compression ratios measured for FASTQ/FASTA
- ✅ Hardware (LZFSE) vs software (gzip) comparison
- ✅ Scale threshold for benefit identified
- ✅ Decision made: Use compression vs skip

**Optional (if promising)**:
- Test streaming decompression (process without materializing full file)
- Test compressed intermediate results (k-mer tables, etc.)
- Test LZMA (best compression) vs LZFSE (fast compression)
- Measure CPU utilization (verify hardware acceleration)

---

## Technical Risks & Mitigations

### Risk 1: AppleArchive Swift/Rust FFI Complexity

**Problem**: FFI may add overhead, negate compression benefit
**Mitigation**: Use Rust zstd/LZFSE crates (no FFI)
**Fallback**: Prototype in Swift, measure benefit before porting

### Risk 2: Compression Ratio Unknown

**Problem**: FASTQ may not compress well (quality scores have high entropy)
**Mitigation**: Measure compression ratio separately first
**Pre-check**: Compress one dataset with each algorithm, check ratio

**Quick Test**:
```bash
# Test compression ratios
original=$(stat -f%z datasets/medium_10k_150bp.fq)
aa archive -d test.lzfse -i datasets/medium_10k_150bp.fq --compression lzfse
compressed=$(stat -f%z test.lzfse)
ratio=$(echo "scale=2; $compressed / $original" | bc)
echo "LZFSE compression ratio: $ratio"
```

### Risk 3: Processing Not I/O-Bound

**Problem**: If compute dominates, compression won't help
**Mitigation**: Choose I/O-heavy operations (fastq_parsing, etc.)
**Validation**: Profile uncompressed version (should show I/O wait time)

### Risk 4: Hardware Acceleration Unclear

**Problem**: Rust crates may not use hardware acceleration
**Mitigation**: Compare Rust zstd vs Swift AppleArchive
**Verification**: Monitor CPU usage (hardware should be <10% CPU)

---

## Compression Ratio Pre-Check

**Before implementing backends, validate compression benefit**:

```bash
#!/bin/bash
# scripts/test_compression_ratios.sh

for dataset in datasets/*.fq; do
    original=$(stat -f%z "$dataset")
    echo "Dataset: $(basename $dataset), Size: $original bytes"

    # LZFSE
    aa archive -d "${dataset}.lzfse" -i "$dataset" --compression lzfse
    lzfse_size=$(stat -f%z "${dataset}.lzfse")
    lzfse_ratio=$(echo "scale=2; $lzfse_size / $original" | bc)
    echo "  LZFSE: ${lzfse_ratio} (${lzfse_size} bytes)"

    # zstd
    zstd -q -o "${dataset}.zst" "$dataset"
    zstd_size=$(stat -f%z "${dataset}.zst")
    zstd_ratio=$(echo "scale=2; $zstd_size / $original" | bc)
    echo "  zstd: ${zstd_ratio} (${zstd_size} bytes)"

    # gzip
    gzip -c "$dataset" > "${dataset}.gz"
    gzip_size=$(stat -f%z "${dataset}.gz")
    gzip_ratio=$(echo "scale=2; $gzip_size / $original" | bc)
    echo "  gzip: ${gzip_ratio} (${gzip_size} bytes)"

    echo ""
done
```

**Expected Ratios** (based on typical FASTQ):
- LZFSE: 0.40-0.50 (50-60% compression)
- zstd: 0.35-0.45 (55-65% compression)
- gzip: 0.40-0.50 (50-60% compression)

**Decision Point**: If ratios <0.50, compression likely beneficial

---

## Timeline

**Optimistic** (Using Rust crates, no FFI):
- Day 1: Compression ratio pre-check, integrate zstd/LZFSE Rust crates
- Day 2: Implement 3 compression backends, create harness
- Day 3: Run 72 experiments, analyze results
- **Total**: 3 days

**Realistic** (With Swift FFI for AppleArchive):
- Day 1: Compression pre-check, Swift wrapper
- Day 2: Rust FFI bindings, compression backends
- Day 3: Experiment harness, run experiments
- Day 4: Analysis and documentation
- **Total**: 4 days

**Conservative** (Full exploration):
- Week 1: Implementation (FFI, backends, harness)
- Week 2: Experiments, analysis, documentation
- **Total**: 1.5-2 weeks

---

## Deliverables

1. **Protocol** (this document): Experimental design
2. **Compression Ratio Report**: Pre-check results for FASTQ data
3. **Implementation**: Compression backends for 3 operations
4. **Harness**: `asbb-pilot-compression` experiment binary
5. **Datasets**: Compressed versions of all 6 scales (24 files total)
6. **Results**: CSV with 72 experiments
7. **Analysis**: Summary with speedup vs compression algorithm
8. **Lab Notebook**: Entry documenting Hardware Compression pilot
9. **Checkpoint Update**: `PILOT_CHECKPOINT.md` (7/9 complete)

---

## Decision Point: Implementation Approach

### Option A: Pure Rust (Recommended for Speed)

**Crates**:
- `flate2` (gzip): Well-supported
- `zstd`: Rust bindings to libzstd
- `lzfse-rust`: Wrapper around Apple's liblzfse

**Pros**:
- No FFI complexity
- Fast implementation (2-3 days)
- Cross-platform (portable)

**Cons**:
- May not use hardware acceleration (software implementation)
- Can't directly test AppleArchive

### Option B: Swift AppleArchive (Most "Apple Silicon First")

**Approach**: Swift wrapper with FFI (like Neural Engine design)

**Pros**:
- Guaranteed hardware acceleration
- Direct access to AppleArchive APIs
- Most Apple Silicon-native

**Cons**:
- FFI complexity (adds 1-2 days)
- macOS-only

### Hybrid: Option A + Validation

**Recommended**:
1. Implement using Rust crates (fast, 2-3 days)
2. Compare to `gzip` command (baseline)
3. Validate hardware acceleration with CPU monitoring
4. If promising, prototype Swift AppleArchive for comparison

**Rationale**: Faster iteration, can add AppleArchive later if needed

---

## References

- AppleArchive Framework: https://developer.apple.com/documentation/applearchive
- LZFSE: https://github.com/lzfse/lzfse
- Zstandard: https://facebook.github.io/zstd/
- Rust zstd crate: https://docs.rs/zstd/latest/zstd/
- Compression in bioinformatics: https://academic.oup.com/bib/article/20/4/1076/5066661

---

**Created**: November 2, 2025
**Status**: Design complete, ready for compression ratio pre-check
**Next Steps**:
1. Run compression ratio pre-check
2. Choose implementation approach (Pure Rust recommended)
3. Implement compression backends
4. Create experiment harness
5. Execute 72 experiments
