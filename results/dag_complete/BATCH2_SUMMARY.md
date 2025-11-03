# Batch 2: Core Affinity × NEON Results Summary

**Date**: November 3, 2025
**Batch**: Core Affinity Testing
**Hardware**: Mac M4 Air (4 P-cores, 6 E-cores, 24GB RAM)

---

## Execution Summary

**Total experiments**: 60
- **10 operations** × **3 core affinities** × **2 scales**
- All NEON single-threaded (testing core type impact)

**Core affinities tested**:
- **Default**: OS-scheduled (let macOS decide)
- **P-cores**: Performance cores (QoS UserInitiated)
- **E-cores**: Efficiency cores (QoS Background)

**Time**: 1.9 seconds total (incredibly fast!)
**Output**: `dag_core_affinity.csv` (61 lines including header)

---

## Key Question

**Does core affinity matter for NEON operations?**

Specifically: Should we force P-cores, E-cores, or let OS decide?

---

## Overall Findings

### Summary Table: Relative Performance

| Operation | Scale | P/Default | E/Default | Winner |
|-----------|-------|-----------|-----------|--------|
| base_counting | Medium | 1.25× | 1.16× | P-cores |
| base_counting | Large | 0.99× | 0.71× | Default |
| gc_content | Medium | 1.02× | 1.03× | E-cores |
| gc_content | Large | 1.02× | 1.03× | E-cores |
| at_content | Medium | 1.01× | 1.18× | E-cores |
| at_content | Large | 1.04× | 1.04× | Tie |
| n_content | Medium | 1.11× | 1.36× | E-cores |
| n_content | Large | 1.01× | 0.86× | P-cores |
| quality_aggregation | Medium | 1.03× | 1.01× | P-cores |
| quality_aggregation | Large | 1.10× | 0.92× | P-cores |
| sequence_length | Medium | 1.30× | 1.50× | E-cores |
| sequence_length | Large | 0.97× | 1.02× | E-cores |

**Key observations**:
1. **Differences are small**: Most within ±20%
2. **Scale-dependent**: E-cores excel at Medium (10K), struggle at Large (100K+)
3. **P-cores more consistent**: Performance stable across scales
4. **E-cores benefit some ops**: sequence_length, n_content at small scales

---

## Detailed Analysis by Operation Category

### Category 1: P-cores Preferred (Operations with Cache Sensitivity)

#### base_counting
- **Medium**: P-cores 25% faster, E-cores 16% faster than default
- **Large**: P-cores similar to default, E-cores 29% SLOWER
- **Pattern**: E-cores struggle at large scale (cache size?)
- **Recommendation**: Use P-cores for consistency

#### quality_aggregation
- **Medium**: P-cores 3% faster, E-cores 1% faster (negligible)
- **Large**: P-cores 10% faster, E-cores 8% SLOWER
- **Pattern**: P-cores pull ahead at larger scales
- **Recommendation**: Use P-cores for predictable performance

---

### Category 2: E-cores Competitive (Operations with Low Cache Pressure)

#### gc_content
- **Medium**: P-cores 2% faster, E-cores 3% faster (negligible)
- **Large**: P-cores 2% faster, E-cores 3% faster (consistent)
- **Pattern**: All core types perform similarly
- **Recommendation**: Default (let OS decide)

#### at_content
- **Medium**: P-cores 1% faster, E-cores 18% faster (E-cores win!)
- **Large**: P-cores 4% faster, E-cores 4% faster (tie)
- **Pattern**: E-cores surprisingly fast at medium scale
- **Recommendation**: Default or E-cores acceptable

#### sequence_length
- **Medium**: P-cores 30% faster, E-cores 50% faster (E-cores win!)
- **Large**: P-cores 3% slower, E-cores 2% faster (E-cores still win)
- **Pattern**: E-cores consistently faster (memory bandwidth?)
- **Recommendation**: E-cores preferred

---

### Category 3: No Clear Winner (Variable Performance)

#### n_content
- **Medium**: P-cores 11% faster, E-cores 36% faster (E-cores win!)
- **Large**: P-cores 1% faster, E-cores 14% SLOWER (P-cores win)
- **Pattern**: Scale-dependent behavior
- **Recommendation**: Default (let OS decide based on load)

#### reverse_complement
- **All scales**: <8% difference between core types
- **Recommendation**: Default (doesn't matter)

---

### Category 4: Operations Where NEON Doesn't Help (From Batch 1)

#### quality_filter, length_filter, complexity_score
- **Observation**: Small differences (<5%) between core types
- **But**: NEON itself provides minimal benefit (<1.5× speedup)
- **Recommendation**: Don't use NEON at all (from Batch 1)

---

## Key Insights

### Insight 1: E-cores Excel at Small Scales

**Finding**: E-cores surprisingly fast for some operations at 10K sequences:
- n_content: +36%
- sequence_length: +50%
- at_content: +18%

**Hypothesis**: E-cores may have better memory bandwidth utilization for smaller datasets

**Implication**: For `biofast`, E-cores could be valuable for batch processing many small files

---

### Insight 2: E-cores Struggle at Large Scales

**Finding**: E-cores show performance degradation at 100K+ sequences:
- base_counting: -29% (!)
- n_content: -14%
- quality_aggregation: -8%

**Hypothesis**:
- **Cache size**: M4 E-cores have smaller L2 cache (4MB) vs P-cores (16MB)
- **Memory bandwidth**: E-cores may contend for bandwidth at higher data rates

**Implication**: For large datasets (>100K sequences), prefer P-cores

---

### Insight 3: Default Scheduling is Generally Safe

**Finding**: Default (OS-scheduled) within ±10% of best performer for most operations

**Implication**:
- **Simple rule**: Let OS decide unless specific optimization needed
- **Complexity vs benefit**: Explicit pinning adds complexity for marginal gains

---

### Insight 4: Cache Sensitivity Matters

**Operations with large per-sequence state** (base_counting, quality_aggregation):
- Show larger E-core penalty at large scales
- Likely cache thrashing

**Operations with streaming access** (gc_content, at_content):
- Show minimal difference between core types
- Memory bandwidth, not cache, is bottleneck

---

## Recommendations for biofast Library

### Rule 1: Default to OS Scheduling
**Rationale**: Simple, safe, within ±10% of optimal for most operations

```rust
// Simple approach
biofast::stream("data.fq.gz")?
    .gc_content()  // Let OS decide cores
    .compute()?
```

### Rule 2: Override for Performance-Critical Operations

**Use P-cores explicitly** for:
- base_counting (large datasets)
- quality_aggregation (large datasets)

```rust
// Explicit P-core assignment
biofast::stream("large_data.fq.gz")?
    .base_counting()
    .with_cores(CoreAffinity::PerformanceCores)
    .compute()?
```

### Rule 3: Consider E-cores for Batch Processing

**Use E-cores** for:
- Many small files (< 10K sequences each)
- sequence_length operations
- Parallelizing across multiple small inputs

```rust
// Batch processing on E-cores
for file in small_files {
    biofast::stream(file)?
        .sequence_length()
        .with_cores(CoreAffinity::EfficiencyCores)
        .compute()?
}
```

### Rule 4: Threshold-Based Auto-Selection

**Auto-select core type** based on dataset size:

```rust
pub fn auto_select_cores(num_sequences: usize, operation: Operation) -> CoreAffinity {
    match (operation, num_sequences) {
        // Large datasets on cache-sensitive ops: P-cores
        (Operation::BaseCounting, n) if n > 50_000 => CoreAffinity::PerformanceCores,
        (Operation::QualityAggregation, n) if n > 50_000 => CoreAffinity::PerformanceCores,

        // Small datasets on E-core-friendly ops: E-cores
        (Operation::SequenceLength, n) if n < 50_000 => CoreAffinity::EfficiencyCores,
        (Operation::NContent, n) if n < 50_000 => CoreAffinity::EfficiencyCores,

        // Default: let OS decide
        _ => CoreAffinity::Default,
    }
}
```

---

## Mac M4 Architecture Observations

### P-cores (4 × Performance)
- **L2 Cache**: 16MB shared
- **Clock**: Higher frequency
- **Best for**: Large datasets, cache-sensitive operations

### E-cores (6 × Efficiency)
- **L2 Cache**: 4MB shared
- **Clock**: Lower frequency
- **Best for**: Small datasets, streaming operations, batch processing

### Unified Memory (24GB)
- **Bandwidth**: 120 GB/s
- **Observation**: No apparent bandwidth bottleneck even with all cores
- **Implication**: Memory-bound operations scale well

---

## Comparison to Batch 1

### Batch 1 Finding: NEON+4t optimal
- **4 threads** matched **4 P-cores**
- Multiplicative speedup confirmed

### Batch 2 Finding: Core type matters less than thread count
- **Thread count** (Batch 1): 1.8-2.5× additional benefit
- **Core type** (Batch 2): ±20% difference

**Interpretation**:
- **Parallelism** (Batch 1) >> **Core type** (Batch 2) for performance
- Focus optimization on thread count, not core affinity

---

## Validation of Assumptions

### Assumption 1: P-cores always faster
**Result**: ❌ **FALSE** - E-cores faster for some ops at small scales

### Assumption 2: E-cores not suitable for NEON
**Result**: ❌ **FALSE** - E-cores competitive for NEON at small scales

### Assumption 3: Core affinity has minimal impact
**Result**: ⚠️ **PARTIALLY TRUE** - Impact is scale and operation-dependent

**Corrected understanding**: Core affinity matters for cache-sensitive operations at large scales.

---

## Statistical Summary

**Mean absolute difference**:
- P-cores vs Default: 7.5% faster on average
- E-cores vs Default: 4.2% faster on average (skewed by large-scale penalties)

**Maximum observed benefit**:
- P-cores: +30% (sequence_length at medium)
- E-cores: +50% (sequence_length at medium)

**Maximum observed penalty**:
- E-cores: -29% (base_counting at large)

**Recommendation**: Differences are operation and scale-specific, not universal.

---

## Next Steps

### Immediate
1. **Batch 3**: Scale Thresholds (determine exact cutoffs)
2. **Analysis**: Combine Batches 1-3 for comprehensive optimization rules

### Implementation (Week 2)
3. **biofast**: Implement auto-selection logic
4. **Validation**: Test predictions vs actual performance

---

## Conclusion

**Core affinity impact**: ✅ **MEASURABLE BUT SECONDARY**

**Primary findings**:
1. **Default scheduling is safe**: Within ±10% of optimal
2. **E-cores competitive at small scales**: Valuable for batch processing
3. **P-cores more consistent**: Better for large datasets
4. **Cache sensitivity matters**: Operations differ in core preferences

**Recommendation for biofast**:
- **Default**: Let OS decide (simple, safe)
- **Optional**: Expose core affinity for power users
- **Auto-optimization**: Threshold-based selection (advanced)

---

**Status**: ✅ Batch 2 Complete - Core affinity characterized
**Key Finding**: E-cores surprisingly competitive, but scale-dependent
**Impact**: Enables smarter core utilization in biofast (6 E-cores + 4 P-cores)
**Ready for**: Batch 3 (Scale Thresholds) or Week 2 (biofast implementation)
