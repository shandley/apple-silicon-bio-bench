# ASBB Project Roadmap

**Last Updated**: November 1, 2025
**Status**: Phase 1 Complete, Moving to Level 1/2 Automation

---

## Overview

**Apple Silicon Bio Bench (ASBB)** is a systematic framework for mapping bioinformatics performance across Apple Silicon hardware dimensions, enabling automatic optimization of sequence analysis tools.

**Paradigm**: Science (systematic exploration) over Engineering (ad-hoc optimization)

---

## Project Phases

### ✅ Phase 1: Dimensional Testing (COMPLETE - Nov 1, 2025)

**Goal**: Systematically characterize performance across hardware dimensions with current 10-operation set

**Approach**: Exhaustive pilot testing (N operations × M configs × K scales)

**Dimensions Tested** (4/4 testable):
1. ✅ **NEON SIMD** (60 experiments) - Complexity-speedup relationship (R² = 0.536)
2. ✅ **GPU Metal** (32 experiments) - First GPU win (complexity_score 2.74×)
3. ✅ **2-bit Encoding** (72 experiments) - Conversion overhead quantified
4. ✅ **Parallel/Threading** (600 experiments) - E-cores competitive (7.5% faster!)

**Dimensions Deferred** (4, all valid rationale):
- ⏸️ AMX Matrix Engine (requires matrix operations)
- ⏸️ Neural Engine (requires ML operations)
- ⏸️ Hardware Compression (requires streaming architecture)
- ✅ GCD/QoS (complete via Parallel super-linear speedup evidence)

**Total Experiments**: 824 (165% of target)

**Key Deliverables**:
- Lab notebook: 12 entries (chronological documentation)
- Optimization rules: Empirically-derived decision trees
- Infrastructure: ~4,500 lines Rust + Metal, all tests passing
- **Publication-ready findings** with multiple breakthrough discoveries

**Novel Contributions**:
1. First complexity-speedup relationship for NEON on Apple Silicon
2. NEON effectiveness predicts GPU benefit (unified memory paradigm)
3. E-cores competitive for metadata/aggregation operations
4. 2-bit encoding overhead quantified (challenges conventional wisdom)
5. Super-linear parallel speedups documented (150-268% efficiency)

**Status**: ✅ **COMPLETE** - All testable dimensions characterized

**Documentation**:
- Checkpoint: `lab-notebook/2025-11/20251101-012-CHECKPOINT-phase1-complete.md`
- Rules: `results/phase1/phase1_optimization_rules.md`
- Completeness review: `PHASE1_COMPLETENESS_REVIEW.md`

---

### 🚀 Level 1/2: Automated Harness & Cross-Validation (NEXT - 3 weeks)

**Goal**: Validate that Phase 1 rules compose correctly across expanded operation set

**Approach**: Automated testing framework with statistical analysis

#### Step 1: Design Automated Harness (Week 1, 2-3 days)
- Define experiment matrix (20 ops × 25 configs × 6 scales = ~3,000 experiments)
- Implement parallel experiment runner
- Statistical analysis infrastructure (regression, decision trees, cross-validation)

#### Step 2: Expand Operation Set (Week 1-2, 1 week)
- Add 10 operations to reach 20 total
- Representative coverage of all operation categories:
  - Element-wise: Translation, masking
  - Search: K-mer matching, fuzzy matching, motif finding
  - Pairwise: Hamming distance, edit distance
  - Aggregation: Statistics, sketching
  - I/O: Decompression benchmarking

#### Step 3: Run Level 1 Experiments (Week 3, 1-2 days execution)
- Execute ~3,000 automated experiments
- Estimated runtime: 1-2 days (parallelized execution)
- Storage: ~50 GB Parquet files

#### Step 4: Statistical Analysis (Week 3-4, 1 week)
- Cross-validate Phase 1 rules
- Refine NEON speedup model (target R² > 0.6)
- Extract decision trees for GPU/parallel/encoding
- 80/20 train/test split, measure prediction accuracy (target >80%)

#### Step 5: Publication Preparation (Week 4, concurrent)
- Draft methodology paper
- Create figures (complexity-speedup, GPU cliff, E-core performance, parallel speedups)
- Prepare supplementary materials (protocols, datasets, rules JSON)

**Deliverables**:
- `crates/asbb-explorer/src/automated_harness.rs`
- 10 new operation implementations
- `results/level1_primitives_complete.parquet` (full dataset)
- `results/level1_refined_rules.md`
- `crates/asbb-rules/src/lib.rs` (Rust rules implementation)
- Draft methodology paper

**Timeline**: 3 weeks

**Target Metrics**:
- Prediction accuracy: >80% within 20% error
- R² for regression: >0.6
- Decision tree accuracy: >90%

**Status**: ⏳ **READY TO START**

---

### 💡 Phase 3: Creative Hardware Applications (FUTURE - After Level 1/2)

**Goal**: Explore novel "guide vs replace" paradigm for AMX and Neural Engine

**Discovery** (Nov 1, 2025): During completeness review, identified that AMX/Neural Engine could **supplement/guide** traditional bioinformatics (not just replace operations)

#### Traditional Thinking (Phase 1 Assessment):
- "AMX replaces alignment" → 2-10× speedup
- "Neural Engine replaces classification" → 2-10× speedup

#### Creative Thinking (Phase 3 Exploration):
- "AMX **guides** which sequences to align" → Avoid 95% of alignments
- "Neural Engine **predicts** filtration outcomes" → Skip unnecessary work
- **Expected impact**: 50-500× speedups (vs 2-10× for direct replacement)

---

#### Phase 3a: AMX "Guide" Paradigm Testing (2-3 days)

**Experiment**: Batch Hamming Distance for Contamination Screening

**Traditional approach**:
- Align 10K sequences to 100 contamination references
- 1M alignments @ 1-5ms each = 1,000-5,000 seconds

**AMX "guide" approach**:
1. AMX batch Hamming (N×M matrix): 10-50ms → Filter to 5% of sequences
2. Align only candidates (500 sequences): 500-2,500ms
3. **Total**: 0.5-2.5 seconds (vs 1,000-5,000 seconds = 400-10,000× speedup)

**Implementation**:
- AMX matrix operation for N×M Hamming distance
- Compare backends: CPU naive, NEON, AMX
- Measure contamination screening time + accuracy

**Decision criteria**:
- ✅ If AMX >10× faster: "Guide" paradigm validated → Proceed to Phase 3b
- ❌ If AMX <2× faster: AMX not beneficial → Document and conclude

**Deliverables**:
- `crates/asbb-amx/` (if viable)
- `results/phase3/phase3a_amx_guide_paradigm.md`
- Lab notebook entry

**Timeline**: 2-3 days

---

#### Phase 3b: Neural Engine "Predict" Paradigm Testing (1 week)

**Goal**: Validate ML prediction to avoid expensive operations

**Priority Experiments**:

1. **Adapter Presence Prediction**
   - Train: Binary classifier (sequence ends → adapter present/absent)
   - Expected: 10-20× faster prediction, 90%+ accuracy
   - Workflow: Only trim sequences predicted to have adapters (5-20%)

2. **Contamination Screening**
   - Train: Multi-class classifier (k-mer profile → contamination type)
   - Expected: 50-100× faster screening, 85-95% accuracy
   - Workflow: Only align sequences flagged as high-probability contamination

**Implementation**:
- Core ML model training and export
- Neural Engine inference integration
- Comparison to traditional approaches

**Deliverables**:
- `crates/asbb-neural/` (if viable)
- Trained Core ML models
- `results/phase3/phase3b_neural_predict_paradigm.md`
- Lab notebook entry

**Timeline**: 1 week (after Phase 3a, if AMX successful)

---

#### Phase 3c: Combined "Smart Pipeline" (1 week)

**Goal**: Demonstrate hardware cooperation for dramatic speedups

**Combined Workflow**:
```rust
// Smart Contamination Screening
let predictions = neural_engine.predict_contamination(&sequences)?;  // 1-2ms
let candidates = filter_by_probability(&predictions, 0.1);           // 95% eliminated

let distances = amx.batch_hamming_distance(&candidates, &db)?;       // 5-10ms
let close_matches = filter_by_distance(&distances, threshold);       // 99% eliminated

let confirmed = neon.smith_waterman(&close_matches, &references)?;   // 10-20ms

// Total: 15-32ms vs traditional 10-50 seconds = 300-3000× speedup
```

**Implementation**:
- Integrate Neural Engine, AMX, NEON backends
- Build smart pipeline with multi-stage filtering
- Compare to traditional "align everything"

**Deliverables**:
- `crates/asbb-smart/` - Smart pipeline framework
- `results/phase3/phase3c_combined_smart_pipeline.md`
- Lab notebook entry

**Timeline**: 1 week (after Phase 3a and 3b)

---

#### Phase 3 Success Criteria

**Technical**:
- [ ] AMX batch operations >10× faster than CPU
- [ ] Neural Engine prediction >90% accuracy
- [ ] Combined pipeline >50× faster than traditional
- [ ] Workflows demonstrated on real datasets

**Scientific**:
- [ ] "Guide vs Replace" paradigm validated
- [ ] Novel contribution to bioinformatics optimization
- [ ] Publishable findings (separate paper or extended methodology)

**Community**:
- [ ] Reusable smart pipeline framework
- [ ] Pre-trained models for common tasks
- [ ] Integration examples

**Total Timeline**: 2-3 weeks (if all successful), OR single negative result stops exploration

**Status**: ⏸️ **DEFERRED** until after Level 1/2

**Documentation**: `AMX_NEURAL_CREATIVE_EXPLORATION.md` (comprehensive analysis)

---

## Long-Term Vision

### Phase 4: BioMetal Integration (After Level 1/2 or Phase 3)

**Goal**: Integrate ASBB optimization rules into production BioMetal tool

**Approach**:
1. Publish `asbb-rules` crate (v0.1.0)
2. Add dependency to BioMetal
3. Update BioMetal commands to query rules
4. Benchmark BioMetal with automatic optimization

**Expected benefit**: 10-200× speedups for BioMetal commands (automatic, zero per-command tuning)

**Timeline**: 2-3 days integration work

---

### Phase 5: Publication & Community Release

**Methodology Paper**:
- Title: "Systematic Performance Characterization of Sequence Operations on Apple Silicon"
- Venue: PLOS Computational Biology or Bioinformatics
- Content: Phase 1 + Level 1/2 findings
- Timeline: Draft during Level 1/2 Week 4, submit after analysis complete

**Possible Additional Papers**:
1. **Creative Hardware Applications** (if Phase 3 successful)
   - "Guide vs Replace: Novel Paradigms for Hardware Acceleration in Bioinformatics"
2. **Application Paper** (after Phase 4)
   - "Automatic Optimization of BioMetal Using Empirically-Derived Rules"

**Community Release**:
- [ ] `asbb-rules` crate published to crates.io
- [ ] Datasets published to Zenodo/FigShare
- [ ] Documentation website
- [ ] Integration examples
- [ ] Tutorial videos

---

## Current Status Summary

**Completed**:
- ✅ Phase 1: 4 dimensions, 824 experiments, optimization rules derived
- ✅ Creative exploration: "Guide vs Replace" paradigm identified

**In Progress**:
- 🚀 Documentation updates (this roadmap, NEXT_STEPS.md, lab notebook)

**Next Up**:
- 🚀 Level 1/2 automated harness design and implementation (3 weeks)

**Future**:
- 💡 Phase 3: Creative hardware applications (2-3 weeks, optional)
- 🔗 Phase 4: BioMetal integration (2-3 days)
- 📄 Phase 5: Publication and community release (ongoing)

---

## Key Documents

**Planning**:
- `PROJECT_ROADMAP.md` (this file) - High-level overview
- `NEXT_STEPS.md` - Detailed next steps and timelines
- `METHODOLOGY.md` - Experimental design methodology
- `CLAUDE.md` - Development guide and philosophy

**Phase 1 Results**:
- `lab-notebook/INDEX.md` - Chronological record (12 entries)
- `lab-notebook/2025-11/20251101-012-CHECKPOINT-phase1-complete.md` - Phase 1 completion
- `results/phase1/phase1_optimization_rules.md` - Empirical rules
- `PHASE1_COMPLETENESS_REVIEW.md` - Comprehensive completeness audit

**Creative Exploration**:
- `AMX_NEURAL_CREATIVE_EXPLORATION.md` - Phase 3 detailed analysis

---

**Last Updated**: November 1, 2025
**Next Review**: After Level 1/2 completion
