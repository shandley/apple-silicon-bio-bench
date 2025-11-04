# Phase 6: Comprehensive Streaming Study Protocol

**Date**: November 3, 2025
**Goal**: Comprehensively characterize streaming architecture benefits and overhead
**Status**: Protocol design
**Timeline**: 8-10 hours (implementation + experiments + analysis)

---

## Motivation

**Current state**: We have strong evidence for SIMD benefits (307 DAG experiments) and I/O overhead characterization (Phase 5: 92.5% overhead with batch processing).

**Missing**: Direct measurement of streaming architecture:
1. **Memory footprint**: Claimed "240,000× reduction" is estimated, not measured
2. **Performance overhead**: What's the cost of streaming vs batch processing?
3. **Real-world validation**: Does streaming preserve NEON benefits in practice?

**Goal**: Provide comprehensive streaming characterization to inform biofast library design.

---

## Three-Benchmark Design

### Benchmark 1: Memory Footprint (Streaming vs Batch)
**Question**: How much memory does streaming actually save?

**Approach**: Compare RSS memory for batch (load-all) vs streaming patterns
- **Batch pattern**: `load_all(file) -> process(all) -> write(results)`
- **Streaming pattern**: `for seq in stream(file) { write(process(seq)) }`

**Measurements**:
- Peak RSS memory during processing
- Memory per sequence processed
- Scaling behavior (100 → 1M sequences)

**Expected**: Batch ~360 MB @ 1M seq, Streaming ~10-50 MB constant

---

### Benchmark 2: Streaming Overhead (Performance Cost)
**Question**: What's the performance penalty for streaming?

**Approach**: Compare throughput for batch vs streaming
- **Batch**: Load dataset into Vec, iterate and process
- **Streaming**: Iterator-based, process one record at a time
- Both using same operations (base counting, GC content)

**Measurements**:
- Throughput (sequences/sec) for batch and streaming
- Overhead percentage: `(batch_throughput - streaming_throughput) / batch_throughput`
- Impact of NEON optimization on overhead

**Expected**: Streaming = 95-98% of batch throughput (2-5% overhead)

---

### Benchmark 3: End-to-End Real-World Pipeline
**Question**: Does streaming preserve NEON benefits in real-world pipelines?

**Approach**: Complete pipeline with file I/O
- Read compressed FASTQ (gzip)
- Stream through processing (base counting + quality filtering)
- Write filtered output FASTQ + statistics

**Measurements**:
- Total pipeline throughput (sequences/sec)
- Memory footprint (peak RSS)
- NEON speedup (naive vs NEON with streaming)
- Compare to Phase 5 batch results (validate improvement)

**Expected**:
- NEON speedup maintained (~15-16×)
- Memory constant (~50 MB regardless of input size)
- Throughput higher than batch (due to I/O overlap)

---

## Experimental Design

### Operations to Test
Focus on operations with strong NEON benefit (from DAG findings):

1. **base_counting**: 16.8× NEON speedup, aggregate operation
2. **gc_content**: 16.8× NEON speedup, aggregate operation
3. **quality_filter**: Sequential operation, streaming-friendly

**Rationale**: Mix of NEON-optimized (base counting, GC) and sequential (quality filter) to show general applicability.

### Scales to Test
- **Small**: 1,000 sequences (~300 KB)
- **Medium**: 10,000 sequences (~3 MB)
- **Large**: 100,000 sequences (~30 MB)
- **VeryLarge**: 1,000,000 sequences (~301 MB)

**Rationale**: Test memory scaling and performance across realistic dataset sizes.

### Configurations
- **Naive**: Scalar implementation (baseline)
- **NEON**: Vectorized implementation (optimized)

**Rationale**: Validate NEON benefit is preserved with streaming.

### Statistical Rigor
- **Repetitions**: N=30 per experiment (consistent with DAG study)
- **Warmup**: 3 runs (OS cache stabilization)
- **Outliers**: IQR 1.5× threshold removal
- **Statistics**: Median, mean, std dev, 95% CI

**Rationale**: Match DAG study quality (publication-ready).

---

## Implementation Plan

### Benchmark 1: Memory Footprint (2-3 hours)

**Binary**: `crates/asbb-cli/src/bin/streaming_memory_benchmark.rs`

**Approach**:
```rust
// Batch pattern
fn benchmark_batch(path: &str, scale: usize) -> MemoryStats {
    let baseline = measure_rss();
    let sequences = load_all_sequences(path, scale); // Load into Vec
    let peak = measure_rss();

    for seq in &sequences {
        process(seq); // Process each
    }
    let after_process = measure_rss();

    MemoryStats { baseline, peak, after_process }
}

// Streaming pattern
fn benchmark_streaming(path: &str, scale: usize) -> MemoryStats {
    let baseline = measure_rss();

    for seq in stream_sequences(path, scale)? {
        process(seq); // Process immediately
        measure_rss_periodically(); // Track peak
    }

    MemoryStats { baseline, peak, after_process }
}
```

**Key measurements**:
- RSS before loading
- RSS peak during processing
- RSS after processing
- Memory reduction factor: `batch_peak / streaming_peak`

**Output**: CSV with operation, scale, pattern (batch/streaming), memory stats

---

### Benchmark 2: Streaming Overhead (2-3 hours)

**Binary**: `crates/asbb-cli/src/bin/streaming_overhead_benchmark.rs`

**Approach**:
```rust
// Batch processing
fn benchmark_batch(sequences: &[SequenceRecord], op: &dyn Operation) -> f64 {
    let start = Instant::now();
    let results: Vec<_> = sequences.iter()
        .map(|seq| op.process(seq))
        .collect();
    start.elapsed().as_secs_f64()
}

// Streaming processing
fn benchmark_streaming(sequences: &[SequenceRecord], op: &dyn Operation) -> f64 {
    let start = Instant::now();
    for seq in sequences.iter() {
        let result = op.process(seq);
        // Simulate write (in-memory, no actual I/O)
    }
    start.elapsed().as_secs_f64()
}
```

**Key measurements**:
- Throughput for batch pattern (seq/sec)
- Throughput for streaming pattern (seq/sec)
- Overhead: `(batch - streaming) / batch * 100%`
- Validate for both naive and NEON configs

**Output**: CSV with operation, scale, config, pattern, throughput, overhead_pct

---

### Benchmark 3: End-to-End Pipeline (3-4 hours)

**Binary**: `crates/asbb-cli/src/bin/streaming_e2e_benchmark.rs`

**Approach**:
```rust
// End-to-end streaming pipeline
fn benchmark_e2e_streaming(input: &str, output: &str, op: &dyn Operation) -> PipelineStats {
    let start = Instant::now();
    let baseline_mem = measure_rss();
    let mut peak_mem = baseline_mem;
    let mut count = 0;

    let reader = FastqReader::from_gzip(input)?;
    let mut writer = FastqWriter::new(output)?;

    for record in reader.records() {
        let seq = record?;

        // Process
        let result = op.process(&seq);

        // Filter (example: quality >= 20)
        if passes_filter(&seq) {
            writer.write_record(&seq)?;
        }

        // Track memory periodically
        if count % 1000 == 0 {
            peak_mem = peak_mem.max(measure_rss());
        }
        count += 1;
    }

    let elapsed = start.elapsed().as_secs_f64();

    PipelineStats {
        total_sequences: count,
        elapsed_sec: elapsed,
        throughput: count as f64 / elapsed,
        peak_memory_mb: peak_mem,
        baseline_memory_mb: baseline_mem,
    }
}
```

**Key measurements**:
- Total throughput (sequences/sec) for complete pipeline
- Peak memory during processing
- NEON speedup (naive vs NEON)
- Compare to Phase 5 batch results (I/O overhead reduction)

**Output**: CSV with operation, scale, config, throughput, memory, speedup

---

## Expected Results

### Hypothesis 1: Memory Footprint
**Predicted**:
- Batch: 360 MB @ 1M sequences (measured in Entry 017)
- Streaming: 10-50 MB constant (independent of input size)
- **Reduction factor**: 7-36× (not 240,000×, corrected estimate)

**Impact**: Validates memory-efficient streaming claim with realistic numbers.

### Hypothesis 2: Streaming Overhead
**Predicted**:
- Overhead: 2-5% (iterator abstractions, bounds checking)
- NEON operations: Lower overhead (compute dominates)
- Sequential operations: Higher overhead (iterator cost more visible)

**Impact**: Quantifies performance cost of streaming architecture.

### Hypothesis 3: Real-World NEON Benefit
**Predicted**:
- Phase 5 batch + I/O: 2.16× end-to-end speedup (92.5% I/O overhead)
- Streaming + I/O: 10-14× end-to-end speedup (I/O overlapped)
- **Improvement**: 4.6-6.5× better by using streaming

**Impact**: Proves streaming preserves NEON benefits in real-world pipelines.

---

## Total Experiment Count

### Benchmark 1 (Memory):
- 3 operations × 4 scales × 2 configs × 2 patterns (batch/streaming) = **48 experiments**
- N=30 → 1,440 measurements

### Benchmark 2 (Overhead):
- 3 operations × 4 scales × 2 configs × 2 patterns = **48 experiments**
- N=30 → 1,440 measurements

### Benchmark 3 (End-to-End):
- 2 operations × 3 scales × 2 configs = **12 experiments**
- N=30 → 360 measurements

**Total**: 108 experiments, 3,240 measurements

**Estimated runtime**:
- Small/Medium: Fast (<1 sec/exp)
- Large/VeryLarge: Moderate (1-10 sec/exp)
- Total: ~1-2 hours for all experiments

---

## Analysis Plan

### Memory Analysis
**Plots**:
1. Memory vs scale for batch and streaming (log-log plot)
2. Memory reduction factor by operation

**Metrics**:
- Peak memory (batch vs streaming)
- Memory per sequence
- Reduction factor with 95% CI

### Overhead Analysis
**Plots**:
1. Throughput comparison (batch vs streaming) by scale
2. Overhead percentage by operation and config

**Metrics**:
- Throughput (seq/sec) for both patterns
- Overhead percentage
- Impact of NEON on overhead

### End-to-End Analysis
**Plots**:
1. NEON speedup comparison (batch Phase 5 vs streaming)
2. Memory footprint during E2E pipeline

**Metrics**:
- Pipeline throughput
- NEON speedup preservation
- Memory stability across scales

---

## Deliverables

### Code (3 binaries)
1. `streaming_memory_benchmark.rs` (~300 lines)
2. `streaming_overhead_benchmark.rs` (~250 lines)
3. `streaming_e2e_benchmark.rs` (~400 lines)

### Data (3 CSV files)
1. `streaming_memory_n30.csv` (48 exp, 1,440 measurements)
2. `streaming_overhead_n30.csv` (48 exp, 1,440 measurements)
3. `streaming_e2e_n30.csv` (12 exp, 360 measurements)

### Analysis
1. Python analysis script (`analyze_streaming.py`, ~400 lines)
2. 6 publication-quality plots (300 DPI PNG)
3. Comprehensive findings report (`FINDINGS.md`)

### Timeline
- **Implementation**: 7-8 hours (3 binaries + analysis script)
- **Experiments**: 1-2 hours (108 exp × 30 reps)
- **Analysis + docs**: 1-2 hours
- **Total**: 9-12 hours

---

## Success Criteria

✅ Validate memory benefit with measurements (not estimates)
✅ Quantify streaming overhead (2-5% expected)
✅ Prove streaming preserves NEON benefits (10-14× vs 2.16× batch)
✅ Provide evidence base for biofast library design
✅ Publication-quality data (N=30, 95% CI, comprehensive plots)

---

## Relationship to biofast Library

**This study informs**:
1. **Architecture decision**: Streaming-first design (validated as low-overhead)
2. **Memory claims**: Realistic memory footprint (<50 MB vs 360 MB+)
3. **Performance promises**: NEON benefits preserved (10-14× end-to-end)
4. **User guidance**: When streaming helps vs batch processing

**After this study**: Complete evidence base for biofast design and implementation.

---

**Next Steps**: Implement Benchmark 1 (Memory Footprint)
