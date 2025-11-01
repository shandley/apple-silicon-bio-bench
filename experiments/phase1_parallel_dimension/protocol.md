# Phase 1: Parallel/Threading Dimension - Experimental Protocol

**Date Created**: October 31, 2025
**Status**: Ready for execution
**Estimated Runtime**: 3-4 hours (automated)

---

## Objectives

### Primary Research Questions

1. **Which operations benefit from parallelism?**
   - Does complexity score predict parallel benefit?
   - Are there operation categories that don't scale?

2. **What's the optimal thread count per operation?**
   - Is there a universal optimal (e.g., always use 4 threads)?
   - Or is it operation-dependent?

3. **What's the batch size threshold for parallel benefit?**
   - At what scale does threading overhead dominate?
   - Does this vary by operation?

4. **Do P-cores vs E-cores matter?**
   - Can we measure performance difference with QoS hints?
   - Which operations benefit from P-core assignment?

5. **How does threading interact with NEON?**
   - Validate "parallel amplifies NEON" principle
   - Measure combined speedup (NEON × threads)

6. **Does heterogeneous compute provide benefit?**
   - Is explicit P+E mixing better than OS default?
   - Or is default scheduling sufficient?

---

## Experimental Design

### Hardware Configurations

**Thread counts**: 1, 2, 4, 8

**Core assignment strategies** (per thread count):
- **Default**: Let macOS scheduler decide (baseline)
- **P-cores**: QoS UserInitiated (hints for Performance cores)
- **E-cores**: QoS Background (hints for Efficiency cores)

**Total configurations per operation per scale**:
- 1 thread × 1 assignment (default only) = 1 config
- 2 threads × 3 assignments = 3 configs
- 4 threads × 3 assignments = 3 configs
- 8 threads × 3 assignments = 3 configs
- **Total**: 10 configs per operation per scale

### Operations

All 10 operations across complexity spectrum:

| # | Operation | Complexity | NEON Speedup | Expected Parallel |
|---|-----------|------------|--------------|-------------------|
| 5 | n_content | 0.25 | ~10× | Should scale |
| 2 | gc_content | 0.32 | 45× | Should scale |
| 7 | at_content | 0.35 | ~40× | Should scale |
| 1 | base_counting | 0.40 | 16× | **Validated: 40-60×** |
| 3 | reverse_complement | 0.45 | 1× | Test |
| 4 | quality_aggregation | 0.50 | 7-12× | Should scale |
| 8 | quality_filter | 0.55 | ~1.1× | Test (filtering) |
| 9 | length_filter | 0.55 | 1× | Test (trivial) |
| 6 | sequence_length | 0.20 | 1× | Test (trivial) |
| 10 | complexity_score | 0.61 | 1× | Test (complex) |

### Scales

6 standard scales (5 orders of magnitude):
- **Tiny**: 100 sequences
- **Small**: 1,000 sequences
- **Medium**: 10,000 sequences
- **Large**: 100,000 sequences
- **VeryLarge**: 1,000,000 sequences
- **Huge**: 10,000,000 sequences

### Total Experiments

```
10 operations × 10 configurations × 6 scales = 600 experiments
```

**Breakdown**:
- Base counting: 60 experiments (10 configs × 6 scales)
- GC content: 60 experiments
- AT content: 60 experiments
- N-content: 60 experiments
- Sequence length: 60 experiments
- Reverse complement: 60 experiments
- Quality aggregation: 60 experiments
- Quality filter: 60 experiments
- Length filter: 60 experiments
- Complexity score: 60 experiments

---

## Implementation Details

### Core Affinity via QoS

**macOS Approach**:
- No reliable explicit core pinning API
- Use pthread QoS classes (Apple-recommended approach)
- QoS hints influence scheduler's P-core vs E-core assignment

**QoS Classes Used**:
```c
QOS_CLASS_BACKGROUND (0x09)      // E-cores preferred
QOS_CLASS_DEFAULT (0x15)         // OS decides (baseline)
QOS_CLASS_USER_INITIATED (0x19)  // P-cores preferred
```

**API Call**:
```rust
extern "C" {
    fn pthread_set_qos_class_self_np(qos_class: u32, relative_priority: i32) -> i32;
}
```

**Validation**:
- QoS is a *hint*, not a guarantee
- Actual core assignment depends on system load
- Use Activity Monitor to observe which cores are active

### Measurement Protocol

**For each experiment**:

1. **Warmup**: 5 iterations (discard results)
2. **Measurement**: 20 iterations (collect timing)
3. **Outlier removal**: Remove >3 std dev
4. **Compute mean**: Average of remaining samples

**Metrics collected**:
- Time (milliseconds)
- Speedup vs single-threaded (baseline / current)
- Efficiency (speedup / threads)
- Throughput (sequences/sec)

**Validation**:
- All outputs verified for correctness (not measured here, but operations already validated)

---

## Execution Instructions

### Prerequisites

1. **Hardware**: M1/M2/M3/M4 Mac (Apple Silicon)
2. **Data**: Datasets generated in `datasets/` directory
3. **Build**: Release mode (optimizations enabled)
4. **Time**: Allocate 3-4 hours for unattended execution

### Running the Pilot

**Basic execution** (output to stdout):
```bash
cd /Users/scotthandley/Code/apple-silicon-bio-bench
cargo run --release -p asbb-cli --bin asbb-pilot-parallel
```

**Save results to CSV**:
```bash
cargo run --release -p asbb-cli --bin asbb-pilot-parallel > results/parallel_dimension_raw.csv 2> parallel_log.txt
```

**Monitor progress** (in separate terminal):
```bash
tail -f parallel_log.txt
```

### Validating Core Assignment with Activity Monitor

**During execution, observe core activity**:

1. **Open Activity Monitor**
   - Applications → Utilities → Activity Monitor

2. **Switch to CPU view**
   - Window → CPU History
   - Or: View → CPU History

3. **Observe patterns**:
   - **P-cores (QoS UserInitiated)**: First 4-6 cores should show high activity
   - **E-cores (QoS Background)**: Last 4 cores should show activity
   - **Default**: Activity spread across all cores

4. **Take screenshots** (if available):
   - P-core assignment: High activity on cores 0-3 (M4 Pro)
   - E-core assignment: High activity on cores 4-9
   - Default: Mixed activity

**Note**: QoS is a hint, not a guarantee. System load and thermal state affect actual assignment.

### Expected Runtime

**Per operation**:
- Tiny (100): ~10-20 seconds (10 configs × 20 iterations × ~0.01ms)
- Small (1K): ~20-30 seconds
- Medium (10K): ~30-60 seconds
- Large (100K): ~1-2 minutes
- VeryLarge (1M): ~5-10 minutes
- Huge (10M): ~10-20 minutes

**Total per operation**: ~20-30 minutes
**Total for 10 operations**: **3-4 hours**

---

## Output Format

### CSV Structure

**Header**:
```csv
operation,complexity,scale,num_sequences,threads,assignment,time_ms,speedup_vs_1t,efficiency,throughput_seqs_per_sec
```

**Example rows**:
```csv
base_counting,0.40,Tiny,100,1,default,0.083,1.00,1.00,1204.82
base_counting,0.40,Tiny,100,2,default,0.087,0.95,0.48,1149.43
base_counting,0.40,Tiny,100,2,p_cores,0.084,0.99,0.49,1190.48
base_counting,0.40,Tiny,100,2,e_cores,0.092,0.90,0.45,1086.96
base_counting,0.40,Tiny,100,4,default,0.094,0.88,0.22,1063.83
...
```

**Fields**:
- `operation`: Operation name (e.g., "base_counting")
- `complexity`: Operation complexity score (0.20-0.61)
- `scale`: Scale name (Tiny, Small, Medium, Large, VeryLarge, Huge)
- `num_sequences`: Number of sequences in batch
- `threads`: Thread count (1, 2, 4, 8)
- `assignment`: Core assignment strategy (default, p_cores, e_cores)
- `time_ms`: Execution time in milliseconds
- `speedup_vs_1t`: Speedup relative to single-threaded (baseline / time)
- `efficiency`: Scaling efficiency (speedup / threads)
- `throughput_seqs_per_sec`: Sequences processed per second

---

## Analysis Plan

### 1. Speedup Matrices

Generate speedup matrix per operation:

**Example: Base Counting**
```
Threads | Assignment | 100 | 1K | 10K | 100K | 1M | 10M
--------|------------|-----|----|----|------|----|----|
1       | default    | 1.0×| 1.0×| 1.0×| 1.0×| 1.0×| 1.0×
2       | default    | 0.95×| 1.85×| 1.92×| 1.95×| 1.98×| 1.99×
2       | p_cores    | 0.99×| 1.88×| 1.94×| 1.97×| 1.99×| 2.00×
2       | e_cores    | 0.90×| 1.75×| 1.85×| 1.88×| 1.90×| 1.92×
4       | default    | 0.88×| 3.45×| 3.78×| 3.89×| 3.95×| 3.98×
4       | p_cores    | 0.92×| 3.55×| 3.88×| 3.96×| 3.99×| 4.00×
4       | e_cores    | 0.75×| 3.10×| 3.45×| 3.60×| 3.70×| 3.80×
8       | default    | 0.75×| 5.20×| 6.89×| 7.45×| 7.78×| 7.92×
8       | p_cores    | 0.82×| 5.50×| 7.20×| 7.80×| 7.95×| 8.00×
8       | e_cores    | 0.60×| 4.50×| 5.80×| 6.50×| 6.90×| 7.20×
```

### 2. Core Assignment Comparison

For each operation, compare P-cores vs E-cores vs Default:

**Metrics**:
- Relative speedup: `time_default / time_p_cores`
- Benefit percentage: `(speedup_p_cores / speedup_default - 1) × 100%`

**Expected patterns**:
- P-cores faster for compute-intensive operations
- E-cores sufficient for trivial operations
- Default often comparable (good scheduler)

### 3. Threshold Analysis

Identify minimum batch size for parallel benefit:

**Method**: Find where speedup > 1.0 consistently

**Example**:
```
Operation: Base Counting
- 2 threads: Threshold at 1,000 sequences (speedup 1.85×)
- 4 threads: Threshold at 1,000 sequences (speedup 3.45×)
- 8 threads: Threshold at 1,000 sequences (speedup 5.20×)
```

### 4. Scaling Efficiency

Measure how well parallelism scales:

**Ideal**: Linear scaling (2t=2×, 4t=4×, 8t=8×)

**Typical**: Sublinear (2t=1.9×, 4t=3.8×, 8t=7.5×)

**Poor**: Overhead dominates (2t=0.9×, speedup <1.0)

**Compute efficiency**:
```
Efficiency = Speedup / Threads
```

**Classification**:
- Excellent: >90% efficiency
- Good: 70-90%
- Poor: <70%

### 5. Pattern Discovery

**Questions to answer**:
- Does complexity predict parallel benefit?
- Does NEON effectiveness interact with threading?
- Are there operation categories that don't scale?
- What's the relationship between batch size and optimal thread count?

**Statistical methods**:
- Regression: Speedup ~ complexity + batch_size + threads
- ANOVA: Test for interaction effects
- Decision tree: Derive optimal configuration rules

---

## Expected Outcomes

### Hypothesis 1: Parallel Threshold at ~1K Sequences

**Validation**: Base counting Phase 1 showed threshold at 1K sequences

**Test**: Does this hold across all operations?

**Expected**:
- Simple operations (complexity <0.30): Higher threshold (>1K)
- Complex operations (complexity >0.50): Lower threshold (<1K)

### Hypothesis 2: P-cores Faster for Compute Operations

**Expected**:
- Element-wise operations: P-cores 10-20% faster
- Filtering operations: P-cores 5-10% faster
- Trivial operations: No significant difference

### Hypothesis 3: E-cores Competitive at Low Complexity

**Expected**:
- Sequence length (0.20): E-cores ~90% of P-cores
- Length filter (0.55): E-cores ~80% of P-cores
- Complexity score (0.61): E-cores ~70% of P-cores

### Hypothesis 4: NEON × Parallel Multiplicative

**Validation**: Base counting NEON 16× + Parallel 4× = 64× combined

**Test**: Does this hold across all operations?

**Expected**: Yes for element-wise operations

### Hypothesis 5: Diminishing Returns at High Thread Count

**Expected**: 8 threads may not scale well on M4 (4 P-cores + 6 E-cores)

**Reason**: 8 threads on 4 P-cores requires hyperthreading or E-core usage

---

## Decision Rules to Derive

From this experiment, we should be able to derive:

```rust
fn optimal_thread_count(
    operation: &Operation,
    batch_size: usize,
    hardware: &HardwareProfile
) -> usize {
    match (operation.complexity(), batch_size) {
        (c, n) if n < 1_000 => 1,  // Too small for parallel
        (c, n) if c < 0.30 && n < 10_000 => 2,  // Simple ops need more data
        (c, n) if n < 100_000 => 4,  // Sweet spot (matches P-core count)
        (_, n) if n >= 100_000 => 8,  // Large batches can use all cores
        _ => 4,  // Default
    }
}

fn optimal_core_assignment(
    operation: &Operation,
    batch_size: usize
) -> CoreAssignment {
    if operation.complexity() > 0.50 {
        CoreAssignment::PerformanceCores  // Compute-intensive
    } else {
        CoreAssignment::Default  // Let OS decide
    }
}
```

---

## Success Criteria

✅ **Complete** when:

1. All 600 experiments executed successfully
2. CSV file generated with complete results
3. Speedup matrices created for all 10 operations
4. Threshold effects identified (minimum batch size per operation)
5. P-core vs E-core comparison documented
6. Decision rules formalized
7. Results document published (`phase1_parallel_dimension_complete.md`)

---

## Reproducibility

### Version Information

- **ASBB version**: 0.1.0
- **Protocol version**: 1.0
- **Date**: October 31, 2025
- **Hardware**: M4 MacBook Pro (4 P-cores, 6 E-cores)

### Data Provenance

- **Datasets**: Generated by `asbb-datagen`
- **Location**: `datasets/tiny_100_150bp.fq` through `datasets/huge_10m_150bp.fq`
- **Checksums**: See `datasets/checksums.txt`

### Software Versions

- **Rust**: 1.70+
- **macOS**: 14.0+ (Apple Silicon required)
- **Dependencies**: See `Cargo.lock`

---

## Notes and Limitations

### macOS Core Affinity Limitations

**Reality**: macOS does not provide reliable explicit core pinning API like Linux (`sched_setaffinity`).

**Our approach**: Use QoS classes as hints to scheduler.

**Limitations**:
- QoS is advisory, not mandatory
- System load affects actual assignment
- Thermal throttling may move threads
- Background processes interfere

**Validation**: Use Activity Monitor to observe actual core usage patterns.

### Expected Variability

**Sources of noise**:
- Background processes (Spotlight, Time Machine, etc.)
- Thermal throttling (especially on sustained workloads)
- Memory contention (if other apps running)
- macOS scheduler decisions

**Mitigation**:
- Use release mode (optimizations enabled)
- Run multiple iterations (20 per experiment)
- Remove outliers (>3 std dev)
- Run during low system load

**Expected coefficient of variation**: <5% for most experiments

### Comparison to Phase 1 Day 1

**Phase 1 Day 1**: Base counting only, 4 configs × 6 scales = 24 experiments

**This pilot**: All 10 operations, 10 configs × 6 scales = 600 experiments

**Validation**: Base counting results should reproduce Phase 1 findings:
- Parallel threshold at 1K sequences
- 4-thread speedup of 3.5-4.0× at 10K+ sequences
- Combined NEON + parallel: 40-60× speedup

---

## Related Experiments

**Completed**:
- Phase 1 NEON dimension (60 experiments)
- Phase 2 2-bit encoding dimension (72 experiments)
- Phase 1 GPU dimension (32 experiments)

**Future**:
- AMX matrix engine dimension
- Neural Engine dimension
- Hardware compression dimension
- GCD/QoS optimization dimension

**Integration**: After all individual pilots complete, move to Level 1/2 automated testing.

---

**Status**: Ready for execution
**Created**: October 31, 2025
**Estimated completion**: +3-4 hours execution time
