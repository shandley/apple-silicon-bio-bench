# ASBB Methodology: Systematic Experimental Design

**Version**: 1.0
**Date**: October 30, 2025
**Status**: Design Complete, Implementation Beginning

---

## Overview

This document describes the experimental methodology for systematically characterizing the performance of bioinformatics sequence operations across Apple Silicon hardware configurations.

**Goal**: Derive formal optimization rules that can be applied automatically to ANY sequence operation.

**Approach**: Hierarchical experimental design with statistical validation.

---

## Guiding Philosophy: Novel Approaches for Novel Hardware

### Breaking Free from x86 Assumptions

**Critical lesson from BioMetal**: Traditional bioinformatics optimization patterns are not optimal for Apple Silicon.

**Why this matters for ASBB**:
- Most reference implementations (BLAST, BWA, Bowtie, etc.) were designed for x86 architectures
- Traditional approaches assume: discrete GPUs, separate memory spaces, homogeneous cores, SSE/AVX as afterthought
- **Apple Silicon is fundamentally different**: unified memory, heterogeneous cores, NEON-first, Neural Engine, AMX
- **We must actively resist** copying x86 optimization strategies

### Experimental Approach: Traditional + Novel

For each operation, we test **both**:

**1. Traditional implementations** (baseline):
- Standard algorithms from literature
- Naive ports from x86 tools
- "Obvious" optimizations

**2. Apple Silicon-native implementations** (exploratory):
- NEON-first designs (not SIMD ports)
- Unified memory exploitation (zero-copy CPU/GPU)
- Heterogeneous compute (P-cores + E-cores + GCD)
- Neural Engine experiments (ML-based approaches)
- AMX exploration (matrix reformulations)
- Metal-native designs (tile memory, threadgroups)
- Hardware compression integration

### Novel Opportunities to Explore

**These capabilities did not exist pre-2020**:

| Feature | Traditional Assumption | Apple Silicon Reality | Experimental Questions |
|---------|------------------------|------------------------|------------------------|
| **Unified Memory** | Copy to GPU (100ms overhead) | Zero-copy CPU↔GPU | Can CPU+GPU collaborate on same buffer? |
| **NEON SIMD** | SSE/AVX afterthought, library-only | First-class, always available | Can we design NEON-native algorithms? |
| **Neural Engine** | No equivalent | 16-38 TOPS ML inference | Can we frame sequence ops as ML tasks? |
| **P-cores + E-cores** | Homogeneous cores | Heterogeneous + QoS | Can we pipeline I/O (E) + compute (P)? |
| **AMX** | CPU or discrete GPU | Integrated 512-bit matrix ops | Can we reformulate as matrix operations? |
| **Metal** | CUDA/OpenCL memory model | Tile memory, unified memory | Can we exploit Metal's memory hierarchy? |
| **HW Compression** | Software zlib (slow) | Hardware-accelerated | Can we compress intermediate results? |
| **GCD + QoS** | Manual thread management | System-level optimization | Can OS optimize thermal/power for us? |

### Integration into Operation Categories

Each operation category (below) includes:
- **Traditional approach**: Standard implementation
- **Expected hardware**: Based on x86 wisdom
- **Novel explorations**: Apple Silicon-specific experiments
- **Open questions**: What to test

**Goal**: Don't just measure traditional approaches. Discover what becomes possible on Apple Silicon.

---

## The Dimensional Space

### 1. Data Characteristics (Input Space)

#### Format
- **FASTA**: Sequences only (headers + bases)
- **FASTQ**: Sequences + quality scores (Phred+33 encoding)

#### Scale (5 orders of magnitude)
- **Tiny**: 10-100 sequences (unit tests, edge cases)
- **Small**: 1,000 sequences (quick iteration)
- **Medium**: 10,000 sequences (typical chunk size)
- **Large**: 100,000 sequences (GPU threshold region)
- **Very Large**: 1,000,000 sequences (production scale)
- **Huge**: 10,000,000+ sequences (whole genome)

**Key insight**: Performance characteristics change dramatically with scale (e.g., GPU cliff at 50K)

#### Read Structure
- **Single-end**: One read per molecule
- **Paired-end**: R1 + R2 per molecule
- **Interleaved**: R1/R2 alternating in single file

#### Sequence Length
- **Short**: 50-150bp (Illumina short)
- **Medium**: 150-1,000bp (Illumina long, Ion Torrent)
- **Long**: 1kb-100kb (PacBio, Nanopore)

#### Quality Score Distribution (FASTQ only)
- **Uniform high**: Q40 across all bases (synthetic)
- **Degrading**: Q40 → Q20 over length (realistic)
- **Bimodal**: High quality + low quality regions
- **Real**: Actual sequencing run distributions

### 2. Operations (Transformation Space)

#### Category 1: Element-Wise Operations
**Characteristics**: Independent per-sequence, highly vectorizable

**Operations**:
- Base counting (A/C/G/T/N counts)
- GC content calculation
- Reverse complement
- DNA to protein translation
- Base masking (replace with N)
- Quality score aggregation (mean, min, max)

**Traditional approach**:
- Scalar loops processing one base at a time
- Optional SIMD via auto-vectorization or intrinsics
- Standard parallel (OpenMP, pthreads)

**Expected hardware** (x86 wisdom):
- ✓ NEON SIMD (high speedup expected, 98× validated for reverse complement)
- ✓ Rayon parallel (scales linearly)
- ✗ GPU (overhead too high for small operations)

**Novel explorations** (Apple Silicon-specific):
- **NEON-native designs**: Algorithms designed around 16-byte lanes (64 bases in 2-bit encoding)
- **NEON lookup tables**: Replace branching with table lookups for translation
- **2-bit encoding**: 4× memory reduction, better cache locality, enables 64-base SIMD
- **Unified memory + GPU**: Even small batches (1K sequences) if zero-copy eliminates overhead
- **P-core + E-core**: Quality aggregation on E-cores while P-cores process next chunk

**Open questions**:
- Does 2-bit encoding always win, or only at large scale?
- Can GPU compete with NEON if unified memory removes overhead?
- Can we design operations that compose NEON + Metal (CPU transforms, GPU reduces)?

#### Category 2: Filtering Operations
**Characteristics**: Data-dependent, may be sequential

**Operations**:
- Quality filtering (min/mean quality thresholds)
- Length filtering (min/max length)
- Complexity filtering (low-complexity detection)
- N-content filtering (max ambiguous bases)

**Traditional approach**:
- Sequential read-by-read processing
- Early termination for failed filters
- Single-threaded or coarse-grained parallel

**Expected hardware** (x86 wisdom):
- ✓ NEON SIMD (quality score processing, 16 scores in parallel)
- ✓ Rayon parallel (independent sequences)
- ? GPU (depends on batch size)

**Novel explorations** (Apple Silicon-specific):
- **NEON predication**: Compare 16 quality scores simultaneously, mask-based filtering
- **Neural Engine classification**: Train model to predict pass/fail from sequence features
- **Pipelined I/O + filtering**: E-cores parse, P-cores filter, E-cores write (3-stage pipeline)
- **Complexity detection via AMX**: Represent sequence as matrix, detect low-rank structure

**Open questions**:
- Can Neural Engine beat rule-based filtering (lower latency, higher accuracy)?
- Does pipelining help or hurt (overhead vs. parallelism)?
- Can we combine filters (quality + length + complexity) into single NEON pass?

#### Category 3: Search Operations
**Characteristics**: Memory-intensive, parallel

**Operations**:
- K-mer extraction
- K-mer counting (spectrum analysis)
- K-mer matching (exact)
- Fuzzy k-mer matching (Hamming distance ≤ 1)
- Motif finding
- Adapter detection

**Traditional approach**:
- Hash tables for k-mer storage
- Sequential counting
- Exhaustive search for fuzzy matching

**Expected hardware** (x86 wisdom):
- ? NEON SIMD (hashing may not vectorize well)
- ✓ Rayon parallel (independent sequences)
- ✓ GPU (for large batches, fuzzy matching validated 6×)

**Novel explorations** (Apple Silicon-specific):
- **Metal tile memory**: Perfect cache locality for k-mer counting (k-mer hash → tile memory)
- **NEON parallel hashing**: Even small speedup (2-3×) is valuable at scale
- **Neural Engine k-mer classification**: Train model to predict k-mer matches (adapter detection as classification)
- **Unified memory streaming**: CPU extracts k-mers, GPU matches simultaneously (no copy)
- **AMX for fuzzy matching**: Reformulate Hamming distance as matrix operations

**Open questions**:
- Can Metal tile memory beat CPU cache for k-mer counting?
- Is Neural Engine faster than exhaustive search for adapter detection?
- Does unified memory enable new streaming patterns (CPU+GPU concurrent)?
- Can AMX accelerate fuzzy matching (matrix formulation)?

#### Category 4: Pairwise Operations
**Characteristics**: O(n²) or O(n log n), may not parallelize well

**Operations**:
- Hamming distance (all-pairs)
- Edit distance (Levenshtein)
- Sequence deduplication
- Clustering (single-linkage, complete-linkage)

**Expected hardware**:
- ✓ NEON SIMD (distance metrics, validated 8-9× for Hamming)
- ✓ Rayon parallel (pairs are independent)
- ? GPU (may not be memory-efficient)

#### Category 5: Aggregation Operations
**Characteristics**: Reducible, highly parallelizable

**Operations**:
- Statistics (mean, median, quantiles)
- Histogram generation
- MinHash sketching
- Random sampling

**Expected hardware**:
- ✓ NEON SIMD (depends on operation)
- ✓ Rayon parallel (reduction patterns)
- ? GPU (simple operations, overhead may dominate)

#### Category 6: I/O Operations
**Characteristics**: Bandwidth-limited, not compute-bound

**Operations**:
- FASTA/FASTQ parsing
- Gzip decompression
- Zstd decompression
- Format conversion (FASTA ↔ FASTQ)
- Writing output

**Traditional approach**:
- Sequential I/O (read, decompress, parse)
- Software decompression (zlib, libdeflate)
- Blocking I/O

**Expected hardware** (x86 wisdom):
- ✗ NEON SIMD (not compute-bound)
- ? Rayon parallel (I/O may serialize)
- ✗ GPU (irrelevant for I/O)
- ✓ E-cores + GCD (background I/O)
- ✓ Hardware compression (AppleArchive/zstd acceleration)

**Novel explorations** (Apple Silicon-specific):
- **AppleArchive hardware acceleration**: 2-5× faster decompression vs. software zlib
- **E-core I/O + P-core processing**: Complete pipeline on E-cores (parse + write), P-cores compute
- **GCD dispatch groups**: Automatic load balancing for parallel decompression
- **Compressed intermediate buffers**: Hardware compression for in-memory data (reduce memory bandwidth)
- **QoS-based prioritization**: Background QoS for I/O, user-initiated for processing

**Open questions**:
- How much faster is hardware decompression vs. software?
- Can we stream compressed data directly to GPU (decompress on GPU)?
- Does E-core I/O + P-core compute pipeline improve throughput?
- Can hardware compression reduce memory bandwidth contention?

### 3. Hardware Configurations (Optimization Space)

#### CPU Features
- **Scalar**: Naive implementation, no SIMD
- **NEON SIMD**: 128-bit vectorization (16 bytes/instruction)
- **Threading**: 1, 2, 4, 8 threads
- **Thread assignment**: P-cores only, E-cores only, mixed

#### Memory Features
- **Encoding**: ASCII (1 byte/base) vs 2-bit (0.25 bytes/base)
- **Unified memory**: CPU/GPU shared memory (zero-copy)
- **Cache optimization**: Data layout, prefetching

#### GPU Features (Metal)
- **Batch processing**: Single dispatch per file
- **Batch size threshold**: Test 100, 1K, 10K, 50K, 100K, 1M
- **Unified memory bandwidth**: Zero-copy CPU↔GPU

#### Specialized Hardware
- **AMX matrix engine**: 512-bit matrix operations
- **Neural Engine**: ML inference (Core ML models)
- **Hardware compression**: AppleArchive, zstd acceleration

#### System Features
- **GCD dispatch**: Quality of Service based threading
- **QoS levels**: Background, Utility, Default, UserInitiated, UserInteractive
- **Power efficiency**: Performance vs battery optimization

---

## Experimental Design

### Approach 1: Hierarchical Testing (RECOMMENDED)

**Rationale**: Test primitives first, then validate compositions. Efficient and principled.

#### Level 1: Primitive Operations (~500 tests, 1 day)

**Goal**: Identify which hardware works for which operation category.

**Design**:
```
Operations: 20 primitives
Hardware configs: 25 key configurations
Data: Medium scale (10K sequences, 150bp)

Total tests: 20 × 25 = 500
```

**Hardware configs tested** (25 total):
1. Baseline: Scalar, single thread, ASCII encoding
2-5. NEON variants: NEON + 1/2/4/8 threads
6-9. Rayon variants: Scalar + 1/2/4/8 threads
10-13. NEON + Rayon: Combined + 1/2/4/8 threads
14. 2-bit encoding: NEON + Rayon + 2-bit
15. GPU: Metal GPU, single dispatch
16-19. P-core variants: NEON + 1/2/4/8 P-cores
20-23. E-core variants: Scalar + 1/2/4/8 E-cores
24. GCD: E-cores for I/O, P-cores for compute
25. Unified memory: Zero-copy CPU↔GPU

**Output**:
- Main effects: Which hardware features matter most?
- Category patterns: Does element-wise always benefit from NEON?
- Initial rules: Rough guidelines per category

#### Level 2: Scaling Analysis (~3,000 tests, 1 week)

**Goal**: Identify performance cliffs and thresholds.

**Design**:
```
Operations: 20 primitives
Hardware configs: 25 configurations
Scales: 6 data sizes (100, 1K, 10K, 100K, 1M, 10M sequences)

Total tests: 20 × 25 × 6 = 3,000
```

**Key questions**:
- Where is the GPU threshold? (Expected ~50K, validate)
- Does NEON speedup depend on data size? (Expected no, validate)
- Do threading benefits diminish at small scale? (Expected yes)
- When does memory encoding matter? (Expected always, validate)

**Output**:
- Threshold rules: "Use GPU if >50K sequences"
- Scaling laws: "NEON speedup constant across scales"
- Cliff visualization: Performance vs data size plots

#### Level 3: Validation (~500 tests, 1 day)

**Goal**: Validate that primitive rules compose correctly.

**Design**:
```
Compound operations: 10 (filter+trim, screen=kmer+match, etc.)
Hardware configs: 50 (including combinations not yet tested)
Data: Various scales

Total tests: 10 × 50 = 500
```

**Validation metrics**:
- Prediction accuracy: How close is predicted performance to actual?
- Composition rules: Do primitive speedups multiply correctly?
- Edge cases: Are there surprising interactions?

**Output**:
- Confidence intervals: "Prediction accuracy 80% ± 5%"
- Refinement: Adjust rules based on validation failures
- Publication-ready: Validated methodology

### Approach 2: Fractional Factorial (Alternative)

**Rationale**: Statistical experimental design, covers interactions efficiently.

**Design**: 2^(15-5) fractional factorial
```
Factors: 15 (operation category, data size, NEON, threads, GPU, etc.)
Resolution: V (main effects + 2-way interactions)
Tests: 2^10 = 1,024
```

**Pros**:
- Rigorous statistical foundation
- Efficient coverage of interaction space
- Standard practice in experimental science

**Cons**:
- Less intuitive than hierarchical
- Requires statistical expertise
- Harder to explain in publication

**When to use**: If hierarchical approach misses critical interactions.

---

## Data Generation

### Synthetic Datasets

**Advantages**:
- Controlled characteristics (length, quality distribution)
- Reproducible (seeded RNG)
- No privacy concerns
- Can stress-test edge cases

**Generation strategy**:
```rust
fn generate_dataset(chars: &DataCharacteristics) -> Vec<SequenceRecord> {
    let mut rng = StdRng::seed_from_u64(FIXED_SEED);

    (0..chars.num_sequences).map(|i| {
        // Generate sequence
        let seq = match chars.format {
            DataFormat::FASTA => generate_fasta_sequence(&mut rng, chars),
            DataFormat::FASTQ => generate_fastq_sequence(&mut rng, chars),
        };

        SequenceRecord::new(format!("seq_{}", i), seq)
    }).collect()
}

fn generate_fastq_sequence(rng: &mut StdRng, chars: &DataCharacteristics) -> BitSeq {
    // Sample length from distribution
    let length = sample_length(rng, chars.seq_length_mean, chars.seq_length_std);

    // Generate bases
    let bases = (0..length)
        .map(|_| sample_base(rng))
        .collect::<Vec<_>>();

    // Generate quality scores
    let quality = match chars.quality_distribution {
        Some(QualityDist::UniformHigh) => vec![b'I'; length],  // Q40
        Some(QualityDist::Degrading) => generate_degrading_quality(length),
        Some(QualityDist::Realistic) => sample_realistic_quality(rng, length),
        None => vec![b'I'; length],
    };

    BitSeq::from_bytes(&bases, Some(&quality)).unwrap()
}
```

**Dataset library** (pre-generated, versioned):
```
datasets/
  ├── small-100/
  │   ├── fasta.fa
  │   ├── fastq-uniform.fq
  │   └── fastq-realistic.fq
  ├── medium-10k/
  ├── large-100k/
  ├── vlarge-1m/
  └── huge-10m/
```

### Real Datasets (Validation)

**Purpose**: Validate that synthetic findings hold on real data.

**Sources**:
- SRA (Sequence Read Archive)
- NCBI RefSeq
- Public metagenomics datasets

**Selection criteria**:
- Representative organisms (E. coli, human, meta)
- Various sequencing platforms (Illumina, PacBio, Nanopore)
- Different read lengths and qualities

**Use sparingly**: Real data for final validation only, not primary experiments.

---

## Performance Measurement

### Metrics Collected

```rust
struct PerformanceResult {
    // Throughput
    throughput_seqs_per_sec: f64,
    throughput_mbps: f64,

    // Latency
    latency_first_result: Duration,  // Time to first output
    latency_p50: Duration,            // Median latency
    latency_p99: Duration,            // 99th percentile

    // Resources
    memory_peak: usize,               // Peak RSS
    memory_avg: usize,                // Average over run
    cpu_utilization: f64,             // % of CPU used
    gpu_utilization: Option<f64>,     // % of GPU used (if applicable)

    // Energy (if measurable)
    energy_joules: Option<f64>,       // Total energy consumed
    power_watts: Option<f64>,         // Average power

    // Correctness
    output_matches_reference: bool,   // Validates correctness
    output_checksum: u64,             // For byte-level comparison
}
```

### Measurement Protocol

**For each experiment**:

1. **Warm-up** (5 iterations):
   - Eliminate cold start effects
   - Populate caches
   - Discard timing results

2. **Measurement** (20 iterations):
   - Collect timing data
   - Monitor resources
   - Validate output

3. **Statistical analysis**:
   - Remove outliers (>3 standard deviations)
   - Compute mean, median, std dev
   - Calculate confidence intervals (95%)

4. **Validation**:
   - Compare output to reference implementation
   - Byte-level checksum comparison
   - Flag any discrepancies

### Correctness Validation

**Reference implementations**: Naive, well-tested versions of each operation.

**Validation strategy**:
```rust
fn validate_output<T: PartialEq>(
    test_output: &T,
    reference_output: &T
) -> bool {
    test_output == reference_output
}
```

**Critical**: Every experiment must pass validation. Performance without correctness is meaningless.

---

## Statistical Analysis

### Main Effects Analysis

**Question**: Which hardware features have the largest impact?

**Method**: ANOVA (Analysis of Variance)
```
Factor         | Effect Size | p-value | Significance
---------------|-------------|---------|-------------
NEON           | 45.2×       | <0.001  | ***
GPU (large)    | 6.1×        | <0.001  | ***
Rayon (4 core) | 1.59×       | <0.001  | ***
2-bit encoding | 1.8×        | 0.002   | **
E-cores        | 1.1×        | 0.15    | ns
```

### Interaction Effects

**Question**: Do hardware features interact? (e.g., NEON + Rayon multiplicative?)

**Method**: Two-way ANOVA
```
Interaction    | Expected  | Observed | Type
---------------|-----------|----------|-------------
NEON × Rayon   | 45 × 1.6  | 70×      | Multiplicative
NEON × GPU     | 45 × 6    | 48×      | Independent (no benefit)
GPU × Small    | 6 × 1     | 0.00004× | Negative (overhead)
```

### Regression Modeling

**Question**: Can we predict performance without running every test?

**Models**:
1. **Linear regression**: Simple, interpretable
2. **Decision tree**: Captures non-linear effects
3. **Random forest**: Ensemble, higher accuracy
4. **Gradient boosting**: State-of-the-art prediction

**Training**:
```python
# Features: operation category, data size, hardware config
X = [
    [op_category, num_seqs, use_neon, num_threads, use_gpu, ...],
    ...
]

# Target: log(speedup)
y = [log(speedup_1), log(speedup_2), ...]

# Train model
model = RandomForestRegressor(n_estimators=100)
model.fit(X_train, y_train)

# Validate
predictions = model.predict(X_test)
r2_score = r2(y_test, predictions)  # Expect >0.8
```

### Rule Extraction

**Question**: How do we turn models into human-readable rules?

**Method**: Decision tree → Rules
```
IF operation_category == ElementWise:
    IF format == FASTQ:
        → Use NEON (expected 10-50× speedup)
    IF num_sequences > 1000:
        → Use Rayon parallel (expected 1.5× speedup)

IF operation_category == Search:
    IF num_sequences > 50000:
        → Use GPU (expected 6× speedup)
    ELSE:
        → Use NEON + Rayon (GPU overhead too high)
```

---

## Validation Strategy

### Cross-Validation

**Method**: 80/20 train/test split
- Train optimization rules on 80% of experiments
- Test prediction accuracy on held-out 20%
- Report accuracy, precision, recall

**Success criterion**: >80% prediction accuracy

### Composition Validation

**Test**: Do primitive rules compose correctly?

**Example**:
```
filter = quality_filter + length_check
Expected speedup = speedup(quality_filter) × speedup(length_check)
Observed speedup = measure_actual(filter)

Validate: |Expected - Observed| / Expected < 20%
```

### Real Data Validation

**Test**: Do synthetic-derived rules work on real sequencing data?

**Method**:
1. Apply rules to real datasets (SRA)
2. Measure actual performance
3. Compare to predictions

**Success criterion**: <30% deviation from predictions

---

## Result Storage & Reproducibility

### Data Format

**Primary storage**: Apache Parquet (columnar, compressed, efficient)

**Schema**:
```
experiment_id: string
timestamp: datetime
operation: string
category: string
data_format: string
num_sequences: int64
seq_length_mean: int64
use_neon: bool
num_threads: int32
use_gpu: bool
throughput: float64
latency_p50: float64
memory_peak: int64
output_valid: bool
...
```

**Advantages**:
- Efficient queries (Polars, DuckDB)
- Compressed (10× smaller than CSV)
- Schema-enforced
- Fast read/write

### Version Control

**Experiments**: Version-controlled protocols
```
experiments/001-primitives/
  ├── protocol.md        # What we're testing
  ├── design.toml        # Configuration
  ├── expected-tests.txt # ~500 tests
  └── results-v1.parquet # Output
```

**Datasets**: Checksums for reproducibility
```
datasets/medium-10k/fastq.fq
  SHA256: a1b2c3d4...
```

**Code**: Git tags for each experiment
```
git tag experiment-001-v1
git push --tags
```

---

## Publication Plan

### Manuscript Structure

**Title**: "Systematic Performance Characterization of Bioinformatics Sequence Operations on Apple Silicon"

**Abstract** (250 words):
- Problem: Bioinformatics tools face combinatorial optimization space
- Approach: Hierarchical experimental design, 4,000 automated tests
- Results: Formal optimization rules, 80%+ prediction accuracy
- Impact: Enables automatic optimization of any sequence tool

**Introduction**:
- Apple Silicon hardware landscape (NEON, GPU, unified memory, etc.)
- Bioinformatics sequence operations (20 primitives, 6 categories)
- Combinatorial explosion (2M configurations)
- Need for systematic approach

**Methods**:
- Experimental design (hierarchical, factorial)
- Data generation (synthetic datasets)
- Performance measurement (throughput, latency, resources)
- Statistical analysis (ANOVA, regression, decision trees)
- Validation strategy (cross-validation, real data)

**Results**:
- Main effects (NEON 10-98×, GPU 6× for large batches)
- Interaction effects (NEON × Rayon multiplicative)
- Performance cliffs (GPU threshold at 50K)
- Optimization rules (decision tree, regression models)
- Prediction accuracy (80%+ validated)

**Discussion**:
- Implications for tool developers
- Generalizability to other ARM platforms
- Limitations and future work
- Comparison to ad-hoc optimization

**Conclusion**:
- Systematic approach enables automatic optimization
- Open framework for community
- Foundation for hardware-aware bioinformatics

**Supplementary**:
- Complete experimental protocols
- All raw data (Parquet files)
- Analysis scripts (Python, R)
- Replication guide

---

## Timeline

**Month 1**: Framework development (infrastructure, operations)
**Month 2**: Data generation, validation setup
**Month 3**: Experimentation (4,000 automated tests)
**Month 4**: Statistical analysis, rule extraction
**Month 5**: Manuscript preparation
**Month 6**: Community release, crates.io publication

---

**Status**: Methodology finalized, ready for implementation

**Last Updated**: October 30, 2025
