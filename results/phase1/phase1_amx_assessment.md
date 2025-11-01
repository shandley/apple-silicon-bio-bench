# Phase 1: AMX (Apple Matrix Coprocessor) - Applicability Assessment

**Date**: October 31, 2025
**Status**: ⏸️ **DEFERRED** - Not applicable to current operation set
**Recommendation**: Revisit when matrix-based operations are implemented

---

## Executive Summary

After systematic research into Apple's AMX (Apple Matrix Coprocessor) capabilities, we determined that **none of our current 10 primitive operations are naturally suited for matrix acceleration**. AMX is designed for outer-product matrix operations and ML/HPC workloads, while our operations are primarily element-wise counting, filtering, and reduction operations.

**Key Finding**: AMX dimension testing should be **deferred** until we implement matrix-based operations such as sequence alignment, position weight matrices, or k-mer similarity matrices.

---

## What is AMX?

### Architecture

Apple Matrix Coprocessor (AMX) is a 512-bit wide matrix accelerator integrated into Apple Silicon processors:

- **Compute Grid**: 32×32 grid of compute units
- **Operations**: Multiply-accumulate (FMA) operations in outer-product mode
- **Data Types**: FP16, FP32, FP64 support
- **Integration**: Tight CPU integration (lower latency than discrete accelerators)
- **Performance**: M4 peaks at 2.9 FP32 TFLOPS with >200 GFLOPS/Watt efficiency

### Primary Use Cases

AMX excels at:
1. **Matrix multiplication** (outer product operations)
2. **Machine learning inference** (neural network layers)
3. **Scientific computing** (dense linear algebra)
4. **Batch operations** on matrices

### Programming AMX

**Documented Access**: Apple's Accelerate framework (abstracted, no direct control)
**Undocumented Access**: `amx-rs` Rust crate (reverse-engineered, unstable)

AMX instructions are **intentionally undocumented** by Apple. Direct instruction-level programming is possible via reverse engineering but not officially supported.

---

## Analysis of Current Operations

### Our 10 Primitive Operations

| Operation | Category | Computation Pattern | Matrix-Friendly? |
|-----------|----------|---------------------|------------------|
| Base Counting | Element-wise | Count A, C, G, T independently | ❌ No |
| GC Content | Element-wise | Count G+C, divide by total | ❌ No |
| AT Content | Element-wise | Count A+T, divide by total | ❌ No |
| N-Content | Element-wise | Count N bases | ❌ No |
| Sequence Length | Trivial | Return length field | ❌ No |
| Reverse Complement | Transform | Base-by-base lookup + reverse | ❌ No |
| Quality Aggregation | Reduction | Min/max/mean of quality scores | ❌ No |
| Quality Filter | Filtering | Threshold comparison | ❌ No |
| Length Filter | Filtering | Length threshold | ❌ No |
| Complexity Score | Aggregation | Character diversity count | ❌ No |

**Result**: **0 out of 10 operations** are naturally matrix-based.

### Why These Operations Don't Benefit from AMX

**Element-wise operations** (base counting, GC content, etc.):
- Process each base independently
- No matrix structure to exploit
- NEON SIMD already vectorizes efficiently (16 bases in parallel)
- AMX overhead (converting to matrix representation) would negate any benefit

**Filtering operations** (quality filter, length filter):
- Sequential decision-making with branches
- Data-dependent control flow
- Cannot be expressed as matrix operations
- Already branch-limited (NEON shows minimal benefit)

**Reduction operations** (quality aggregation, complexity score):
- Accumulation across sequences
- Simple sum/min/max operations
- No matrix multiplication involved
- Already efficiently handled by NEON + Rayon

**Transform operations** (reverse complement):
- Lookup table operations
- Per-base transformations
- No matrix structure
- NEON lookup tables already efficient

---

## Operations That WOULD Benefit from AMX

### 1. Sequence Alignment (Smith-Waterman, Needleman-Wunsch)

**Why AMX fits**:
- Dynamic programming fills a **matrix** of alignment scores
- Each cell computes from previous cells (outer product pattern)
- Classic matrix operation with known parallelism

**Expected benefit**: 2-10× speedup over NEON for alignment matrix computation

**Implementation effort**: High (requires new operation + AMX backend)

### 2. Position Weight Matrix (PWM) Scoring

**Why AMX fits**:
- PWM is literally a matrix (bases × positions)
- Scoring is matrix-vector multiplication
- Natural fit for AMX outer product

**Expected benefit**: 3-5× speedup for motif scanning

**Implementation effort**: Medium (new operation, straightforward matrix multiply)

### 3. Multiple Sequence Alignment (MSA)

**Why AMX fits**:
- Batched pairwise alignments
- Batched matrix multiply operations
- AMX designed for this workload

**Expected benefit**: 5-15× speedup for large MSA

**Implementation effort**: Very high (complex algorithm + AMX backend)

### 4. K-mer Similarity Matrix

**Why AMX fits**:
- All-vs-all k-mer comparison creates similarity matrix
- Batched dot products (inner products)
- Could leverage AMX vector mode

**Expected benefit**: 2-4× speedup for large k-mer sets

**Implementation effort**: Medium-high (new operation + matrix formulation)

---

## Recommendation

### Option A: Defer AMX Testing (RECOMMENDED)

**Rationale**:
- No current operations are matrix-based
- Implementing new operations solely for AMX testing would delay systematic exploration
- Other dimensions (Neural Engine, GCD/QoS) are more immediately applicable

**Next steps**:
1. Document this finding (this document)
2. Move to **Neural Engine** dimension (sequence classification, ML-based operations)
3. Continue with **GCD/QoS** dimension (threading optimization)
4. Revisit AMX when implementing alignment/PWM operations later

**Timeline impact**: None (no delay to publication)

### Option B: Implement Matrix-Based Operation Now

**Rationale**:
- Aligns with "Apple Silicon first" philosophy (explore novel approaches)
- AMX + bioinformatics is unexplored territory (novel contribution)
- Could discover unexpected benefits

**Next steps**:
1. Implement Smith-Waterman pairwise alignment
2. Add AMX backend using `amx-rs` or Accelerate framework
3. Test systematically across scales
4. Document findings

**Timeline impact**: +1-2 weeks (implementation + testing)

---

## Decision: Defer AMX

Following the systematic pilot approach, we **defer AMX testing** until matrix-based operations are implemented. This decision is based on:

1. **Applicability**: 0/10 current operations benefit from AMX
2. **Efficiency**: Other dimensions are immediately testable
3. **Priorities**: Complete applicable dimensions first
4. **Future-proof**: Document rationale for later revisit

---

## Comparison to Other Dimensions

### Tested Dimensions (Successful)

| Dimension | Operations Applicable | Key Finding |
|-----------|----------------------|-------------|
| NEON SIMD | 10/10 (all operations) | Universal benefit (1-98× speedup) |
| 2-bit Encoding | 10/10 (all sequences) | Operation-specific (1.3-98× benefit) |
| GPU Metal | 1/10 (complexity score only) | NEON effectiveness predicts benefit |
| Parallel/Threading | 10/10 (all operations) | Batch size predicts benefit (10K threshold) |

### Remaining Dimensions (To Test)

| Dimension | Estimated Applicable | Rationale |
|-----------|---------------------|-----------|
| **AMX** | **0/10** (**deferred**) | **No matrix operations** |
| Neural Engine | 2-3/10 (classification) | Sequence quality prediction, contamination detection |
| GCD/QoS | 10/10 (all operations) | Thread scheduling optimization |
| Hardware Compression | 10/10 (I/O bound) | Transparent compression for all ops |

---

## Novel Contributions from This Assessment

### 1. First Systematic Analysis of AMX for Bioinformatics

**Finding**: Standard bioinformatics primitive operations (counting, filtering, transforms) do not map to matrix operations.

**Implication**: AMX benefit requires rethinking algorithm design, not just porting existing operations.

### 2. Operation Categorization by Hardware Fit

**Pattern discovered**:
- **Element-wise operations** → NEON SIMD (not AMX)
- **Matrix operations** → AMX (not yet implemented)
- **Complex aggregations** → GPU (if NEON ineffective)
- **Data-parallel operations** → Rayon threading

This taxonomy helps predict hardware benefit without testing.

### 3. AMX Access Barrier Identified

**Challenge**: AMX is undocumented by Apple, requiring reverse-engineered libraries.

**Implication**: Production use risky (APIs may change), research use acceptable.

---

## Future Work

### When to Revisit AMX

**Trigger conditions**:
1. Implementing sequence alignment operations (Smith-Waterman, Needleman-Wunsch)
2. Adding PWM/motif scoring capabilities
3. Implementing MSA or k-mer similarity matrices
4. After completing all other applicable hardware dimensions

**Expected timeline**: Post-publication (after Level 1/2 analysis)

### Potential AMX Experiments (Future)

**Experiment 1: Smith-Waterman Alignment**
- Test: AMX vs NEON vs Parallel vs GPU
- Hypothesis: AMX 2-10× faster for alignment matrix fill
- Scales: Pair of sequences (100bp → 10Kbp each)
- Thread counts: 1, 2, 4, 8

**Experiment 2: PWM Scoring**
- Test: AMX matrix multiply vs NEON vectorized
- Hypothesis: AMX 3-5× faster for large PWMs
- Scales: PWM size (10×4 → 1000×4) × sequence count (100 → 10M)

**Experiment 3: K-mer Similarity Matrix**
- Test: All-vs-all k-mer comparison using AMX
- Hypothesis: AMX vector mode 2-4× faster
- Scales: K-mer set size (100 → 100K unique k-mers)

---

## Experimental Artifacts

### Files Created

- `results/phase1_amx_assessment.md` - This document

### Research Conducted

- AMX architecture and capabilities (MIT 2025 thesis, reverse-engineering docs)
- Rust `amx-rs` crate (undocumented AMX access)
- Apple Accelerate framework (documented AMX access)
- Matrix operation patterns in bioinformatics

### Code Status

- ❌ No AMX code implemented (not applicable to current operations)
- ✅ All 10 operations tested with other hardware (NEON, GPU, Parallel)
- ⏸️ AMX testing deferred until matrix-based operations implemented

---

## Conclusions

### Main Findings

1. **AMX is not applicable** to our current 10 primitive operations (0/10 benefit)
2. **Matrix-based operations required** for AMX benefit (alignment, PWM, MSA)
3. **Defer AMX testing** until applicable operations are implemented
4. **No impact on publication** (other dimensions provide sufficient insights)

### Practical Impact

**For ASBB**:
- Document AMX as "not applicable" dimension
- Focus on immediately applicable dimensions (Neural Engine, GCD/QoS)
- Revisit AMX when implementing alignment operations

**For BioMetal**:
- Current operations do not benefit from AMX
- If alignment/PWM operations added later, test AMX at that time
- No optimization missed by skipping AMX now

**For Community**:
- First documentation that standard sequence operations don't map to AMX
- Guidance for when to consider AMX (alignment, PWM, MSA)
- Establishes that AMX is specialized, not universal

---

**Assessment Complete Date**: October 31, 2025
**Key Finding**: AMX not applicable to current operation set
**Recommendation**: Defer AMX, proceed to Neural Engine dimension
**Status**: AMX dimension DEFERRED ⏸️ - Ready for next dimension (Neural Engine)
