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
- Built 16 bioinformatics commands with various optimizations
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

## CRITICAL: Development Methodology - Stay The Course

### The Systematic Pilot Approach (DO NOT SKIP)

**This project's success depends on EXHAUSTIVE individual dimension testing BEFORE automation.**

### Why Individual Pilots Matter

**What we've learned from systematic testing**:
- NEON dimension (10 ops × 6 scales) revealed **NEON effectiveness predicts GPU benefit**
- 2-bit encoding dimension revealed **encoding affects vectorizability dramatically**
- GPU dimension (4 ops × 8 scales) revealed **complexity threshold at 0.55-0.60**

**Every dimension tested exhaustively reveals unexpected patterns.** This is not wasted effort - it's the core science.

### What "Exhaustive Testing" Means

For EACH hardware dimension, test:
- **All 10 operations** (or representative subset across complexity spectrum)
- **Multiple configurations** (e.g., 1/2/4/8 threads for parallelism)
- **6 scales**: 100, 1K, 10K, 100K, 1M, 10M sequences
- **Document patterns** with detailed analysis (like GPU/NEON docs)

**Typical pilot scope**: 10 operations × 4 configs × 6 scales = **240 experiments per dimension**

### Dimensions Requiring Individual Pilots

**Status as of October 31, 2025**:

✅ **COMPLETED**:
1. NEON SIMD (10 operations × 6 scales = 60 experiments)
2. 2-bit Encoding (2 operations × 6 scales = 12 experiments)
3. GPU Metal (4 operations × 8 scales = 32 experiments)

⏳ **REMAINING** (DO NOT SKIP THESE):
4. **Parallel/Threading** (1/2/4/8 threads, P-core vs E-core)
5. **AMX Matrix Engine** (for applicable operations)
6. **Neural Engine** (ML-based approaches, Core ML)
7. **Hardware Compression** (AppleArchive framework)
8. **GCD/QoS** (Grand Central Dispatch optimization)
9. **M5 GPU Neural Accelerators** (if available)

### DO NOT Jump to Level 1/2 Until All Pilots Complete

**Anti-pattern to avoid**:
> "We have enough data, let's build the automated harness now and test everything at once"

**Why this is wrong**:
- Individual pilots reveal unexpected patterns (NEON effectiveness, complexity thresholds)
- Automation without understanding leads to data without insight
- Each dimension deserves the same exhaustive treatment that found GPU patterns

### Correct Sequence

1. ✅ Design and implement 10 primitive operations
2. ✅ Test NEON dimension exhaustively → Document findings
3. ✅ Test 2-bit encoding exhaustively → Document findings
4. ✅ Test GPU dimension exhaustively → Document findings
5. ⏳ **Test parallel/threading exhaustively → Document findings** ← **CURRENT**
6. ⏳ Test AMX exhaustively → Document findings
7. ⏳ Test Neural Engine exhaustively → Document findings
8. ⏳ Test hardware compression exhaustively → Document findings
9. ⏳ Test GCD/QoS exhaustively → Document findings
10. ✅ **THEN** build Level 1/2 automated harness
11. ✅ **THEN** run full factorial experiments
12. ✅ **THEN** statistical analysis and rule extraction

### For Claude: Reminders

**When tempted to suggest "moving on to Level 1/2"**:
- ❌ STOP - Check if all individual pilots are complete
- ❌ Don't suggest automation until ALL dimensions tested
- ✅ Suggest the NEXT individual pilot instead
- ✅ Use the same exhaustive approach (N ops × M configs × K scales)

**When user asks "what's next after GPU?"**:
- ✅ Answer: "Parallel/threading dimension pilot with same exhaustive testing"
- ❌ Don't answer: "Let's build the Level 1/2 automated harness"

**Success pattern**:
```
Individual Pilot → Unexpected Pattern Discovered → Document Thoroughly → Next Pilot
```

**This approach has worked perfectly. Don't abandon it.**

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

## Critical Philosophy: Think Apple Silicon First

### The Trap of Traditional Thinking

**Lesson from BioMetal development**: One of the biggest challenges was repeatedly falling back into traditional bioinformatics optimization patterns developed for x86 architectures.

**The problem**:
- Most bioinformatics tools were designed pre-Apple Silicon (pre-2020)
- Traditional approaches optimized for x86 SSE/AVX, discrete GPUs, separate memory spaces
- These patterns are **not optimal** for Apple Silicon's unique architecture
- We unconsciously carried forward assumptions that no longer hold

**The directive**:
- ✓ **Learn from** traditional bioinformatics wisdom (algorithms, data structures, domain knowledge)
- ✗ **Don't blindly copy** optimization strategies from x86-era tools
- ✓ **Explore novel** approaches that leverage Apple Silicon's unique capabilities
- ✓ **Question assumptions** about what's "obviously" fast or slow

### Apple Silicon's Unique Opportunities

These capabilities **did not exist** in traditional bioinformatics tool development:

#### 1. Unified Memory Architecture
**Traditional**: Copy data to GPU, process, copy back (huge overhead)
**Apple Silicon**: CPU and GPU share memory (zero-copy)

**Novel opportunities**:
- Stream processing where CPU and GPU read same buffer
- CPU preprocesses data in place, GPU operates directly on it
- No batch size minimum due to copy overhead
- Could we pipeline CPU→GPU without explicit data transfer?

#### 2. NEON as First-Class Citizen
**Traditional**: x86 SSE/AVX as afterthought, often via libraries
**Apple Silicon**: NEON is primary SIMD, consistently fast, well-integrated

**Novel opportunities**:
- Design algorithms NEON-first, not scalar-first
- Use NEON lookup tables instead of branching
- Exploit 128-bit operations for DNA (16 bytes = 64 bases in 2-bit encoding)
- Could we design new sequence representations optimized for NEON lanes?

#### 3. Neural Engine & GPU Neural Accelerators
**Traditional**: No equivalent hardware in x86 or traditional HPC
**Apple Silicon**:
- **Neural Engine**: 16-core (11 TOPS on M1, 38 TOPS on M4)
- **M5 NEW**: Neural Accelerators integrated into each GPU core (4× AI performance vs M4)

**Novel opportunities**:
- Sequence classification (contamination, quality prediction, taxonomy)
- Quality score prediction from sequence context
- Adapter detection as image recognition problem (visualize k-mer matrix)
- Could we train Core ML models for fuzzy matching instead of exhaustive search?
- **M5**: GPU Neural Accelerators blur line between compute shaders and ML inference
- **M5**: Frame operations as ML problems to leverage GPU Neural Accelerators (4× faster AI workloads)
- **M5**: Compare Neural Engine vs GPU Neural Accelerators for sequence operations

#### 4. Heterogeneous Compute (P-cores + E-cores)
**Traditional**: Homogeneous cores, scale linearly
**Apple Silicon**: Performance cores (P) + Efficiency cores (E) with QoS-based dispatch

**Novel opportunities**:
- I/O on E-cores (background QoS), processing on P-cores (user-initiated QoS)
- Pipeline architecture: E-cores parse, P-cores process, E-cores write
- Could we use GCD to automatically optimize thread placement?
- Thermal-aware processing: shift to E-cores when throttling detected

#### 5. AMX (Apple Matrix Coprocessor)
**Traditional**: GPUs for matrix operations, but different programming model
**Apple Silicon**: 512-bit matrix operations, integrated with CPU

**Novel opportunities**:
- Sequence alignment as matrix operations (Smith-Waterman, Needleman-Wunsch)
- Multiple sequence alignment (MSA) as batched matrix multiply
- Could we reformulate k-mer counting as matrix operations?
- Position weight matrix (PWM) scoring with AMX

#### 6. Metal Compute Shaders & GPU Neural Accelerators
**Traditional**: CUDA/OpenCL with separate memory spaces
**Apple Silicon**: Metal with unified memory, tile memory, threadgroup shared memory
- **M5 NEW**: Neural Accelerators in each GPU core (tensor core-like functionality)

**Novel opportunities**:
- Use tile memory for k-mer counting (perfect cache locality)
- Threadgroup barriers for collaborative filtering
- Could we design operations specifically for Metal's memory hierarchy?
- Metal Performance Shaders (MPS) for standard operations
- **M5**: GPU Neural Accelerators enable hybrid compute+ML shaders
- **M5**: Frame sequence operations as ML problems to leverage 4× AI performance
- **M5**: Test GPU Neural Accelerators vs Neural Engine for sequence classification

#### 7. Hardware Compression/Decompression
**Traditional**: Software zlib/gzip (slow, CPU-intensive)
**Apple Silicon**: AppleArchive framework with hardware acceleration

**Novel opportunities**:
- Compress intermediate results on-the-fly (zero CPU cost)
- Stream processing with transparent compression
- Could we design file formats optimized for hardware compression?
- Memory bandwidth optimization via compressed in-memory buffers

#### 8. System-Level Integration (GCD, QoS)
**Traditional**: Fight with OS for resources, manual thread management
**Apple Silicon**: Embrace Grand Central Dispatch, Quality of Service

**Novel opportunities**:
- Mark k-mer indexing as background task → OS optimizes power/thermal
- Use dispatch barriers for coordination instead of explicit locks
- Could we design tools that cooperate with macOS power management?
- Automatic adaptation to battery vs. plugged-in states

### Experimental Mindset for ASBB

**For every operation, ask**:
1. How would a traditional x86 tool approach this?
2. What Apple Silicon features could we leverage instead?
3. Are we making assumptions that no longer hold?
4. What novel approaches become possible?

**Examples**:

**Bad (traditional thinking)**:
> "K-mer counting needs hash tables. Hash tables don't vectorize well. Skip NEON."

**Good (Apple Silicon thinking)**:
> "K-mer counting traditionally uses hash tables, but could we:
> - Use NEON for parallel hash computation (even small speedup adds up)?
> - Use Metal tile memory for perfect k-mer cache locality?
> - Use AMX if we represent k-mers as small matrices?
> - Use Neural Engine if we frame it as classification (k-mer present/absent)?
> Let's test all approaches and measure."

**Bad (traditional thinking)**:
> "Quality filtering is sequential (read-by-read). No parallelism possible."

**Good (Apple Silicon thinking)**:
> "Quality filtering processes sequences independently. Could we:
> - Use NEON for parallel quality score comparison (16 scores at once)?
> - Use E-cores for I/O + P-cores for filtering (pipelined)?
> - Use GCD dispatch groups for automatic work distribution?
> Let's test configurations and measure."

### Integration into ASBB Experiments

**Every experiment should explore**:
- Traditional approach (baseline)
- NEON-native approach (designed for SIMD, not ported)
- Metal-native approach (leverage tile memory, threadgroups)
- Heterogeneous approach (P-cores + E-cores + GCD)
- Novel approach (Neural Engine, AMX, etc.)

**Document not just what works, but what we tried**:
- "Neural Engine for k-mer classification: 0.8× slower (model overhead)"
- "AMX for alignment: 2.1× faster than NEON (matrix operations fit well)"
- "E-core I/O pipelining: 1.3× faster (reduced P-core blocking)"

### Publication Impact

This philosophy differentiates ASBB from "we ported x86 tools to ARM":

**Traditional paper**: "We benchmarked BLAST on Apple Silicon"
**ASBB paper**: "We discovered novel optimization strategies impossible on x86"

**Value**: Not just performance numbers, but **new ways of thinking** about sequence analysis on modern hardware.

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

### 4. Apple Silicon Evolution & Generalization

**M1 → M4 (2020-2024)**:
- Similar performance characteristics across generations
- NEON speedups transfer consistently
- GPU cliffs at similar thresholds
- Unified memory architecture consistent

**M5 (October 2025) - Significant Evolution**:
- **Neural Accelerators in GPU cores**: 4× AI performance vs M4 (NEW capability)
- **Memory bandwidth**: 153 GB/s (vs 120 GB/s M4, 27.5% increase)
- **SSD performance**: 2× faster storage
- **Process**: Third-gen 3nm (N3P)

**Implications for ASBB**:
- **Rules likely generalize** across M1-M4 (validate with spot checks)
- **M5 requires new experiments**: GPU Neural Accelerators are a paradigm shift
- **Forward compatibility**: Design experiments to detect architectural changes
- **Test on multiple generations**: M1/M2/M3/M4 (consistent), M5 (validate new capabilities)

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

**Don't**: "NEON always gives the same speedup for all operations"
**Do**: "NEON speedup varies by operation complexity (measured range: 1-85×, 95% CI)"

**Rationale**: Confidence intervals and operation-specific measurements matter

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
2. Recognition that optimization knowledge was inconsistent across commands
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

**Apple Silicon-first thinking** (CRITICAL):
- **Resist x86 assumptions**: Traditional bioinformatics patterns may not apply
- **Explore novel approaches**: Unified memory, Neural Engine, AMX, heterogeneous cores
- **Question everything**: "How would x86 do this?" → "What does Apple Silicon enable?"
- **Document failures**: Neural Engine 0.8× slower is valuable knowledge
- **For every operation**: Test traditional + NEON-native + Metal-native + heterogeneous + novel
- **Remember**: Most bioinformatics tools were designed pre-2020, pre-Apple Silicon

### When in Doubt

**Ask**: "Does this serve the systematic exploration goal?"
- If yes: Proceed with rigor
- If no: Defer or remove

**Remember**: This is science, not engineering. The goal is **universal understanding**, not one-off solutions.

---

## Current Session Status (October 31, 2025 - Evening)

### What We Accomplished This Session

**GPU Dimension Testing - COMPLETE ✅**:

1. **4 GPU pilots completed** (systematic testing across complexity spectrum):
   - Base Counting (complexity 0.40, 8 scales tested)
   - Reverse Complement (complexity 0.45, 8 scales tested)
   - Quality Aggregation (complexity 0.50, 8 scales tested)
   - **Complexity Score (complexity 0.61, 8 scales tested) - FIRST GPU WIN!**

2. **GPU infrastructure built**:
   - `crates/asbb-gpu/` - Complete Metal backend
   - 7 GPU kernels implemented (base counting, GC, AT, reverse complement, quality, complexity, filters)
   - Unified memory architecture validated (zero transfer overhead)

3. **Breakthrough finding**:
   - **Complexity score shows 2-3× GPU speedup** for batches >10K sequences
   - GPU cliff threshold identified at 10K sequences
   - GPU wins when: NEON ineffective (<2× speedup) AND high complexity (>0.55) AND batch >10K

**Phase 2 (2-bit Encoding) - EXPERIMENTAL COMPLETE ✅**:

1. **Pure 2-bit infrastructure** (BitSeq):
   - Implemented pure 2-bit reverse_complement() (no ASCII roundtrip)
   - Algorithm: Extract → Reverse → Complement (XOR 0b11) → Re-pack
   - Handles partial bytes correctly
   - 12/12 tests passing ✅

2. **2-bit backends for 4 operations**:
   - base_counting.rs: `execute_2bit_naive()` + `execute_2bit_neon()`
   - gc_content.rs: `execute_2bit_naive()` + `execute_2bit_neon()`
   - at_content.rs: `execute_2bit_naive()` + `execute_2bit_neon()`
   - reverse_complement.rs: `execute_2bit_naive()` + `execute_2bit_neon()`
   - 50+ tests passing across all operations ✅

3. **Pilot program created**: `asbb-pilot-2bit`
   - Tests ASCII vs 2-bit across 6 scales (100 → 10M sequences)
   - Tests 2 operation categories (transform, counting)
   - Measures encoding benefit objectively

**Phase 2 EXPERIMENTAL RESULTS - UNEXPECTED FINDINGS 🔬**:

Ran systematic experiments comparing ASCII vs 2-bit encoding:

**Reverse Complement (Transform Operation)**:
- Tiny (100 seqs):    2-bit NEON **0.23×** ASCII NEON (4.4× slower)
- Small (1K):         2-bit NEON **0.22×** ASCII NEON (4.5× slower)
- Medium (10K):       2-bit NEON **0.25×** ASCII NEON (4.0× slower)
- Large (100K):       2-bit NEON **0.27×** ASCII NEON (3.7× slower)
- VeryLarge (1M):     2-bit NEON **0.28×** ASCII NEON (3.6× slower)
- Huge (10M):         2-bit NEON **0.56×** ASCII NEON (1.8× slower)

**Base Counting (Counting Operation)**:
- Consistent pattern: 2-bit NEON **~0.4×** ASCII NEON (~2.5× slower)
- Pattern holds across all scales (100 → 10M sequences)

**Key Discovery**: 2-bit encoding incurs 2-4× overhead in **isolated operations** due to:
- ASCII → 2-bit conversion (input processing)
- 2-bit → ASCII conversion (output generation)
- Current scalar implementation (not yet fully NEON optimized)

This is a VALUABLE finding: encoding overhead matters significantly when operations are isolated (convert in, convert out per operation). The overhead dominates any potential algorithmic benefit for single operations.

**Files Created/Modified** (This Session):
- `crates/asbb-core/src/encoding.rs` (+140 lines, pure 2-bit reverse complement)
- `crates/asbb-ops/src/base_counting.rs` (+120 lines, 2-bit backends)
- `crates/asbb-ops/src/gc_content.rs` (+115 lines, 2-bit backends)
- `crates/asbb-ops/src/at_content.rs` (+110 lines, 2-bit backends)
- `crates/asbb-ops/src/reverse_complement.rs` (+95 lines, 2-bit backends)
- `crates/asbb-cli/src/pilot_2bit.rs` (330 lines, new pilot program)
- **Total**: ~910 lines of production code + tests

**Test Status**: ✅ 50+ tests passing (12 encoding + 38 operation tests)

### Key Findings from N=10 Validation

**Complexity-Speedup Relationship**:
- Linear model R² = 0.536 (explains 54% of variance)
- Prediction accuracy: 72.2% within 20% error (practically useful!)
- Model equation: `speedup ≈ 19.69 - 6.56×complexity - 8.20×log10(scale)`

**Operation Categories by NEON Benefit**:
- **Very simple** (0.20-0.25): 1.0× NEON (lower bound, not worth implementing)
- **Simple counting** (0.30-0.40): 10-50× NEON (peak benefit range)
- **Medium complexity** (0.45-0.50): 1-8× NEON (operation-dependent)
- **Filtering** (0.55): 1.1-1.4× NEON (branch-limited)
- **Complex aggregation** (0.61): 7-23× NEON (moderate benefit)

**Novel Discoveries**:
1. Lower bound confirmed: Operations <0.25 complexity see no NEON benefit
2. Pattern validation: AT content (0.35) and GC content (0.315) show nearly identical speedup
3. Category distinction: Filtering behaves differently than counting (branches limit SIMD)
4. Scale effects: NEON speedup decreases with scale (cache effects at small scales)

### What's Ready for Next Session

**NEXT DIMENSION: Parallel/Threading** (Following systematic pilot approach)

**Immediate Next Steps** (Full day):
1. **Create parallel/threading pilot program**:
   - Test thread counts: 1, 2, 4, 8 threads
   - Test across 10 operations (or representative subset across complexity spectrum)
   - Test across 6 scales (100 → 10M sequences)
   - Expected scope: ~240 experiments (10 ops × 4 thread counts × 6 scales)

2. **Measure P-core vs E-core performance**:
   - Use thread affinity to assign work to specific core types
   - Compare performance vs default scheduling
   - Identify which operations benefit from E-core assignment

3. **Document findings**:
   - Which operations benefit from parallelism?
   - What's the optimal thread count per operation category?
   - Does complexity score predict parallelism benefit?
   - Create comprehensive analysis document (like GPU dimension docs)

**Following Dimensions** (in order):
4. AMX Matrix Engine dimension
5. Neural Engine dimension
6. Hardware Compression dimension
7. GCD/QoS dimension
8. **THEN** Level 1/2 automation (after ALL pilots complete)

**DO NOT skip to automation** - Each dimension deserves exhaustive individual testing.

### Repository Status

```
Location: /Users/scotthandley/Code/apple-silicon-bio-bench
GitHub:   https://github.com/shandley/apple-silicon-bio-bench
Branch:   main

Individual Pilot Dimensions Completed:
✅ NEON SIMD:        10 operations × 6 scales = 60 experiments
✅ 2-bit Encoding:   2 operations × 6 scales = 12 experiments
✅ GPU Metal:        4 operations × 8 scales = 32 experiments
⏳ Parallel/Thread:  Next dimension to test
⏳ AMX:              Not started
⏳ Neural Engine:    Not started
⏳ HW Compression:   Not started
⏳ GCD/QoS:          Not started

Total Experiments So Far: ~104 systematic tests
```

**Build Status**: ✅ All crates compile, GPU features functional

**Documentation**:
- `results/n10_final_validation.md` - NEON dimension complete analysis
- `results/phase2_encoding_complete_results.md` - Encoding dimension results
- `results/phase1_gpu_dimension_complete.md` - GPU dimension complete analysis (BREAKTHROUGH)

### Key Findings from GPU Dimension

**NEON Effectiveness as Primary Predictor**:
- GPU benefit requires: NEON < 2× speedup AND complexity > 0.55 AND batch > 10K
- Base counting: NEON 16× → GPU never competitive (1.3× slower at 5M sequences)
- Reverse complement: NEON 1× but CPU fast → GPU 2× slower
- Quality aggregation: NEON 7-12× → GPU 4× slower
- **Complexity score: NEON 1× AND complex (0.61) → GPU 2-3× FASTER** ✅

**GPU Performance Characteristics**:
- Dispatch overhead: ~3-4ms fixed cost per call
- Cliff threshold: 10K sequences for complexity score
- Unified memory: Zero transfer overhead (validated)
- Scales well for compute-intensive operations

**Decision Rules Derived**:
```rust
fn should_use_gpu(op: &Op, batch: usize, neon: f64) -> bool {
    neon < 2.0 && op.complexity() > 0.55 && batch > 10_000
}
```

### Next Session Priority

**Test Parallel/Threading Dimension** (following same systematic approach):

1. Create comprehensive parallel pilot program
2. Test thread counts: 1, 2, 4, 8 threads
3. Test across operation spectrum (all 10 operations)
4. Test across 6 scales (100 → 10M)
5. Document findings thoroughly
6. Expected scope: ~240 experiments

**DO NOT**:
- ❌ Jump to Level 1/2 automation
- ❌ Skip any remaining dimensions
- ❌ Suggest "we have enough data"

**DO**:
- ✅ Test each dimension exhaustively
- ✅ Follow proven systematic approach
- ✅ Document unexpected patterns

---

**Status**: 3 dimensions complete (NEON, Encoding, GPU), 5 remaining
**Next**: Parallel/Threading dimension pilot (exhaustive testing)

**Last Updated**: October 31, 2025 (Evening)
