# Phase 1: Neural Engine Dimension Pilot - Protocol

**Date**: November 2, 2025
**Status**: Design phase
**Pilot**: 6/9 dimension pilots
**Category**: Apple Silicon-specific hardware dimension

---

## Objective

Systematically test Apple's Neural Engine performance for sequence operations framed as machine learning problems to determine if dedicated ML acceleration provides speedup over CPU baselines.

**Research Questions**:
1. Can sequence operations be effectively framed as ML problems?
2. Does Neural Engine (16-core, 38 TOPS on M4) provide speedup vs CPU?
3. What is the overhead of data conversion to ML model inputs?
4. What is the minimum data scale for Neural Engine benefit?
5. Which operation types benefit most from ML-based approaches?

**Hypothesis**: Operations with learned patterns (quality filtering, complexity assessment, adapter detection) may benefit from Neural Engine acceleration if conversion overhead is manageable.

---

## Background: Apple Neural Engine

### Hardware Specifications

**Neural Engine Evolution**:
- **M1**: 16-core, 11 TOPS
- **M2**: 16-core, 15.8 TOPS
- **M3**: 16-core, 18 TOPS
- **M4**: 16-core, 38 TOPS (2.1× faster than M3)
- **M5**: 16-core, ~50 TOPS (estimated)

**Current Test Platform**: M4 MacBook Pro (38 TOPS)

### Core ML Framework

**Integration**:
- Core ML models compiled for Neural Engine dispatch
- Automatic fallback to CPU/GPU if Neural Engine busy
- MLMultiArray for tensor input/output
- Model conversion overhead critical

**Supported Model Types**:
- Neural networks (dense, convolutional, recurrent)
- Tree ensembles (random forests, gradient boosted trees)
- Support vector machines
- Generalized linear models

---

## Challenge Analysis

### Why Neural Engine is Different from Previous Pilots

**NEON/AMX/Parallel**: Direct algorithm acceleration
- Same algorithm, different execution path
- Low/zero conversion overhead
- Deterministic output (exact match)

**Neural Engine**: Learned approximation
- Different algorithmic approach (ML inference)
- High conversion overhead (sequence → tensor)
- Approximate output (may not match exact algorithm)

### Key Challenges

1. **Model Creation**: Need to train/obtain Core ML models for each operation
2. **Conversion Overhead**: Sequence data → MLMultiArray tensor
3. **Accuracy Trade-offs**: ML approximation vs exact algorithms
4. **Model Size**: Large models may not fit in Neural Engine cache
5. **Batch Processing**: Neural Engine benefits from batching

---

## Experimental Design

### Approach: Pragmatic ML-Amenable Operations

Rather than forcing all 20 operations into ML formulations, focus on operations where ML approaches are **naturally applicable**:

#### Tier 1: Naturally ML-Amenable (Primary Focus)

**1. quality_filter** (Binary Classification)
- **Task**: Predict if sequence passes quality thresholds
- **Input**: Quality score vector (150 values → fixed-size tensor)
- **Output**: Binary (pass/fail)
- **Model**: Simple MLP or tree ensemble
- **Ground Truth**: Existing quality_filter implementation

**2. complexity_score** (Regression)
- **Task**: Predict linguistic complexity score
- **Input**: K-mer frequency vector or sequence encoding
- **Output**: Continuous score (0.0-1.0)
- **Model**: Regression network or gradient boosted trees
- **Ground Truth**: Existing complexity_score implementation

**3. adapter_trimming** (Sequence Detection/Classification)
- **Task**: Detect adapter sequences in reads
- **Input**: Sliding window of sequence k-mers
- **Output**: Adapter position or binary (present/absent)
- **Model**: CNN or RNN for sequence pattern matching
- **Ground Truth**: Existing adapter_trimming implementation

#### Tier 2: Experimentally Interesting (Secondary)

**4. sequence_masking** (Low-Complexity Detection)
- **Task**: Identify low-complexity regions
- **Input**: Sliding window k-mer features
- **Output**: Per-base masking mask
- **Model**: Sequence labeling (RNN/CNN)

**5. quality_statistics** (Prediction/Classification)
- **Task**: Predict quality statistics without full scan
- **Input**: Sample of quality scores
- **Output**: Mean/std estimates
- **Model**: Regression

#### Out of Scope (Not ML-Amenable)

- **Exact algorithms**: edit_distance, hamming_distance, reverse_complement
- **Simple counting**: base_counting, gc_content, at_content, n_content
- **Deterministic**: kmer_extraction, kmer_counting, fastq_parsing
- **Threshold-based**: length_filter

---

## Pilot Scope: Minimal Viable Test

### Conservative Start (Recommended)

**Operations**: 3 (quality_filter, complexity_score, adapter_trimming)
**Backends**: 2 (Naive CPU, Neural Engine)
**Scales**: 6 (100, 1K, 10K, 100K, 1M, 10M sequences)
**Total**: 3 operations × 2 backends × 6 scales = **36 experiments**

**Rationale**:
- Minimal scope to validate Neural Engine viability
- Covers classification, regression, and sequence detection
- Low implementation burden (3 models vs 20)
- Fast iteration (36 experiments vs 240)

### Model Simplification Strategy

**Option 1: Pre-trained Stub Models** (Fastest)
- Create simple Core ML models with random weights
- Test infrastructure and overhead, not accuracy
- Validates conversion and Neural Engine dispatch
- **Goal**: Pure performance testing, accuracy secondary

**Option 2: Lightweight Trained Models** (Moderate)
- Train small MLPs on synthetic data
- Reasonable accuracy, low training time
- Tests end-to-end ML pipeline
- **Goal**: Realistic performance + accuracy trade-offs

**Option 3: Production-Quality Models** (Full, Deferred)
- Train on real datasets, optimize hyperparameters
- High accuracy, publication-quality
- **Deferred**: Only if Option 1/2 shows promise

**Recommended**: Start with **Option 1** (stub models) to validate infrastructure.

---

## Implementation Plan

### Phase 1: Infrastructure (2-3 days)

1. **Core ML Integration**:
   - Add `coreml` Rust crate (or FFI to Objective-C/Swift)
   - Create ML model wrapper (load .mlmodel, run inference)
   - Implement sequence → MLMultiArray conversion

2. **Stub Model Creation** (Python/Core ML Tools):
   ```python
   import coremltools as ct

   # quality_filter: 150 inputs → 1 binary output
   model = create_simple_classifier(input_size=150, output_classes=2)
   model.save("quality_filter.mlmodel")

   # complexity_score: 256 inputs → 1 continuous output
   model = create_simple_regressor(input_size=256)
   model.save("complexity_score.mlmodel")

   # adapter_trimming: 600 inputs → 1 binary output
   model = create_simple_classifier(input_size=600, output_classes=2)
   model.save("adapter_trimming.mlmodel")
   ```

3. **Neural Engine Backend** (Rust):
   ```rust
   // crates/asbb-ops/src/quality_filter.rs
   fn execute_neural(&self, data: &[SequenceRecord]) -> Result<OperationOutput> {
       let model = CoreMLModel::load("models/quality_filter.mlmodel")?;

       let mut results = Vec::new();
       for record in data {
           // Convert quality scores to MLMultiArray
           let input = quality_to_ml_array(&record.quality)?;

           // Neural Engine inference
           let output = model.predict(input)?;

           // Parse prediction
           let passes = output.class_label() == "pass";
           if passes {
               results.push(record.clone());
           }
       }

       Ok(OperationOutput::Records(results))
   }
   ```

### Phase 2: Experiment Harness (1 day)

Create `asbb-pilot-neural` binary:
- 3 operations with Neural Engine backends
- 2 backends (Naive, Neural)
- 6 scales
- CSV output with conversion overhead measurement

### Phase 3: Execution & Analysis (1 day)

Run experiments and analyze:
- Neural Engine speedup vs CPU
- Conversion overhead (sequence → tensor)
- Scale threshold for benefit
- Accuracy trade-offs (if using Option 2/3 models)

---

## Expected Outcomes

### Scenario 1: Neural Engine Overhead Dominates (Likely)

**Result**: Neural Engine 0.5-0.8× slower than CPU
**Cause**: Conversion overhead (sequence → tensor) exceeds inference speedup
**Conclusion**: Neural Engine not beneficial for sequence operations
**Impact**: Skip Neural Engine for remaining operations (critical negative finding)

### Scenario 2: Neural Engine Competitive at Scale (Possible)

**Result**: Neural Engine 1.0-1.5× faster at 1M+ sequences
**Cause**: Batch inference amortizes conversion overhead
**Conclusion**: Neural Engine viable for large-scale batch operations
**Impact**: Explore Neural Engine for high-throughput pipelines

### Scenario 3: Neural Engine Wins (Unlikely but Valuable)

**Result**: Neural Engine 2-5× faster across all scales
**Cause**: Dedicated ML hardware + good conversion efficiency
**Conclusion**: Frame more operations as ML problems
**Impact**: Major architectural shift toward ML-based sequence analysis

---

## Success Criteria

**Minimum Viable Pilot**:
- ✅ 36 experiments complete (3 ops × 2 backends × 6 scales)
- ✅ Neural Engine dispatch confirmed (not falling back to CPU)
- ✅ Conversion overhead measured and documented
- ✅ Speedup characterized across scales
- ✅ Decision made: Continue vs skip Neural Engine dimension

**Optional (if promising)**:
- Train lightweight models (Option 2)
- Measure accuracy trade-offs
- Test 2-3 additional operations
- Compare Neural Engine vs M5 GPU Neural Accelerators (if M5 available)

---

## Technical Risks & Mitigations

### Risk 1: Core ML Integration Complexity

**Problem**: Rust ↔ Core ML FFI may be complex
**Mitigation**: Use Swift/Objective-C wrapper, FFI via C bindings
**Fallback**: Implement pilot in Swift, port to Rust if successful

### Risk 2: Model Availability

**Problem**: No pre-trained models for sequence operations
**Mitigation**: Use stub models (Option 1) for infrastructure testing
**Fallback**: Train simple models on synthetic data (Option 2)

### Risk 3: Neural Engine Dispatch Unclear

**Problem**: Core ML may use CPU/GPU instead of Neural Engine
**Mitigation**: Monitor with Instruments (Core ML profiling)
**Verification**: Check GPU/CPU utilization (should be low)

### Risk 4: Conversion Overhead Unknown

**Problem**: Sequence → tensor conversion cost unpredictable
**Mitigation**: Measure separately from inference
**Reporting**: Break down: conversion time vs inference time

---

## Timeline

**Optimistic** (Option 1: Stub models):
- Day 1: Core ML integration, stub models
- Day 2: Neural Engine backends, experiment harness
- Day 3: Run experiments, analyze results
- **Total**: 3 days

**Realistic** (Option 1 + infrastructure challenges):
- Week 1: Core ML integration, debugging FFI
- Week 2: Implement backends, create harness
- Week 3: Run experiments, document findings
- **Total**: 2-3 weeks

**Conservative** (Option 2: Lightweight trained models):
- Week 1-2: Core ML integration
- Week 3-4: Model training on synthetic data
- Week 5: Experiments and analysis
- **Total**: 4-5 weeks

---

## Deliverables

1. **Protocol** (this document): Experimental design and rationale
2. **Models**: 3 Core ML models (.mlmodel files)
3. **Implementation**: Neural Engine backends for 3 operations
4. **Harness**: `asbb-pilot-neural` experiment binary
5. **Results**: CSV with 36 experiments
6. **Analysis**: Summary document with speedup characterization
7. **Lab Notebook**: Entry documenting Neural Engine pilot
8. **Checkpoint Update**: `PILOT_CHECKPOINT.md` (6/9 complete)

---

## Decision Point: Proceed or Defer?

### Arguments for Proceeding Now

**Pro**:
- Natural next pilot in sequence (6/9)
- Neural Engine is key Apple Silicon differentiator
- Negative finding would be valuable (like AMX)
- Core ML integration useful for future work

**Con**:
- Higher complexity than previous pilots (model creation, FFI)
- Likely negative result (conversion overhead)
- May take 2-3 weeks vs 1-2 days for other pilots
- Only 3 operations naturally ML-amenable

### Arguments for Deferring

**Alternative**: Skip to **Hardware Compression** pilot (7/9)
- **Simpler**: AppleArchive framework (native Rust bindings)
- **Faster**: 2-3 days implementation + execution
- **Applicable**: All I/O operations benefit
- **Likely positive**: Hardware compression well-proven

**Then**: Return to Neural Engine after easier pilots complete

### Recommendation

**Option A** (Aggressive): Proceed with Neural Engine (Option 1: Stub models)
- 3-day sprint to validate infrastructure
- Accept likely negative finding
- Maintain sequential pilot approach

**Option B** (Pragmatic): Defer Neural Engine, do Hardware Compression next
- Faster iteration (7/9 → 8/9 → 9/9 → then Neural Engine)
- Build momentum with likely-positive pilots
- Neural Engine as "advanced" pilot after basics complete

**Vote**: Defer to user preference

---

## References

- Apple Core ML Documentation: https://developer.apple.com/documentation/coreml
- Neural Engine Architecture: https://github.com/hollance/neural-engine
- Core ML Tools (Python): https://coremltools.readme.io/
- Apple Machine Learning: https://machinelearning.apple.com/

---

**Created**: November 2, 2025
**Next Review**: After user decision on proceed vs defer
**Status**: Awaiting approval to proceed with implementation
