# Apple Silicon Bio Bench: Claude Development Guide

**Last Updated**: October 30, 2025

---

## Project Vision

**Apple Silicon Bio Bench (ASBB) is a scientific framework for systematically mapping the performance landscape of bioinformatics sequence operations across Apple Silicon hardware configurations.**

This is not just a benchmarking tool - it's **foundational science** that enables automatic optimization of ANY sequence analysis tool.

---

## The Core Insight

### The Problem

Bioinformatics tools face a combinatorial explosion:
- 20+ primitive operations
- 5 orders of magnitude in data scale (100 → 1B sequences)
- 8+ hardware features (NEON, GPU, P/E-cores, AMX, Neural Engine, etc.)
- **Result**: 2 million potential configurations

**Traditional approach**: Trial and error, ad-hoc optimization, inconsistent results

**ASBB approach**: Systematic experiments → Statistical analysis → Formal rules → Automatic optimization

### The Strategic Pivot

This project emerged from a critical realization during BioMetal development:

**Initial phase** (Months 1-10):
- Built 16 bioinformatics commands
- Optimized `screen` command: 193× speedup (Phases 1-6)
- Learned: NEON gives 98× for reverse complement, GPU gives 6× for large batches
- **Problem**: Technical debt growing, optimizations inconsistent across commands

**The Question** (October 30, 2025):
> "We have learned a lot, but also amassed a fair amount of technical debt and we are not applying what we have learned across all processes. Can we design a robust experimental testing system to systematically explore sequence/hardware space so that we could logically apply rules to all future commands?"

**The Answer**: YES - and this became ASBB.

**The Paradigm Shift**:
- From: Optimize each command individually (engineering)
- To: Map entire performance space systematically (science)
- Result: Universal optimization rules, zero per-command cost

---

## Why This is Valuable

### For This Project

**Eliminates all guesswork**:
- No more "should I try NEON for this?"
- No more "is 10K sequences enough for GPU?"
- **Just query the rules**: "What's optimal for operation X with data Y?"

**Makes future work trivial**:
- Implement naive version of new operation
- Framework auto-optimizes using experimental rules
- **Zero optimization time per operation**

**Eliminates technical debt**:
- All operations optimized consistently
- No ad-hoc optimization decisions
- **Maintainable, systematic architecture**

### For Publication

**This is PhD-level research**:
- **Novel contribution**: First systematic study of bioinformatics + Apple Silicon
- **Generalizable findings**: Rules apply to entire field
- **Reproducible**: Clear methodology, open data
- **High impact**: Enables automatic optimization for community

**Multiple papers possible**:
1. **Methodology paper**: "Systematic Performance Characterization of Sequence Operations on Apple Silicon"
2. **Application paper**: Integration with BioMetal or other tools
3. **Extension papers**: New hardware (M5), new operations, new domains

### For The Field

**Paradigm shift**:
- From ad-hoc optimization → Systematic understanding
- From manual tuning → Automatic optimization
- From intuition → Data-driven decisions

**Community value**:
- Others can apply your rules without re-doing experiments
- Optimization framework portable (not just BioMetal-specific)
- Establishes performance baselines for Apple Silicon bioinformatics

---

## Core Architecture

### The Dimensional Space

**Data Dimensions (Input Space)**:
- Format: FASTA, FASTQ
- Scale: 100 → 1B sequences (5 orders of magnitude)
- Structure: Single-end, paired-end, interleaved
- Length: 50bp → 100kb (short/medium/long reads)
- Quality (FASTQ): Distribution, encoding

**Operation Dimensions (Transformation Space)**:

1. **Element-wise** (independent, vectorizable):
   - Base counting, GC content, quality aggregation, reverse complement, translation, masking

2. **Filtering** (sequential, data-dependent):
   - Quality filtering, length filtering, complexity filtering, adapter detection

3. **Search** (parallel, memory-intensive):
   - K-mer extraction, k-mer counting, k-mer matching (exact/fuzzy), motif finding

4. **Pairwise** (O(n²) or O(n log n)):
   - Hamming distance, edit distance, alignment, deduplication, clustering

5. **Aggregation** (reducible, parallelizable):
   - Statistics, histograms, sketching (MinHash), sampling

6. **I/O** (bandwidth-limited):
   - Parsing, decompression, format conversion, writing

**Hardware Dimensions (Optimization Space)**:
- CPU: Scalar, NEON SIMD, Threading (1-8 cores), P-core vs E-core
- Memory: ASCII vs 2-bit encoding, unified memory, cache optimization
- GPU: Metal compute shaders, batch processing, unified memory
- Specialized: AMX matrix engine, Neural Engine, hardware compression
- System: GCD dispatch, QoS-based threading, power efficiency

### Experimental Design: Hierarchical Approach

**Level 1: Primitives** (~500 tests, 1 day automated)
```
Test: 20 core operations × 25 hardware configurations
Goal: Identify which hardware works for which operation category
Output: Main effects (NEON good for element-wise, GPU bad for small batches)
```

**Level 2: Scaling** (~3,000 tests, 1 week automated)
```
Test: 20 operations × 25 configs × 6 scales (100, 1K, 10K, 100K, 1M, 10M)
Goal: Identify performance cliffs and thresholds
Output: GPU cliff at 50K sequences, NEON universal across scales
```

**Level 3: Validation** (~500 tests, 1 day)
```
Test: 10 compound operations × 50 configs
Goal: Validate that primitive rules compose correctly
Output: Confidence intervals, prediction accuracy
```

**Total**: ~4,000 experiments, fully automated, highly tractable

---

## Key Design Principles

### 1. Primitives Compose

Commands are compositions of ~20 primitives:
```
filter = quality_filter + length_check + write_output
screen = kmer_extract + hamming_search + count_matches
trim = quality_filter + trim_operation + write_output
```

**If you characterize primitives, you can predict compositions.**

This reduces the problem from 2M test cases to ~4,000 tractable experiments.

### 2. Statistical Experimental Design

**Not all combinations need testing:**
- Use **fractional factorial design** to cover interactions efficiently
- Use **hierarchical approach** to test primitives first, compositions later
- Use **adaptive sampling** to focus on interesting regions

**Example**: 2^(15-5) fractional factorial = 1,024 experiments covers:
- All main effects
- All 2-way interactions
- Some 3-way interactions

**This is standard practice in experimental science** (chemistry, materials, etc.)

### 3. Hardware Interactions Are Limited

Not all hardware combinations interact meaningfully:
- NEON + Rayon: Roughly multiplicative (independent)
- GPU + small data: Always fails (overhead dominant)
- E-cores + I/O: Natural pairing (both background tasks)

**Most of the space is boring** (follows main effects)
**Interesting regions are sparse** (can be found with sampling)

### 4. Apple Silicon Is Consistent

M1, M2, M3, M4 have similar performance characteristics:
- NEON speedups transfer across generations
- GPU cliffs at similar thresholds
- Unified memory works the same way

**Rules likely generalize across generations** (validate with spot checks)

---

## Implementation Philosophy

### Core Types (asbb-core)

```rust
/// Represents characteristics of input data
struct DataCharacteristics {
    format: DataFormat,           // FASTA, FASTQ
    num_sequences: usize,         // 100 to 10M
    seq_length_mean: usize,       // 50 to 100k
    seq_length_std: usize,        // Variation
    read_type: ReadType,          // Single, Paired
    quality_distribution: Option<QualityDist>, // For FASTQ
}

/// Hardware configuration for experiment
struct HardwareConfig {
    use_neon: bool,
    num_threads: usize,
    thread_assignment: ThreadAssignment,  // P-core, E-core, mixed
    encoding: Encoding,                   // TwoBit, ASCII
    use_unified_memory: bool,
    use_gpu: bool,
    gpu_batch_size: Option<usize>,
    use_amx: bool,
    use_neural_engine: bool,
    use_hw_compression: bool,
    use_gcd: bool,
    qos: QualityOfService,
}

/// Performance results from one experiment
struct PerformanceResult {
    throughput_seqs_per_sec: f64,
    throughput_mbps: f64,
    latency_first_result: Duration,
    latency_p50: Duration,
    latency_p99: Duration,
    memory_peak: usize,
    memory_avg: usize,
    cpu_utilization: f64,
    gpu_utilization: Option<f64>,
    energy_joules: Option<f64>,
    output_matches_reference: bool,
}

/// Primitive operation trait
trait PrimitiveOperation {
    fn name(&self) -> &str;
    fn category(&self) -> OperationCategory;

    // Multiple backend implementations
    fn execute_naive(&self, data: &[SequenceRecord]) -> Result<Output>;
    fn execute_neon(&self, data: &[SequenceRecord]) -> Result<Output>;
    fn execute_parallel(&self, data: &[SequenceRecord], threads: usize) -> Result<Output>;
    fn execute_gpu(&self, data: &[SequenceRecord], backend: &MetalBackend) -> Result<Output>;
}
```

### Experimental Harness (asbb-explorer)

```rust
struct SequenceHardwareExplorer {
    operations: Vec<Box<dyn PrimitiveOperation>>,
    hardware_profile: HardwareProfile,
    experimental_design: ExperimentalDesign,
    results: HashMap<(String, DataCharacteristics, HardwareConfig), PerformanceResult>,
}

impl SequenceHardwareExplorer {
    /// Design experiments using statistical methods
    fn design_experiments(&mut self) { /* ... */ }

    /// Run all experiments (parallelized, automated)
    fn run_all_experiments(&mut self) -> Result<()> { /* ... */ }

    /// Analyze results and derive optimization rules
    fn derive_rules(&self) -> OptimizationRules { /* ... */ }

    /// Validate rules on held-out test cases
    fn validate_rules(&self, rules: &OptimizationRules) -> ValidationReport { /* ... */ }
}
```

### Optimization Rules (asbb-rules)

```rust
/// Auto-generated from experimental data
pub struct OptimizationRules {
    decision_tree: DecisionTree,
    regression_models: HashMap<OperationCategory, RegressionModel>,
    lookup_table: HashMap<(Operation, DataScale), HardwareConfig>,
}

impl OptimizationRules {
    /// Predict best hardware configuration
    pub fn optimize(
        &self,
        operation: &Operation,
        data: &DataCharacteristics,
        hardware: &HardwareProfile
    ) -> HardwareConfig {
        // Decision tree based on experimental data
        // Returns optimal config with high confidence
    }

    /// Predict performance without running
    pub fn predict_performance(
        &self,
        operation: &Operation,
        data: &DataCharacteristics,
        config: &HardwareConfig
    ) -> PerformancePrediction {
        // Trained regression model
    }

    /// Explain why a configuration was chosen
    pub fn explain(&self, config: &HardwareConfig, ...) -> String {
        // Human-readable reasoning
    }
}
```

---

## Relationship to BioMetal

ASBB and BioMetal are **separate but synergistic**:

### ASBB (This Repo)
- **Purpose**: Scientific framework for optimization research
- **Audience**: Researchers, tool developers
- **Output**: Performance data, optimization rules
- **Lifecycle**: Continuous discovery (new hardware, new operations)

### BioMetal
- **Purpose**: Practical bioinformatics toolkit
- **Audience**: Bioinformaticians doing sequence analysis
- **Output**: Analyzed sequences, filtered data, statistics
- **Lifecycle**: Feature additions, user feedback, bug fixes

### Dependency Flow

```
ASBB (independent)
  ↓ produces
optimization-rules.json (published artifact)
  ↓ consumed by
BioMetal (depends on published rules)
```

**ASBB doesn't need BioMetal.**
**BioMetal benefits from ASBB.**

### Integration Example

**BioMetal Cargo.toml**:
```toml
[dependencies]
asbb-rules = "0.1"  # Published crate with optimization rules
```

**BioMetal command**:
```rust
use asbb_rules::{OptimizationRules, Operation, OperationCategory};

pub fn run_filter(args: &FilterArgs) -> Result<()> {
    let operation = Operation {
        name: "quality_filter".to_string(),
        category: OperationCategory::Filter,
    };

    let data_chars = DataCharacteristics::from_file(&args.input)?;
    let hardware = HardwareProfile::detect()?;

    // Query ASBB optimization rules
    let config = OptimizationRules::default()
        .optimize(&operation, &data_chars, &hardware);

    // Apply optimal configuration (automatic!)
    execute_with_config(&args, &config)?;
    Ok(())
}
```

**BioMetal gets automatic optimization without implementing experimental framework.**

---

## Development Workflow

### Phase 1: Framework Development (Current, Months 1-2)

**Week 1: Core Infrastructure**
- Define core types (DataCharacteristics, HardwareConfig, PerformanceResult)
- Implement PrimitiveOperation trait
- Set up result storage (Parquet format)

**Week 2: Data Generation**
- Synthetic FASTA/FASTQ generation
- Realistic quality score distributions
- Reproducible (seeded RNG)

**Week 3: Benchmarking Harness**
- Automated experiment execution
- Correctness validation
- Resource monitoring (CPU, GPU, memory, energy)

**Week 4: Operations**
- Implement 5-10 primitive operations
- Multiple backends (naive, NEON, parallel, GPU)
- Validation tests

### Phase 2: Experimentation (Month 3)

**Weeks 1-2: Level 1 Experiments**
- Run primitive tests (~500 experiments)
- Analyze main effects
- Identify operation categories

**Weeks 3-4: Level 2 Experiments**
- Run scaling tests (~3,000 experiments)
- Identify performance cliffs
- Measure interaction effects

### Phase 3: Analysis & Publication (Months 4-5)

**Weeks 1-2: Statistical Analysis**
- Regression modeling
- Decision tree extraction
- Cross-validation

**Week 3: Rule Implementation**
- Codify rules in `asbb-rules` crate
- Validation framework
- Prediction accuracy testing

**Week 4: Documentation**
- Methodology paper
- Integration guide
- Example usage

### Phase 4: Community Release (Month 6)

- Publish `asbb-rules` crate to crates.io
- Documentation website
- Integration examples (BioMetal, other tools)
- Tutorial videos

---

## Expected Findings (Based on BioMetal Experience)

### Validated Patterns from BioMetal

These findings from BioMetal will be systematically validated and formalized:

**NEON SIMD** (Expected universal for element-wise):
- Reverse complement: 98× speedup (validated)
- Base counting: 85× speedup (validated)
- Quality filtering: 10-16× expected
- GC content: 45× (validated)
- Hamming distance: 8-9× (validated)

**Metal GPU** (Expected for batch >50K only):
- Fuzzy k-mer matching: 6× for 100K+ reads (validated)
- Below 50K: 25,000× SLOWER (GPU overhead cliff, validated)
- Pattern: 50-100ms overhead per dispatch

**Rayon Parallel** (Expected 1.5-1.6× on M4):
- 4 P-cores yield 91% efficiency (validated)
- Diminishing returns beyond 4 threads

**2-bit Encoding** (Expected 4× memory, enables other optimizations):
- Validated in BioMetal
- Improves cache locality

### New Explorations

**AMX Matrix Engine**:
- Hypothesis: May accelerate alignment (Smith-Waterman as matrix operation)
- Expected: Limited benefit (sequence ops not matrix-heavy)
- Experiments: Test alignment, assembly operations

**Neural Engine**:
- Hypothesis: ML-based contamination classification
- Expected: Novel capability, not raw speedup
- Experiments: Core ML model for sequence classification

**Hardware Compression** (AppleArchive/zstd):
- Hypothesis: 2-5× I/O speedup
- Expected: High impact for streaming
- Experiments: Test with various compression formats

---

## Key Lessons from BioMetal Journey

### Lesson 1: GPU Has a "Batch Size Cliff"

**January 2025**: GPU was 196× slower for 100 sequences → CPU pivot
**October 2025**: GPU delivered 6× speedup for 100K-5M reads
**Pattern**: GPU overhead (50-100ms) dominates below ~50K sequences

**ASBB application**: Systematically measure cliff across operations, derive formal threshold rule

### Lesson 2: NEON Is Nearly Universal for Element-Wise Ops

**Pattern across BioMetal**:
- Any operation processing bases/quality scores independently benefits
- 10-98× speedups validated
- Works across all data scales

**ASBB application**: Validate universality, identify exceptions, codify rule

### Lesson 3: "Streaming" Is the Killer Feature

**BioMetal insight**: Ability to process 50GB files on laptop without loading into memory
**Not about raw speed**: Enables laptop analysis that previously required HPC
**User experience**: Immediate results, iterative exploration, no queue waiting

**ASBB application**: Ensure rules optimize for streaming workloads (1K-10K chunks)

### Lesson 4: Optimization Patterns Compose

**BioMetal screen command**: 193× total speedup
- Phase 1: u64 k-mers (4.2×)
- Phase 3: Early termination (1.43×)
- Phase 5: Rayon parallel (1.59×)
- Phase 6: Metal GPU (6×)
- **Total: 4.2 × 1.43 × 1.59 × 6 ≈ 57× theoretical** (accounting for overhead)

**Implication**: If primitives are optimized, compositions benefit automatically

**ASBB application**: Validate composition rules, ensure predictions account for overhead

---

## What NOT to Do

### Anti-Pattern 1: Over-Optimize Too Early

**Don't**: Try to optimize every operation before running experiments
**Do**: Implement naive versions, let experiments guide optimization

**Rationale**: You don't know what will matter until you measure

### Anti-Pattern 2: Test Everything

**Don't**: Run all 2 million possible configurations
**Do**: Use factorial design, hierarchical testing, adaptive sampling

**Rationale**: Statistical methods give 80% of insight with 1% of tests

### Anti-Pattern 3: Ignore Failed Experiments

**Don't**: Only report successes (GPU works here!)
**Do**: Document failures (GPU 25,000× slower there!)

**Rationale**: Negative results are just as valuable (prevent future mistakes)

### Anti-Pattern 4: Treat Rules as Absolute

**Don't**: "NEON always gives 98× speedup"
**Do**: "NEON gives 98× for reverse complement, 16× for quality filtering (95% CI)"

**Rationale**: Confidence intervals and exceptions matter

---

## Success Criteria

### Technical

- [ ] 4,000+ experiments run successfully
- [ ] All results validated (output matches reference)
- [ ] Statistical significance (p < 0.05 for main effects)
- [ ] Prediction accuracy >80% on held-out test set
- [ ] Rules published as crates.io package

### Scientific

- [ ] Novel methodology (fractional factorial for hardware/sequence space)
- [ ] Comprehensive coverage (20 operations, 5 scales, 8 hardware features)
- [ ] Reproducible (protocols documented, data published)
- [ ] Generalizable (rules apply to tools beyond BioMetal)

### Community

- [ ] Open data (all experimental results published)
- [ ] Reusable (others can apply rules without re-running experiments)
- [ ] Extensible (clear path to add new operations, hardware)
- [ ] Documented (methodology, integration guide, examples)

---

## Timeline

### Realistic Estimate

**Months 1-2**: Framework development (infrastructure, operations)
**Month 3**: Experimentation (4,000 automated tests)
**Month 4**: Analysis (statistical analysis, rule extraction)
**Month 5**: Publication (manuscript, documentation, examples)
**Month 6**: Community release (crates.io, website, tutorials)

**Total: 6 months to complete framework**

**Then: Continuous value** - new hardware, new operations added incrementally

---

## Citation

When using ASBB or referencing this work:

```bibtex
@software{asbb2025,
  title = {Apple Silicon Bio Bench: Systematic Performance Characterization of Sequence Operations},
  author = {Handley, Scott and {Claude AI}},
  year = {2025},
  url = {https://github.com/yourusername/apple-silicon-bio-bench}
}
```

*(Update upon publication)*

---

## For Claude (AI Development Notes)

### Context Continuity

This project was created from a strategic pivot during BioMetal development. Key context:

1. **10 months of BioMetal optimization** (January - October 2025)
2. **Screen command**: 193× speedup through 6 optimization phases
3. **GPU checkpoint decision**: Determined GPU works for batch (>50K), fails for streaming
4. **Technical debt recognition**: Inconsistent optimization across 16 commands
5. **Strategic question**: "Can we systematize this?" → YES → ASBB

### Key Documents to Read First

When starting new session:
1. Read `CLAUDE.md` (this file) - strategic context
2. Read `METHODOLOGY.md` - experimental design details
3. Read `README.md` - project overview
4. Review `experiments/001-primitives/protocol.md` - current experiment

### Development Philosophy

**Scientific rigor over speed**:
- Proper experimental design (factorial, not brute force)
- Statistical validation (confidence intervals, cross-validation)
- Reproducible (version protocols, seed RNG, publish data)
- Honest (report failures, document limitations)

**Practical utility**:
- Rules must be actionable (not just interesting)
- Integration must be trivial (one crate dependency)
- Explanations must be clear (why this config?)
- Performance predictions must be accurate (80%+ validated)

### When in Doubt

**Ask**: "Does this serve the systematic exploration goal?"
- If yes: Proceed with rigor
- If no: Defer or remove

**Remember**: This is science, not engineering. The goal is **universal understanding**, not one-off solutions.

---

**Status**: Framework design complete, ready for implementation
**Next**: Begin Phase 1 implementation (core infrastructure)

**Last Updated**: October 30, 2025
