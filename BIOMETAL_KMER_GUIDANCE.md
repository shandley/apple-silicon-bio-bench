# biometal: K-mer Implementation Guidance

**Source**: ASBB Entry 034 (K-mer Operations on Apple Silicon)
**Evidence**: Pilot benchmark (N=3), full hardware sweep (NEON + Parallel)
**Date**: November 6, 2025

---

## Executive Summary

**Finding**: K-mer operations are **data-structure-bound** (hash+HashMap), not compute-bound.

**Result**: No Apple Silicon hardware (NEON, GPU, AMX) provides significant speedup.

**Recommendation**: Simple scalar implementations by default, with optional Parallel-4t for extraction only.

---

## Implementation Decisions

### 1. Minimizers ❌ Scalar-Only

**Evidence**: 1.02-1.26× max speedup (Entry 034)
- NEON: 1.02-1.11× (negligible)
- Parallel 2t: 1.06-1.15× (below threshold)
- Parallel 4t: 1.12-1.26× (below 2× threshold)

**Why**: Small output per sequence → thread overhead dominates

**Implementation**:
```rust
/// Minimizer extraction - scalar-only (evidence: Entry 034)
pub fn extract_minimizers(sequence: &[u8], k: usize, w: usize) -> Vec<Minimizer> {
    // Simple scalar implementation with FNV-1a hash
    // No NEON, no parallel (proven not worth the complexity)
}
```

**Priority**: Low (no optimization potential)

---

### 2. K-mer Spectrum (Counting) ❌ Scalar-Only

**Evidence**: 0.95-1.88× inconsistent, sometimes SLOWER (Entry 034)
- NEON: 0.88-1.05× (negligible/negative)
- Parallel 2t: 0.99-1.88× (inconsistent)
- Parallel 4t: 0.95-1.18× (sometimes slower!)

**Why**: HashMap contention → cache thrashing → negative performance

**Implementation**:
```rust
/// K-mer spectrum - scalar-only (evidence: Entry 034)
/// IMPORTANT: DO NOT parallelize (HashMap contention makes it slower)
pub fn kmer_spectrum(sequences: &[&[u8]], k: usize) -> HashMap<Vec<u8>, usize> {
    let mut counts = HashMap::new();

    for seq in sequences {
        for i in 0..=(seq.len() - k) {
            let kmer = &seq[i..i + k];
            if validate_bases(kmer) {
                *counts.entry(kmer.to_vec()).or_insert(0) += 1;
            }
        }
    }

    counts
}
```

**Priority**: No optimization benefit (keep simple)

---

### 3. K-mer Extraction ⚠️ Parallel-4t Optional

**Evidence**: 2.19-2.38× consistent with Parallel-4t (Entry 034)
- NEON: 0.99-1.02× (no benefit)
- Parallel 2t: 1.40-1.46× (moderate)
- Parallel 4t: 2.19-2.38× (consistent, borderline threshold)

**Why**: Large output per sequence → overhead amortized → modest benefit

**Implementation**:
```rust
/// K-mer extraction - scalar default, parallel opt-in
pub struct KmerExtractor {
    parallel: bool,  // false by default (simplicity)
    threads: usize,  // 4 if parallel (empirically optimal from Entry 034)
}

impl KmerExtractor {
    pub fn new() -> Self {
        Self {
            parallel: false,
            threads: 1,
        }
    }

    pub fn with_parallel(threads: usize) -> Self {
        Self {
            parallel: true,
            threads: threads.min(4), // Cap at 4 (Entry 034 evidence)
        }
    }

    pub fn extract(&self, sequences: &[&[u8]], k: usize) -> Vec<Vec<u8>> {
        if self.parallel && sequences.len() > 1000 {
            // Only use parallel for large datasets (overhead amortization)
            self.extract_parallel(sequences, k)
        } else {
            self.extract_scalar(sequences, k)
        }
    }

    fn extract_scalar(&self, sequences: &[&[u8]], k: usize) -> Vec<Vec<u8>> {
        let mut kmers = Vec::new();

        for seq in sequences {
            for i in 0..=(seq.len() - k) {
                let kmer = &seq[i..i + k];
                if validate_bases(kmer) {
                    kmers.push(kmer.to_vec());
                }
            }
        }

        kmers
    }

    fn extract_parallel(&self, sequences: &[&[u8]], k: usize) -> Vec<Vec<u8>> {
        use rayon::prelude::*;

        let pool = rayon::ThreadPoolBuilder::new()
            .num_threads(self.threads)
            .build()
            .unwrap();

        pool.install(|| {
            sequences.par_iter()
                .flat_map(|seq| {
                    (0..=(seq.len() - k))
                        .filter_map(|i| {
                            let kmer = &seq[i..i + k];
                            if validate_bases(kmer) {
                                Some(kmer.to_vec())
                            } else {
                                None
                            }
                        })
                        .collect::<Vec<_>>()
                })
                .collect()
        })
    }
}

// Helper: validate bases (only ACGT)
fn validate_bases(seq: &[u8]) -> bool {
    seq.iter().all(|&b| matches!(b, b'A' | b'C' | b'G' | b'T'))
}
```

**Priority**: Moderate (2.2× benefit, user-configurable)

**Usage**:
```rust
// Default: scalar (simple, fast for small datasets)
let extractor = KmerExtractor::new();
let kmers = extractor.extract(&sequences, 6);

// Opt-in: parallel (2.2× speedup for large datasets)
let extractor = KmerExtractor::with_parallel(4);
let kmers = extractor.extract(&large_sequences, 6);
```

---

## Why K-mers Don't Follow DAG Patterns

### Comparison: Compute-Bound vs Data-Structure-Bound

**Typical operation (base_counting)**:
- NEON: 16.7× speedup
- Parallel 4t: 50-53× speedup (multiplicative!)
- Pattern: Element-wise computation → SIMD-friendly

**K-mer operations**:
- NEON: 0.88-1.11× speedup (negligible/none)
- Parallel 4t: 0.95-2.38× speedup (inconsistent)
- Pattern: Hash+memory operations → NOT SIMD-friendly

### Runtime Breakdown (Entry 034 analysis)

**K-mer operations spend time on**:
1. **Hash computation** (50-60%): Sequential (FNV-1a), can't vectorize
2. **HashMap operations** (30-40%): Sequential updates, thread contention
3. **Base validation** (5-10%): Only part NEON can help → minimal impact
4. **Cache misses** (5-10%): Hash table lookups, random access

**NEON can only accelerate #3** → 5-10% speedup max → ~1× overall

**Parallel helps minimally**:
- Minimizers: Small output → overhead dominates
- Spectrum: HashMap contention → sometimes slower
- Extraction: Large output → overhead amortized → 2.2×

---

## Hardware Dimension Decisions (DAG Framework)

**Tested** (Entry 034):
- ✅ NEON: 0.88-1.11× (no benefit)
- ✅ Parallel: 0.95-2.38× (limited benefit)

**Skipped via DAG rules** (don't waste time implementing):
- ❌ GPU Metal: Complexity <0.55 (Entry 009 threshold)
- ❌ AMX: No matrix operations (Entry 015 rule)
- ❌ 2-bit Encoding: Conversion overhead (Entry 010: 2-4× slower)
- ❌ Neural Engine: Not ML inference workload

**Conclusion**: No Apple Silicon hardware provides significant k-mer speedup.

---

## API Design Principles

### 1. Simplicity by Default

**Most users don't need optimization**:
```rust
// Dead simple API for 90% of use cases
let kmers = extract_kmers(&sequences, 6);
let spectrum = kmer_spectrum(&sequences, 21);
let minimizers = extract_minimizers(&sequences, 15, 5);
```

**Evidence**: 1-2× speedup not worth API complexity

### 2. Power-User Opt-In

**For the 10% who need it**:
```rust
// Advanced: parallel extraction for large datasets (2.2× benefit)
let extractor = KmerExtractor::with_parallel(4);
let kmers = extractor.extract(&large_sequences, 6);
```

**Evidence**: 2.2× is modest but consistent (Entry 034)

### 3. Document Why Not Optimized

```rust
/// K-mer spectrum (frequency counting)
///
/// **Performance note**: This operation is intentionally scalar-only.
/// ASBB Entry 034 found that parallelization causes HashMap contention,
/// making it SLOWER (0.95-1.88×, inconsistent). NEON provides no benefit
/// (hash computation is sequential).
///
/// Evidence: https://github.com/shandley/apple-silicon-bio-bench/blob/main/lab-notebook/2025-11/20251106-034-EXPERIMENT-kmer-operations.md
pub fn kmer_spectrum(sequences: &[&[u8]], k: usize) -> HashMap<Vec<u8>, usize> {
    // Scalar implementation
}
```

**Why**: Educates users, prevents "why not parallel?" issues

---

## Testing Requirements

### Correctness

**All variants must produce identical output**:
```rust
#[test]
fn test_extraction_parallel_matches_scalar() {
    let sequences = generate_test_data(10_000, 150);

    let scalar = KmerExtractor::new().extract(&sequences, 6);
    let parallel = KmerExtractor::with_parallel(4).extract(&sequences, 6);

    // Sort both (order may differ)
    let mut scalar_sorted = scalar;
    scalar_sorted.sort();
    let mut parallel_sorted = parallel;
    parallel_sorted.sort();

    assert_eq!(scalar_sorted, parallel_sorted);
}
```

### Performance

**Verify Entry 034 speedups** (smoke test):
```rust
#[test]
#[ignore] // Run manually, not in CI
fn test_extraction_parallel_speedup() {
    let sequences = generate_test_data(10_000, 150);

    let scalar_time = bench(|| {
        KmerExtractor::new().extract(&sequences, 6)
    });

    let parallel_time = bench(|| {
        KmerExtractor::with_parallel(4).extract(&sequences, 6)
    });

    let speedup = scalar_time / parallel_time;

    // Entry 034: 2.19-2.38× observed
    assert!(speedup >= 1.8 && speedup <= 3.0,
            "Speedup {} outside expected range (1.8-3.0×)", speedup);
}
```

---

## Documentation for Users

### README Example

```markdown
### K-mer Operations

biometal provides fast k-mer extraction and analysis optimized for Apple Silicon.

**Simple API** (recommended for most users):
```rust
let kmers = biometal::kmer::extract_kmers(&sequences, 6);
let spectrum = biometal::kmer::kmer_spectrum(&sequences, 21);
let minimizers = biometal::kmer::extract_minimizers(&sequences, 15, 5);
```

**Advanced: Parallel Extraction** (2.2× speedup for large datasets):
```rust
use biometal::kmer::KmerExtractor;

let extractor = KmerExtractor::with_parallel(4);
let kmers = extractor.extract(&large_sequences, 6);
```

**Performance Notes**:
- K-mer operations are data-structure-bound (hash+HashMap), not compute-bound
- NEON SIMD provides no benefit (hash computation is sequential)
- Parallelization has limited benefit due to data structure overhead
- Only extraction benefits from parallelization (2.2×), and only for large datasets
- Validated by 1,357+ experiments in Apple Silicon Bio Bench

For details, see [ASBB Entry 034](https://github.com/shandley/apple-silicon-bio-bench/blob/main/lab-notebook/2025-11/20251106-034-EXPERIMENT-kmer-operations.md).
```

---

## Comparison to Existing Tools

### minimap2 (Li, 2018)

**Their approach**: Scalar minimizers, file-level parallelism

**Our finding**: **Validates their design** (Entry 034 confirms minimizers don't benefit from parallelization)

**biometal advantage**: Documented evidence for design decision

### DNABert (Ji et al., 2021)

**Their problem**: K-mer extraction preprocessing bottleneck

**Our finding**: 2.2× improvement possible with Parallel-4t

**biometal advantage**: Optional parallel extraction reduces bottleneck

---

## Summary

**Simple defaults**:
- Minimizers: Scalar-only
- K-mer Spectrum: Scalar-only (parallel makes it SLOWER)
- K-mer Extraction: Scalar default

**Power-user option**:
- K-mer Extraction: Parallel-4t opt-in (2.2× benefit)

**Evidence base**:
- Entry 034: Full hardware sweep (NEON + Parallel)
- 18 pilot experiments (N=3), 2 scales, 2 k-values
- Data-structure-bound operations → no hardware acceleration

**Time saved**:
- Avoided implementing complex optimizations that don't help
- Evidence-based decisions prevent wasted effort

**Next steps**:
1. Implement scalar baselines (minimizers, spectrum, extraction)
2. Add optional parallel to extraction (user-configurable)
3. Document why not optimized (educate users)
4. Test correctness and performance

---

**Document Version**: 1.0
**Date**: November 6, 2025
**Source**: ASBB Entry 034
**Status**: Ready for biometal implementation
