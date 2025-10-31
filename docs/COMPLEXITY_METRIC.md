# Operation Complexity Metric Design

**Date**: October 30, 2025
**Status**: Design phase
**Goal**: Quantify operation complexity to predict NEON/parallel speedup

---

## Motivation

**Discovery from N=5 validation**: Operation complexity affects speedup magnitude as a **continuous gradient**, not discrete categories.

**Observed gradient**:
- Simple counting (base/GC): 35-65× NEON at tiny, 14-18× at large
- Medium counting (N-content): 8× NEON at tiny, 3-6× at large
- Complex aggregation (quality): 16-23× NEON at tiny (peak), 7-8× at large
- Transform (reverse complement): 1× NEON (encoding-limited)

**Goal**: Develop a complexity score (0-1) that predicts speedup magnitude without running experiments.

---

## Complexity Dimensions

### 1. Operations Per Byte (0-1 scale)

**Definition**: Number of distinct operations performed on each byte of sequence data.

**Rationale**: More operations per byte = more work = potentially lower SIMD efficiency.

**Scoring**:
- **0.0-0.2**: Single operation (e.g., count one base type)
- **0.2-0.4**: Multiple comparisons (e.g., count A+C+G+T)
- **0.4-0.6**: Multiple operations + accumulation (e.g., count multiple categories)
- **0.6-0.8**: Aggregation with reductions (e.g., min/max/mean)
- **0.8-1.0**: Complex transformations (e.g., translate, multiple lookups)

**Examples**:
- Base counting: 4 comparisons → **0.3**
- GC content: 4 comparisons + division → **0.35**
- N-content: 14+ comparisons + 3 accumulators → **0.5**
- Quality aggregation: 1 comparison + min + max + sum → **0.7**
- Reverse complement: 4 lookups + writes → **0.6** (ASCII), **0.2** (2-bit)

### 2. Accumulator Count (0-1 scale)

**Definition**: Number of independent accumulator variables maintained during processing.

**Rationale**: More accumulators = more NEON registers = potential register spills = lower efficiency.

**Scoring**:
- **0.0**: No accumulators (pure transformation)
- **0.2**: 1 accumulator
- **0.4**: 2-3 accumulators
- **0.6**: 4-5 accumulators
- **0.8**: 6-8 accumulators
- **1.0**: 9+ accumulators

**ARM NEON context**: 32 NEON registers available, but need space for:
- Accumulators (1 register each)
- Comparison vectors (2-10 registers)
- Temporary values (2-4 registers)
- Typical sweet spot: ~12-16 total registers

**Examples**:
- Base counting: 1 accumulator (count) → **0.2**
- GC content: 2 accumulators (gc_count, total) → **0.4**
- N-content: 3 accumulators (count_n, count_acgt, count_ambiguous) → **0.6**
- Quality aggregation: 3 accumulators (min, max, sum) but **complex reductions** → **0.6**
- Reverse complement: 0 accumulators → **0.0**

### 3. Horizontal Reduction Complexity (0-1 scale)

**Definition**: Presence and complexity of cross-lane operations (horizontal reductions).

**Rationale**: Horizontal operations are expensive on SIMD (require shuffles, pair-wise operations).

**Scoring**:
- **0.0**: No horizontal operations
- **0.3**: Simple horizontal sum (addv, vaddlvq)
- **0.5**: Multiple horizontal sums
- **0.7**: Horizontal min/max (expensive)
- **0.9**: Multiple horizontal min/max + widening operations
- **1.0**: Complex multi-stage reductions

**Examples**:
- Base counting: Simple horizontal sum at end → **0.3**
- GC content: Simple horizontal sum at end → **0.3**
- N-content: Multiple horizontal sums (3 accumulators) → **0.5**
- Quality aggregation: Horizontal min + max + sum with widening → **0.9**
- Reverse complement: No reductions → **0.0**

### 4. Scalar Fallback Percentage (0-1 scale)

**Definition**: Percentage of work that must be done with scalar (non-SIMD) operations.

**Rationale**: More scalar fallback = less SIMD benefit = lower speedup.

**Scoring**:
- **0.0**: 0% scalar fallback (fully vectorizable)
- **0.2**: <5% scalar (remainder only)
- **0.4**: 5-15% scalar (some rare cases)
- **0.6**: 15-30% scalar
- **0.8**: 30-50% scalar
- **1.0**: >50% scalar (mostly non-vectorizable)

**Examples**:
- Base counting: ~1% (remainder only) → **0.2**
- GC content: ~1% (remainder only) → **0.2**
- N-content: ~5-10% (ambiguous codes in remainder + main loop fallback) → **0.4**
- Quality aggregation: ~1% (remainder only, but complex reductions) → **0.2**
- Reverse complement (ASCII): ~1%, but **conditional logic limits SIMD** → **0.8**
- Reverse complement (2-bit): ~1% → **0.2**

### 5. Memory Access Pattern (0-1 scale)

**Definition**: Complexity of memory access during operation.

**Rationale**: Sequential access = cache-friendly = higher speedup. Random access = cache-unfriendly.

**Scoring**:
- **0.0**: Pure sequential read (single pass)
- **0.2**: Sequential read + accumulation
- **0.4**: Sequential read + write
- **0.6**: Multiple passes
- **0.8**: Random access pattern
- **1.0**: Random access + multiple passes

**Examples**:
- Base counting: Sequential read → **0.2** (read + accumulate)
- GC content: Sequential read → **0.2** (read + accumulate)
- N-content: Sequential read → **0.2** (read + accumulate)
- Quality aggregation: Sequential read → **0.2** (read + accumulate)
- Reverse complement: Sequential read + write → **0.4**

### 6. Data Dependencies (0-1 scale)

**Definition**: Presence of data dependencies that prevent pipelining.

**Rationale**: Dependencies stall SIMD pipeline, reduce throughput.

**Scoring**:
- **0.0**: No dependencies (fully independent)
- **0.3**: Accumulation dependencies (expected)
- **0.5**: Conditional dependencies
- **0.7**: Loop-carried dependencies
- **1.0**: Complex inter-iteration dependencies

**Examples**:
- Base counting: Accumulation only → **0.3**
- GC content: Accumulation only → **0.3**
- N-content: Accumulation only → **0.3**
- Quality aggregation: Accumulation + comparison → **0.3**
- Reverse complement: Lookup dependencies → **0.5**

---

## Composite Complexity Score

**Formula**: Weighted average of dimensions

```
Complexity = (
    0.3 × Operations_Per_Byte +
    0.2 × Accumulator_Count +
    0.25 × Horizontal_Reduction +
    0.15 × Scalar_Fallback +
    0.05 × Memory_Access +
    0.05 × Data_Dependencies
)
```

**Weights rationale**:
- **Operations per byte (30%)**: Primary driver of work complexity
- **Horizontal reductions (25%)**: Very expensive on SIMD
- **Accumulator count (20%)**: Register pressure is significant
- **Scalar fallback (15%)**: Directly limits SIMD benefit
- **Memory access (5%)**: Less variable (mostly sequential)
- **Data dependencies (5%)**: Less variable (mostly accumulation)

**Result**: Complexity score from 0.0 (trivial) to 1.0 (maximally complex)

---

## Scoring Existing Operations (N=5)

### Base Counting

**Dimensions**:
- Operations per byte: 4 comparisons (A, C, G, T) → **0.3**
- Accumulator count: 4 accumulators (countA, countC, countG, countT) → **0.6**
- Horizontal reduction: 4 simple horizontal sums → **0.5**
- Scalar fallback: ~1% (remainder) → **0.2**
- Memory access: Sequential read + accumulate → **0.2**
- Data dependencies: Accumulation only → **0.3**

**Composite score**:
```
Complexity = 0.3×0.3 + 0.2×0.6 + 0.25×0.5 + 0.15×0.2 + 0.05×0.2 + 0.05×0.3
          = 0.09 + 0.12 + 0.125 + 0.03 + 0.01 + 0.015
          = 0.39
```

**Expected speedup (based on N=5 data)**: 35-65× NEON at tiny, 16-18× at large

---

### GC Content

**Dimensions**:
- Operations per byte: 4 comparisons + division → **0.35**
- Accumulator count: 2 accumulators (gc_count, total) → **0.4**
- Horizontal reduction: 2 simple horizontal sums → **0.3**
- Scalar fallback: ~1% (remainder) → **0.2**
- Memory access: Sequential read + accumulate → **0.2**
- Data dependencies: Accumulation only → **0.3**

**Composite score**:
```
Complexity = 0.3×0.35 + 0.2×0.4 + 0.25×0.3 + 0.15×0.2 + 0.05×0.2 + 0.05×0.3
          = 0.105 + 0.08 + 0.075 + 0.03 + 0.01 + 0.015
          = 0.315
```

**Expected speedup (based on N=5 data)**: 14-35× NEON at tiny, 14× at large

---

### N-Content

**Dimensions**:
- Operations per byte: 14+ comparisons (N, ACGT, 10 ambiguous) → **0.5**
- Accumulator count: 3 accumulators → **0.6**
- Horizontal reduction: 3 horizontal sums → **0.5**
- Scalar fallback: ~5-10% (ambiguous codes) → **0.4**
- Memory access: Sequential read + accumulate → **0.2**
- Data dependencies: Accumulation only → **0.3**

**Composite score**:
```
Complexity = 0.3×0.5 + 0.2×0.6 + 0.25×0.5 + 0.15×0.4 + 0.05×0.2 + 0.05×0.3
          = 0.15 + 0.12 + 0.125 + 0.06 + 0.01 + 0.015
          = 0.48
```

**Expected speedup (based on N=5 data)**: 8× NEON at tiny, 3-6× at large

---

### Quality Aggregation

**Dimensions**:
- Operations per byte: Comparison + min + max + sum + widening → **0.7**
- Accumulator count: 3 accumulators (min, max, sum) → **0.6**
- Horizontal reduction: min + max + sum with widening → **0.9**
- Scalar fallback: ~1% (remainder, but expensive reductions) → **0.2**
- Memory access: Sequential read + accumulate → **0.2**
- Data dependencies: Accumulation + comparison → **0.3**

**Composite score**:
```
Complexity = 0.3×0.7 + 0.2×0.6 + 0.25×0.9 + 0.15×0.2 + 0.05×0.2 + 0.05×0.3
          = 0.21 + 0.12 + 0.225 + 0.03 + 0.01 + 0.015
          = 0.61
```

**Expected speedup (based on N=5 data)**: 16-23× NEON at tiny (peak at 1K), 7-8× at large

---

### Reverse Complement (ASCII)

**Dimensions**:
- Operations per byte: 4 lookups + conditional logic → **0.6**
- Accumulator count: 0 (transformation) → **0.0**
- Horizontal reduction: None → **0.0**
- Scalar fallback: High (conditional logic limits SIMD) → **0.8**
- Memory access: Sequential read + write → **0.4**
- Data dependencies: Lookup dependencies → **0.5**

**Composite score**:
```
Complexity = 0.3×0.6 + 0.2×0.0 + 0.25×0.0 + 0.15×0.8 + 0.05×0.4 + 0.05×0.5
          = 0.18 + 0.0 + 0.0 + 0.12 + 0.02 + 0.025
          = 0.345
```

**Expected speedup (based on N=5 data)**: 1× NEON (encoding-limited, not complexity)

**Note**: ASCII encoding creates a special case - complexity score is moderate (0.345), but speedup is 1× due to encoding inefficiency. 2-bit encoding would likely show 98× speedup (validated in prior work).

---

## Analysis: Complexity vs Speedup

### Hypothesis

**If complexity metric is valid**, we should see correlation:
- **Lower complexity → Higher NEON speedup**
- **Higher complexity → Lower NEON speedup**

### Observed Scores (Sorted by Complexity)

| Operation | Complexity Score | NEON (tiny) | NEON (large) |
|-----------|-----------------|-------------|--------------|
| GC content | 0.315 | 35× | 14× |
| **Reverse comp*** | **0.345** | **1×** | **1×** (encoding-limited) |
| Base counting | 0.39 | 53-65× | 16-18× |
| N-content | 0.48 | 8× | 3-6× |
| Quality aggr. | 0.61 | 16-23× | 7-8× |

**Observation**: Reverse complement is an **outlier** due to encoding, not complexity.

### Excluding Encoding-Limited Operations

| Operation | Complexity Score | NEON (tiny) | NEON (large) |
|-----------|-----------------|-------------|--------------|
| GC content | 0.315 | 35× | 14× |
| Base counting | 0.39 | 53-65× | 16-18× |
| N-content | 0.48 | 8× | 3-6× |
| Quality aggr. | 0.61 | 16-23× | 7-8× |

**Trend**:
- GC/Base (0.315-0.39): 35-65× → 14-18× (high speedup)
- N-content (0.48): 8× → 3-6× (medium speedup)
- Quality (0.61): 16-23× → 7-8× (complex, but peak behavior)

**Observation**: Correlation exists but **not linear**:
- Quality shows peak at 1K (22.73×), higher than N-content
- This suggests **other factors** beyond complexity (cache sweet spot)

---

## Next Steps

### 1. Validate Scoring Consistency

**Question**: Do independent scorers arrive at similar complexity scores?
- Score operations blind (without knowing speedup)
- Compare scores, identify ambiguities
- Refine dimension definitions

### 2. Regression Modeling

**Goal**: Predict speedup from complexity + scale

**Model options**:
1. **Linear regression**: `speedup ~ complexity + scale`
2. **Polynomial**: `speedup ~ complexity + scale + complexity² + scale²`
3. **Interaction**: `speedup ~ complexity × scale`
4. **Tree-based**: Gradient boosting, random forest

**Training data**: N=5 operations × 6 scales = 30 data points

**Validation**: Cross-validation (leave-one-out), held-out test set

### 3. Identify Outliers and Anomalies

**Known outliers**:
- Reverse complement (encoding-limited, not complexity)
- Quality aggregation (peak behavior at 1K, cache sweet spot)

**Questions**:
- Can we model encoding as separate dimension?
- Can we model cache effects separately?
- Are there other outlier patterns?

### 4. Test Predictions

**Validation approach**:
- Predict speedup for hypothetical operations
- Implement 1-2 new operations
- Measure actual speedup
- Compare to predictions

**Success criteria**:
- Prediction accuracy >70% (within 2× of actual)
- Can identify high-value optimization targets

---

## Success Criteria

**Complexity metric is successful if**:

1. ✅ **Consistent scoring**: Independent scorers agree (±0.1 on 0-1 scale)
2. ✅ **Correlation with speedup**: R² > 0.6 for complexity vs speedup
3. ✅ **Predictive accuracy**: 70%+ accuracy on held-out test operations
4. ✅ **Actionable insights**: Can rank operations by optimization value
5. ✅ **Generalizable**: Works for operations beyond N=5

---

## Timeline

**Week 1: Scoring & Validation**
- Day 1: Refine dimension definitions
- Day 2: Score N=5 operations (independent validation)
- Day 3: Analyze correlation, identify outliers

**Week 2: Regression Modeling**
- Day 4: Build linear/polynomial models
- Day 5: Test gradient boosting, random forest
- Day 6: Cross-validation, hyperparameter tuning
- Day 7: Finalize best model, document predictions

**Total**: ~2 weeks (7 working days)

---

## Open Questions

1. **Should encoding be a separate dimension?** (Binary: ASCII vs 2-bit)
2. **How to model cache effects?** (Peak behavior at certain scales)
3. **What about parallel complexity?** (Focus on NEON for now, extend later)
4. **Can we predict parallel thresholds?** (1K for simple, 10K for complex)
5. **How to handle outliers?** (Encoding-limited, cache sweet spots)

---

**Status**: Design complete, ready for implementation
**Next**: Refine scoring, validate on N=5, build regression model

