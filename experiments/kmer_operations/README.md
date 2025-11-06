# K-mer Operations Experiments (Entry 034)

**Lab Notebook Entry**: `lab-notebook/2025-11/20251106-034-EXPERIMENT-kmer-operations.md`

**Status**: Day 1 - Implementation Complete ✅

---

## Overview

Systematic validation of ARM NEON SIMD potential for k-mer operations critical to biometal ML integration (DNABert preprocessing) and genomic indexing workflows.

### Research Questions

1. **Do minimizer operations benefit from NEON?** (Expected: 10-20× based on quality_filter similarity)
2. **Does k-mer spectrum analysis benefit from NEON?** (Expected: 15-20× based on base_counting pattern)
3. **What is the performance baseline for simple k-mer extraction?** (Expected: <2× NEON benefit, memory-bound)

---

## Operations Under Test

### 1. Minimizers (⭐⭐⭐ HIGHEST priority)

**File**: `crates/asbb-ops/src/minimizers.rs`

**Description**: Extract minimum hash k-mer in each sliding window (w k-mers)

**Use cases**:
- Genomic indexing (minimap2, BWA-MEM2)
- Sequence sketching
- Read mapping

**NEON potential**: HIGH (10-20× expected)
- **Pattern**: Comparison-heavy (parallel min-finding)
- **Evidence**: Similar to quality_filter (25.1×, Cohen's d = 5.14, Entry 020-025)
- **Instructions**: vminq_u64 for vectorized minimum

**Implementation status**:
- ✅ Naive baseline complete
- ✅ NEON variant complete (with vectorized validation)
- ✅ Parallel variant complete
- ✅ 10 unit tests passing

### 2. K-mer Spectrum Analysis (⭐⭐ HIGH priority)

**File**: `crates/asbb-ops/src/kmer_counting.rs`

**Description**: Count k-mer frequencies across dataset

**Use cases**:
- Genome size estimation
- Error correction (SPAdes, Flye)
- DNABert preprocessing
- Metagenomics classification

**NEON potential**: HIGH (15-20× expected)
- **Pattern**: Element-wise counting
- **Evidence**: Reuses base_counting pattern (16.7×, Cohen's d = 4.82, Entry 020)
- **Caveat**: Hash table updates remain scalar (sequential)

**Implementation status**:
- ✅ Naive baseline complete (from Entry 014)
- ✅ NEON variant complete (from Entry 014)
- ✅ Parallel variant complete (from Entry 014)
- ✅ 10 unit tests passing

### 3. Simple K-mer Extraction (Baseline)

**File**: `crates/asbb-ops/src/kmer_extraction.rs`

**Description**: Sliding window extraction (overlapping k-mers)

**Use cases**:
- Foundation for all k-mer workflows
- ML tokenization (DNABert)

**NEON potential**: LOW (<2× expected)
- **Pattern**: Memory-bound (sequential windowing with slicing)
- **Evidence**: Similar to Phase 4 operations (reverse_complement 1.03×, Entry 033)
- **Expected**: Scalar-only in biometal (following Phase 4 precedent)

**Implementation status**:
- ✅ Naive baseline complete (from Entry 014)
- ✅ NEON variant complete (from Entry 014)
- ✅ Parallel variant complete (from Entry 014)
- ✅ 11 unit tests passing

---

## Experimental Design

### Parameters

**Operations**: 3 (minimizers, kmer_counting, kmer_extraction)
**Configurations**: 2 (naive, NEON)
**Scales**: 3 (Small 1K, Medium 10K, Large 100K sequences)
**K-mer sizes**: 2 (k=6 for DNA/ML, k=21 for genomics)

**Total experiments**: 3 ops × 2 configs × 3 scales × 2 k-values = **36 experiments**
**Total measurements**: 36 × N=30 = **1,080 measurements**

### Success Criteria

- **≥5× NEON speedup** → Implement in biometal with NEON optimization
- **<5× NEON speedup** → Scalar-only implementation (following Phase 4 precedent from Entry 033)
- **Statistical rigor**: N=30 repetitions, 95% CI, Cohen's d effect sizes
- **Timeline**: Complete by November 12 (5-7 days, time-boxed)

---

## Day 1 Progress (November 6, 2025)

### Completed ✅

1. **Directory structure**: `experiments/kmer_operations/` created
2. **Minimizers implementation**:
   - 500+ lines of code
   - Naive + NEON + Parallel variants
   - 10 unit tests passing
   - FNV-1a hash (minimap2-compatible)
3. **Code review**: Existing k-mer operations validated
   - kmer_extraction.rs: 11 tests passing
   - kmer_counting.rs: 10 tests passing
4. **Build verification**: All code compiles cleanly

### Next Steps (Day 2)

1. Review NEON implementations for optimization potential
2. Create pilot benchmark (quick validation before N=30)
3. Document any NEON improvements needed

---

## Timeline

- **Day 1 (Nov 6)**: ✅ Setup & naive implementation COMPLETE
- **Day 2 (Nov 7)**: NEON optimization review, pilot validation
- **Day 3 (Nov 8)**: Benchmark harness integration
- **Day 4 (Nov 9)**: Run experiments (36 × N=30)
- **Day 5 (Nov 10)**: Analysis & documentation
- **Days 6-7 (Nov 11-12)**: Publication artifact updates (if needed)

---

## Evidence Base Predictions

### High Confidence (Similar patterns validated)

**Minimizers**: 10-20× NEON speedup
- Pattern: Comparison-heavy (like quality_filter 25.1×)
- Cohen's d expected: >2.0 (very large effect)

**K-mer Spectrum**: 15-20× NEON speedup
- Pattern: Element-wise counting (like base_counting 16.7×)
- Cohen's d expected: >2.0 (very large effect)

### Medium Confidence (New pattern)

**K-mer Extraction**: <2× NEON speedup
- Pattern: Memory-bound (like reverse_complement 1.03×)
- Decision: Scalar-only (won't meet ≥5× threshold)

---

## Integration with biometal

**Why this matters**:
- biometal Week 5-6 (Dec 2-13): Python + ML Integration
- Roadmap explicitly mentions "K-mer extraction for BERT"
- Results ready Nov 12 → inform Week 5-6 implementation (6 weeks lead time)

**Without this research**:
- Week 5-6: Implementing k-mer utilities without evidence (violates core principle)
- Paper 2: "K-mer extraction optimized" claim lacks experimental backing
- Missed opportunity: K-mers might show 20× NEON speedup

---

## References

- **Entry 020-025**: DAG validation (NEON patterns, quality_filter 25.1×, base_counting 16.7×)
- **Entry 026-028**: Streaming validation (block-based processing preserves NEON)
- **Entry 033**: Phase 4 operations (scalar-only precedent for memory-bound ops)
- **Entry 014**: Level 1/2 operation implementation (existing k-mer ops)

---

**Created**: November 6, 2025
**Status**: Day 1 Complete ✅
**Next**: Day 2 - NEON optimization review
