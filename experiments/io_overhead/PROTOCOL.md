# Phase 5: I/O Overhead Characterization Protocol

**Date**: November 3, 2025
**Goal**: Quantify I/O overhead in real-world bioinformatics pipelines
**Status**: Protocol design

---

## Motivation

Current experiments run **in-memory only** (sequences pre-loaded, no file I/O). Real-world workflows involve:
1. Reading FASTQ files (often compressed)
2. Processing sequences
3. Writing results

**Key question**: Does I/O overhead negate NEON speedup benefits in real workflows?

---

## Experimental Design

### Operations to Test (High Performers)
Focus on operations with strong NEON benefit:
1. `base_counting` (16.8× speedup, most common operation)
2. `gc_content` (16.8× speedup, widely used)
3. `at_content` (24.9× speedup, peak performer)
4. `quality_aggregation` (15.9× speedup, quality control)

**Rationale**: If I/O is bottleneck, even 25× compute speedup won't help.

### Scales to Test
- **Medium**: 10K sequences (3 MB uncompressed)
- **Large**: 100K sequences (30 MB uncompressed)
- **VeryLarge**: 1M sequences (301 MB uncompressed)

**Rationale**: Test progression from I/O-bound (small files) to compute-bound (large files).

### Compression Formats
1. **Uncompressed** (.fastq): Baseline, fastest I/O
2. **gzip** (.fastq.gz): Most common format (90%+ of public data)
3. **zstd** (.fastq.zst): Modern, faster decompression

**Rationale**: gzip is standard, zstd shows modern alternative.

### Configurations
- **naive**: Scalar baseline
- **neon**: Vectorized (16.8-24.9× speedup)

**Rationale**: Compare I/O overhead impact on naive vs NEON.

---

## Measurement Protocol

### End-to-End Pipeline
```
[Read FASTQ] → [Parse sequences] → [Execute operation] → [Write results]
     ↓               ↓                    ↓                    ↓
  t_read         t_parse              t_compute            t_write
```

**Timing breakdown**:
- `t_total`: Wall-clock time (start to finish)
- `t_io`: `t_read + t_parse + t_write`
- `t_compute`: Time spent in operation
- `io_fraction`: `t_io / t_total`
- `compute_fraction`: `t_compute / t_total`

### Statistical Rigor
- **Repetitions**: N=30 per experiment
- **Warmup**: 3 runs (OS page cache stabilization)
- **Outliers**: IQR 1.5× threshold
- **Statistics**: Median, mean, std dev, 95% CI

**Total experiments**: 4 ops × 3 scales × 3 formats × 2 configs = 72 experiments
**Total measurements**: 72 × 30 = 2,160 measurements

---

## Amdahl's Law Analysis

**Formula**:
```
Speedup_max = 1 / (f_serial + (1 - f_serial) / S_parallel)
```

Where:
- `f_serial` = I/O fraction (cannot be parallelized/accelerated)
- `S_parallel` = NEON speedup (16-25×)

**Example**:
- If I/O = 50% of time, max speedup = 1 / (0.5 + 0.5/20) = 1.95× (even with 20× NEON!)
- If I/O = 10% of time, max speedup = 1 / (0.1 + 0.9/20) = 9.5×

**Analysis**:
Calculate theoretical maximum speedup vs observed speedup to quantify I/O bottleneck.

---

## Expected Results

### Hypothesis 1: Small files are I/O-bound
- **Medium scale** (3 MB): I/O > 30% → max speedup ~3×
- **Large scale** (30 MB): I/O ~ 10-20% → max speedup ~5-10×
- **VeryLarge scale** (301 MB): I/O < 5% → max speedup ~15-20×

### Hypothesis 2: Compression matters
- **gzip**: Slower decompression, higher I/O overhead
- **zstd**: Faster decompression, lower I/O overhead
- **uncompressed**: Fastest I/O, but largest files (storage trade-off)

### Hypothesis 3: NEON shifts bottleneck
- **naive**: Always compute-bound (slow)
- **NEON**: Becomes I/O-bound at small scales (fast compute reveals I/O)

---

## Implementation Plan

### 1. Create I/O Benchmark Harness (2 hours)

**New binary**: `crates/asbb-cli/src/bin/io_overhead_benchmark.rs`

Features:
- Read FASTQ from file (with compression detection)
- Time each pipeline stage separately
- Write results to CSV
- Support N repetitions with statistics

### 2. Prepare Test Files (30 min)

**Compress existing datasets**:
```bash
# gzip compression
gzip -k -9 datasets/medium_10000_150bp.fq
gzip -k -9 datasets/large_100000_150bp.fq
gzip -k -9 datasets/very_large_1000000_150bp.fq

# zstd compression
zstd -19 datasets/medium_10000_150bp.fq -o datasets/medium_10000_150bp.fq.zst
zstd -19 datasets/large_100000_150bp.fq -o datasets/large_100000_150bp.fq.zst
zstd -19 datasets/very_large_1000000_150bp.fq -o datasets/very_large_1000000_150bp.fq.zst
```

### 3. Run Experiments (1 hour)

**Execute**: 72 experiments × 30 reps = 2,160 measurements

### 4. Analysis & Visualization (1-2 hours)

**Plots**:
1. I/O fraction vs scale (by compression format)
2. Observed speedup vs Amdahl's law maximum
3. Compression format comparison (throughput vs file size)
4. Pipeline breakdown (stacked bar: read/parse/compute/write)

---

## Deliverables

1. **Code**: `io_overhead_benchmark.rs` (~400 lines)
2. **Data**: `results/io_overhead/io_overhead_n30.csv` (72 experiments)
3. **Plots**: 4 × publication-quality PNG (300 DPI)
4. **Report**: `results/io_overhead/FINDINGS.md`

**Timeline**: 4-6 hours total

---

## Success Criteria

✅ Quantify I/O overhead across scales (% of total time)
✅ Validate Amdahl's law predictions
✅ Compare compression formats (gzip vs zstd vs uncompressed)
✅ Show whether NEON speedup holds in real workflows
✅ Provide recommendations for practitioners

---

## Open Questions

1. **Does OS page cache help?** (warmup runs should stabilize this)
2. **Network I/O vs local?** (out of scope - local only)
3. **Streaming vs batch processing?** (future work)

---

**Next Steps**: Implement I/O benchmark harness
