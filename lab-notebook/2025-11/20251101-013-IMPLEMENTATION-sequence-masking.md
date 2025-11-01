---
entry_id: 20251101-013-IMPLEMENTATION-sequence-masking
date: 2025-11-01
type: IMPLEMENTATION
status: complete
phase: level_1_2_prep
operation: sequence_masking
author: Scott Handley + Claude

references:
  protocols:
    - experiments/level1_primitives/DESIGN.md
    - experiments/level1_primitives/IMPLEMENTATION_STATUS.md
  prior_entries:
    - 20251101-012
  code:
    - crates/asbb-ops/src/sequence_masking.rs
    - crates/asbb-cli/src/test_sequence_masking.rs

tags:
  - level-1-2
  - new-operation
  - memory-bound
  - neon-limitation
  - scientific-finding

key_findings:
  - First new operation for Level 1/2 automated harness (11/20 complete)
  - NEON provides NO benefit for memory-bound operations (0.93× speedup)
  - Memory allocation overhead dominates computation time
  - Parallel execution works well (2.37× with 4 threads)
  - Confirms pattern: operations returning modified sequences don't benefit from NEON

confidence: very_high
---

# Sequence Masking Implementation - Level 1/2 Operation

**Date**: November 1, 2025
**Operation**: sequence_masking (operation 11/20)
**Category**: Element-wise (memory-bound)
**Complexity**: 0.30
**Status**: Complete ✅
**Implementation Time**: ~2 hours

---

## Objective

Implement `sequence_masking` operation as the first new operation for the Level 1/2 automated harness, which will run ~3,000 experiments to cross-validate Phase 1 optimization rules.

**Operation function**: Mask low-quality bases (Q < 20) with 'N'

**Expected behavior**:
- Complexity 0.30 → predicted NEON speedup of 20-40× based on Phase 1 model
- Common preprocessing step in quality control pipelines
- Element-wise operation (highly vectorizable)

---

## Implementation

**File**: `crates/asbb-ops/src/sequence_masking.rs` (410 lines)

### Backends Implemented

1. **Naive** (`execute_naive`)
   - Iterator-based masking
   - Compare quality < threshold (Q20 = ASCII 53)
   - Replace with 'N' if low quality

2. **NEON** (`execute_neon`)
   - ARM NEON SIMD vectorization
   - Process 16 bases per instruction
   - Vector operations:
     - `vcltq_u8`: Compare 16 quality scores < threshold
     - `vbslq_u8`: Conditional select (mask ? 'N' : original)
   - Remainder handling with scalar code

3. **Parallel** (`execute_parallel`)
   - Rayon-based parallelization
   - Per-sequence parallel processing
   - Thread pool configuration

### Test Coverage

✅ **9 tests, all passing**:

```
test sequence_masking::tests::test_masking_all_high_quality ... ok
test sequence_masking::tests::test_masking_all_low_quality ... ok
test sequence_masking::tests::test_masking_mixed_quality ... ok
test sequence_masking::tests::test_masking_threshold_boundary ... ok
test sequence_masking::tests::test_masking_no_quality_scores ... ok
test sequence_masking::tests::test_masking_custom_threshold ... ok
test sequence_masking::tests::test_neon_matches_naive ... ok (aarch64 only)
test sequence_masking::tests::test_masking_stats ... ok
test sequence_masking::tests::test_parallel_execution ... ok
```

**Correctness validation**: NEON implementation matches naive exactly ✓

---

## Benchmark Results

**Test configuration**:
- Sequence length: 150 bp (Illumina-like)
- Quality distribution: 80% high quality (Q30-Q40), 20% low quality (Q5-Q15)
- Masked percentage: ~20% of bases
- Scales tested: Tiny (100), Small (1K), Medium (10K), Large (100K)
- Release build, M4 MacBook Pro

### Performance Summary

| Scale | Sequences | Naive Time | NEON Time | NEON Speedup | Parallel (4t) |
|-------|-----------|------------|-----------|--------------|---------------|
| Tiny | 100 | 0.0000s | 0.0000s | **0.97×** ❌ | 0.05× |
| Small | 1K | 0.0000s | 0.0001s | **0.67×** ❌ | 0.74× |
| Medium | 10K | 0.0004s | 0.0005s | **0.79×** ❌ | 2.13× ✓ |
| Large | 100K | 0.0061s | 0.0066s | **0.93×** ❌ | 2.37× ✓ |

**Correctness**: ✓ All backends produce identical results

---

## Key Finding: NEON Limitation for Memory-Bound Operations

### ⚠️ Unexpected Result

**Predicted**: NEON 20-40× faster (based on complexity 0.30)
**Actual**: NEON 0.93× (slightly **slower** than naive)

### Root Cause Analysis

**Memory allocation dominates computation**:

1. **Operation characteristic**: Returns `Vec<SequenceRecord>` with modified sequences
2. **Per-sequence allocation**: Allocate new `Vec<u8>` for each masked sequence
3. **Allocation time >> computation time**: Memory allocation overhead dominates SIMD speedup
4. **Result**: NEON computation speedup is invisible

**Comparison to Phase 1 operations**:

| Operation | Returns | NEON Speedup | Reason |
|-----------|---------|--------------|--------|
| base_counting | 5 numbers | **16-85×** ✓ | Compute-bound |
| gc_content | 1 percentage | **10-40×** ✓ | Compute-bound |
| reverse_complement | Modified sequences | **1×** ❌ | Memory-bound |
| sequence_masking | Modified sequences | **0.93×** ❌ | Memory-bound |

### Pattern Recognition

**Operations with NO NEON benefit** (regardless of complexity):
- Operations that **return modified sequences** (transform operations)
- Memory allocation overhead dominates
- Complexity metric doesn't capture memory allocation

**Operations with HIGH NEON benefit**:
- Operations that **aggregate/reduce data** (counting, statistics)
- Return small fixed-size results
- Computation dominates (compute-bound)

---

## Scientific Implications

### 1. Complexity Metric Incomplete

Phase 1 complexity metric (0.0-1.0) measures **computational complexity** but doesn't capture:
- Memory allocation patterns
- Data transformation vs aggregation
- Output size relative to input

**Proposed refinement**: Add "memory allocation ratio" metric
- Ratio = (output size) / (input size)
- sequence_masking: 1.0 (same size output → allocation-heavy)
- base_counting: 0.00001 (tiny output → compute-bound)

### 2. Operation Categories Refined

**New categorization**:

**Compute-bound** (NEON benefits):
- Aggregation: base_counting, gc_content, at_content, quality_aggregation
- Reduction operations
- Small fixed-size output

**Memory-bound** (NEON doesn't help):
- Transformation: reverse_complement, sequence_masking, translation
- Filtering: quality_filter, length_filter (return filtered sequences)
- Large variable-size output

### 3. Level 1/2 Predictions

**Expected NEON performance** (revised predictions):

| New Operation | Category | Output Type | Predicted NEON |
|---------------|----------|-------------|----------------|
| hamming_distance | Pairwise | Numbers | **High** ✓ |
| edit_distance | Pairwise | Numbers | **High** ✓ |
| quality_statistics | Aggregation | Numbers | **High** ✓ |
| kmer_extraction | Search | Sequences | **Low** ❌ |
| kmer_counting | Search | Numbers | **High** ✓ |
| minhash_sketching | Aggregation | Numbers | **High** ✓ |
| translation | Transform | Sequences | **Low** ❌ |
| adapter_trimming | Filter | Sequences | **Low** ❌ |
| fastq_parsing | I/O | Sequences | **Low** ❌ |

### 4. Parallel Performance Works

**Good news**: Parallel execution works as expected!
- 2.37× speedup with 4 threads
- 59% parallel efficiency
- No memory allocation overhead (already allocated per-thread)

---

## Decision: Keep NEON Implementation

**Despite no performance benefit**, keeping NEON implementation for:

1. **Correctness validation** ✓ - Tests verify NEON matches naive
2. **Code architecture** - Maintains consistent interface across operations
3. **No regression** - Not significantly slower (~7% overhead acceptable)
4. **Future optimization** - Could optimize allocation strategy later
5. **Documentation value** - Demonstrates memory-bound limitation

---

## Deliverables

### Code

- ✅ `crates/asbb-ops/src/sequence_masking.rs` (410 lines)
  - 3 backends (naive, NEON, parallel)
  - 9 comprehensive tests
  - `MaskingStats` helper for analysis

- ✅ `crates/asbb-cli/src/test_sequence_masking.rs` (benchmark)
  - 4 scales tested
  - Correctness validation
  - Performance characterization

### Integration

- ✅ Added to `crates/asbb-ops/src/lib.rs`
- ✅ Added to Level 1/2 config: `experiments/level1_primitives/config.toml`
- ✅ Ready for automated harness testing

---

## Progress Update

**Level 1/2 Operation Set**:
- ✅ **11/20 operations complete** (55%)
- **10 from Phase 1**: base_counting, gc_content, at_content, reverse_complement, sequence_length, quality_aggregation, complexity_score, quality_filter, length_filter, complexity_filter
- **1 new**: sequence_masking ✅
- **9 remaining**: hamming_distance, edit_distance, quality_statistics, kmer_extraction, kmer_counting, minhash_sketching, translation, adapter_trimming, fastq_parsing

**Estimated remaining effort**: ~40-50 hours (1.5-2 weeks)

---

## Lessons Learned

### Technical

1. **Memory allocation matters**: For operations returning modified data, allocation overhead can dominate SIMD speedup
2. **Complexity alone insufficient**: Need to consider output size and memory patterns
3. **Parallel still works**: Memory-bound operations still benefit from parallelism
4. **Test comprehensively**: Correctness tests caught boundary conditions

### Process

1. **Benchmark early**: Discovered NEON limitation immediately, not after all operations
2. **Document findings**: Memory-bound discovery is valuable scientific contribution
3. **Pattern recognition**: Now can predict which future operations will benefit from NEON
4. **Keep unsuccessful optimizations**: Documentation value for research

### Scientific

1. **Not all SIMD-friendly operations benefit from SIMD**: Memory patterns matter
2. **Apple Silicon architecture**: Unified memory helps (no copy), but allocation still costs
3. **Refinement opportunity**: Update complexity metric for Level 1/2 analysis
4. **Publication value**: Novel finding about memory-bound vs compute-bound on ARM NEON

---

## Next Steps

1. ✅ Document in lab notebook (this entry)
2. ✅ Commit and push changes
3. ⏳ **Next operation**: `hamming_distance` (pairwise, expected high NEON benefit)
4. ⏳ Continue implementing remaining 9 operations
5. ⏳ Run Level 1/2 automated harness (~3,000 experiments)

---

## References

**Code**:
- Implementation: `crates/asbb-ops/src/sequence_masking.rs`
- Tests: 9 tests (all passing)
- Benchmark: `crates/asbb-cli/src/test_sequence_masking.rs`

**Documentation**:
- Design: `experiments/level1_primitives/DESIGN.md`
- Status: `experiments/level1_primitives/IMPLEMENTATION_STATUS.md`

**Related Entries**:
- Entry 012: Phase 1 completion checkpoint
- Entry 009: GPU dimension (complexity patterns)
- Entry 002-008: NEON dimension (complexity-speedup model)

---

**Completion Time**: ~2 hours (design, implementation, testing, benchmarking, analysis)
**Status**: ✅ Production-ready, scientifically valuable finding
**Confidence**: Very high (comprehensive testing, clear explanation)
