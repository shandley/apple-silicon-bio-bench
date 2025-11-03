# ASBB + biofast: 2-3 Week Roadmap

**Goal**: Complete DAG traversal + Build production library + Draft manuscript

**Timeline**: November 4-22, 2025 (15-18 working days)

---

## Overview

**Week 1** (Nov 4-8): Complete DAG Traversal
- Fill experimental gaps (740 experiments)
- Document DAG framework
- Derive per-operation optimization rules

**Week 2** (Nov 11-14): Build `biofast` Library
- Implement streaming architecture
- Add auto-optimization logic
- Production features (CLI, docs, error handling)

**Week 3** (Nov 18-22): Validation + Paper Draft
- Validate library performance
- Draft manuscript sections
- Prepare for submission

---

## Week 1: Complete DAG Traversal (Nov 4-8)

### Day 1 (Mon, Nov 4): Build DAG Testing Harness

**Goal**: Unified test framework for systematic hardware exploration

**Morning** (4 hours):
- [ ] Design `asbb-dag-traversal` architecture
  - Config-driven operation specification
  - Automatic DAG traversal with pruning
  - CSV output for all experiments
- [ ] Implement core abstractions:
  - `DAGNode`: Represents optimization config
  - `DAGTraversal`: Executes systematic exploration
  - `PruningStrategy`: Threshold-based pruning

**Afternoon** (4 hours):
- [ ] Implement test execution:
  - Parallel test runner (use all cores)
  - Progress tracking (740 experiments)
  - Error handling and recovery
- [ ] Validation tests:
  - Test on 2 operations (base_counting, gc_content)
  - Verify pruning logic works
  - Confirm CSV output format

**Evening**:
- [ ] Lab notebook entry: "Building DAG Testing Harness"
- [ ] Commit harness code

**Deliverables**:
- `crates/asbb-cli/src/dag_traversal.rs` (500+ lines)
- Unit tests passing
- Ready to run 740 experiments

---

### Day 2-3 (Tue-Wed, Nov 5-6): Run DAG Experiments

**Goal**: Execute 740 experiments to fill gaps

**Day 2 Morning** (4 hours):
- [ ] **Batch 1**: NEON+Parallel composition (240 experiments)
  - 20 operations × 4 configs (naive, NEON, NEON+2t, NEON+4t) × 3 scales
  - Runtime: ~2-3 hours (automated)
  - Monitor for errors

**Day 2 Afternoon** (4 hours):
- [ ] Analyze Batch 1 results:
  - Validate NEON+Parallel multiplicative for all ops
  - Identify exceptions (if any)
  - Update pruning thresholds if needed
- [ ] **Batch 2 Start**: Core affinity × NEON (90/180 experiments)
  - 10 operations × 2 SIMD × 3 cores × 1.5 scales
  - Runtime: ~1-2 hours

**Day 3 Morning** (4 hours):
- [ ] **Batch 2 Complete**: Core affinity × NEON (90/180 experiments)
  - Finish remaining scales
- [ ] Analyze Batch 2 results:
  - Do E-cores remain competitive with NEON?
  - Per-operation core affinity recommendations

**Day 3 Afternoon** (4 hours):
- [ ] **Batch 3**: Precise scale thresholds (320 experiments)
  - 10 operations × 4 configs × 8 scales
  - Runtime: ~3-4 hours (longest batch)
  - Monitor progress carefully

**Evening**:
- [ ] Backup all raw CSVs
- [ ] Initial data validation (check for anomalies)

**Deliverables**:
- `results/dag_complete/dag_neon_parallel.csv` (240 rows)
- `results/dag_complete/dag_core_affinity.csv` (180 rows)
- `results/dag_complete/dag_scale_thresholds.csv` (320 rows)

---

### Day 4 (Thu, Nov 7): Analyze DAG Results

**Goal**: Derive per-operation optimization rules

**Morning** (4 hours):
- [ ] Build analysis scripts:
  - `analysis/analyze_dag_complete.py`
  - Calculate optimal configs per operation
  - Identify precise scale thresholds
  - Generate optimization rules tables

- [ ] Run analysis:
  ```bash
  python analysis/analyze_dag_complete.py \
    --neon-parallel results/dag_complete/dag_neon_parallel.csv \
    --affinity results/dag_complete/dag_core_affinity.csv \
    --thresholds results/dag_complete/dag_scale_thresholds.csv \
    --output results/dag_complete/OPTIMIZATION_RULES_COMPLETE.md
  ```

**Afternoon** (4 hours):
- [ ] Generate per-operation rules:
  - base_counting: NEON+4t+P @ 8,342 seqs
  - gc_content: NEON+4t+P @ 7,891 seqs
  - quality_aggregation: NEON+8t+P @ 52,101 seqs
  - etc. for all 20 operations

- [ ] Create lookup tables for `biofast`:
  - `OPERATION_THRESHOLDS.toml` (machine-readable)
  - Will be embedded in library

**Evening**:
- [ ] Lab notebook Entry 022: "Complete Hardware Optimization DAG"
  - Document all 740 experiments
  - Include optimization rules tables
  - Reference FINDINGS.md

**Deliverables**:
- `results/dag_complete/OPTIMIZATION_RULES_COMPLETE.md`
- `OPERATION_THRESHOLDS.toml` (for biofast)
- Lab notebook Entry 022

---

### Day 5 (Fri, Nov 8): Document DAG Framework

**Goal**: Finalize DAG methodology documentation

**Morning** (4 hours):
- [ ] Review and enhance DAG_FRAMEWORK.md:
  - Add empirical validation results
  - Include pruning effectiveness data
  - Add diagrams (if time permits)

- [ ] Create validation section:
  - Test 5 operations with/without pruning
  - Confirm no missed optimizations in pruned branches
  - Document validation results

**Afternoon** (4 hours):
- [ ] Write "How to use DAG framework" guide:
  - Step-by-step process
  - Example: Testing Neural Engine
  - Example: Testing new platform

- [ ] Create framework package:
  - `crates/asbb-framework/` (library for others to use)
  - Core DAG abstractions
  - Traversal algorithms
  - Example config files

**Evening**:
- [ ] Update CURRENT_STATUS.md:
  - Week 1 complete ✅
  - 740 experiments done
  - DAG framework documented

**Deliverables**:
- Enhanced DAG_FRAMEWORK.md
- `crates/asbb-framework/` package
- Lab notebook Entry 022 finalized

---

## Week 2: Build `biofast` Library (Nov 11-14)

### Day 6 (Mon, Nov 11): Streaming Architecture

**Goal**: Implement core streaming abstraction

**Morning** (4 hours):
- [ ] Create `crates/biofast/` package:
  ```bash
  cargo new --lib crates/biofast
  ```

- [ ] Implement core traits:
  - `StreamingOperation` trait (from BIOFAST_VISION.md)
  - `FastqStream` abstraction
  - `FastqRecord` type

- [ ] Add dependencies:
  ```toml
  # Cargo.toml
  flate2 = "1.0"  # gzip
  zstd = "0.13"    # zstd compression
  indicatif = "0.17"  # progress bars
  num_cpus = "1.16"   # CPU detection
  rayon = "1.8"      # parallelism
  ```

**Afternoon** (4 hours):
- [ ] Implement `FastqStream`:
  - Auto-detect compression (gzip, zstd, uncompressed)
  - Buffered reading
  - Progress bar support
  - Error handling

- [ ] Unit tests:
  - Test with compressed files
  - Test error cases
  - Test progress tracking

**Evening**:
- [ ] Integration test:
  - Read 1M sequence file
  - Verify memory stays constant
  - Measure overhead vs load-all

**Deliverables**:
- `crates/biofast/src/lib.rs` (core abstractions)
- `crates/biofast/src/stream.rs` (streaming logic)
- Unit tests passing

---

### Day 7 (Tue, Nov 12): Implement 10 Operations

**Goal**: Add StreamingOperation implementations

**Morning** (4 hours):
- [ ] Implement simple operations (4):
  - `gc_content.rs`
  - `base_counting.rs`
  - `sequence_length.rs`
  - `n_content.rs`

- [ ] Each implementation includes:
  - Naive version
  - NEON version
  - Auto-config logic (thresholds from OPERATION_THRESHOLDS.toml)
  - Unit tests

**Afternoon** (4 hours):
- [ ] Implement filtering operations (2):
  - `quality_filter.rs`
  - `length_filter.rs`

- [ ] Implement aggregation operations (2):
  - `quality_aggregation.rs`
  - `quality_statistics.rs`

**Evening**:
- [ ] Implement complex operations (2):
  - `reverse_complement.rs`
  - `adapter_trimming.rs`

**Deliverables**:
- 10 operation files in `crates/biofast/src/ops/`
- All with naive + NEON + auto-config
- Unit tests for each

---

### Day 8 (Wed, Nov 13): Auto-Optimization + CLI

**Goal**: Add auto-selection logic and command-line tools

**Morning** (4 hours):
- [ ] Implement auto-optimization:
  - Load OPERATION_THRESHOLDS.toml at compile time
  - `AutoConfig` trait implementation
  - Per-operation optimal config selection
  - Runtime hardware detection

- [ ] Example:
  ```rust
  impl GcContentOp {
      pub fn auto_config(dataset_size: usize) -> GcConfig {
          let threshold = THRESHOLDS["gc_content"];
          match dataset_size {
              n if n < threshold.naive_to_neon => GcConfig::Naive,
              n if n < threshold.neon_to_parallel => GcConfig::Neon,
              _ => GcConfig::NeonParallel(auto_thread_count())
          }
      }
  }
  ```

**Afternoon** (4 hours):
- [ ] Build CLI binary:
  - `crates/biofast-cli/` package
  - Subcommands for each operation:
    - `biofast gc-content`
    - `biofast filter --min-quality 20`
    - `biofast trim-adapters`

- [ ] CLI features:
  - Progress bars (--progress flag)
  - Statistics output (--stats flag)
  - Output redirection (-o flag)
  - Batch processing (--batch flag)

**Evening**:
- [ ] Documentation:
  - README.md for biofast package
  - Examples for each operation
  - API documentation (rustdoc)

**Deliverables**:
- Auto-optimization logic complete
- `biofast` CLI tool
- Documentation and examples

---

### Day 9 (Thu, Nov 14): Production Features + Polish

**Goal**: Error handling, edge cases, user experience

**Morning** (4 hours):
- [ ] Comprehensive error handling:
  - Invalid file formats
  - Corrupted compression
  - Out of memory scenarios
  - Interrupted operations (Ctrl+C)

- [ ] User-friendly error messages:
  - Not: "thread 'main' panicked at..."
  - But: "Error: Unable to read FASTQ file. Is it compressed correctly?"

**Afternoon** (4 hours):
- [ ] Edge case testing:
  - Empty files
  - Single-sequence files
  - Malformed FASTQ
  - Mixed quality encodings (Phred33/64)

- [ ] Performance testing:
  - Verify no memory leaks (run on 10M sequences)
  - Verify progress bars accurate
  - Verify auto-selection working

**Evening**:
- [ ] Polish and final testing:
  - Run full test suite
  - Fix any failing tests
  - Code formatting (rustfmt)
  - Clippy lints

- [ ] Prepare for validation week:
  - Test datasets ready
  - Validation protocol written

**Deliverables**:
- Production-ready `biofast` library
- CLI tools tested and working
- Ready for Entry 023 (validation)

---

## Week 3: Validation + Paper Draft (Nov 18-22)

### Day 10 (Mon, Nov 18): Streaming Validation (Entry 023)

**Goal**: Validate Data Access pillar experimentally

**Morning** (4 hours):
- [ ] Memory usage experiments (20 experiments):
  - Test `biofast` vs load-all
  - Scales: 1K, 10K, 100K, 1M, 10M sequences
  - Operations: gc_content, quality_filter, base_counting, reverse_complement
  - Measure RSS (resident set size)

- [ ] Target validation:
  - `biofast` memory: <100 MB constant ✅
  - Load-all memory: 360-716 MB per 1M sequences ❌

**Afternoon** (4 hours):
- [ ] Performance overhead experiments (10 experiments):
  - Streaming vs load-all throughput
  - Expected: <10% overhead
  - If overhead >10%: Profile and optimize

- [ ] Auto-selection validation (20 experiments):
  - Test at threshold boundaries
  - Example: gc_content at 7,500, 8,000, 8,500 seqs (threshold ~8K)
  - Verify correct config selected

**Evening**:
- [ ] Lab notebook Entry 023: "Streaming Architecture Validation"
  - Document all experiments
  - Include memory usage plots
  - Performance overhead analysis
  - Auto-selection accuracy

**Deliverables**:
- `results/biofast_validation/streaming_validation.csv`
- Lab notebook Entry 023
- Proof that Data Access pillar is validated ✅

---

### Day 11 (Tue, Nov 19): Cross-Platform Validation (Entry 024)

**Goal**: Verify library performance matches experimental predictions

**Morning** (4 hours):
- [ ] Mac M4 validation (10 experiments):
  - Test all 10 operations with `biofast`
  - Compare to original ASBB experiments
  - Expected: Within 5% of predicted speedups

- [ ] AWS Graviton validation (10 experiments):
  - Run `biofast` on Graviton 3 instance
  - Compare to Entry 021 (Graviton experiments)
  - Verify portability maintained

**Afternoon** (4 hours):
- [ ] Large-scale testing:
  - 1M sequence datasets (real FASTQ, not synthetic)
  - 10M sequence datasets (if time permits)
  - Test with actual SRA data (compressed)

- [ ] Error handling validation:
  - Test with malformed files
  - Test with interrupted operations (kill during run)
  - Verify graceful failures

**Evening**:
- [ ] Lab notebook Entry 024: "biofast Performance Validation"
  - Cross-platform results
  - Large-scale validation
  - Error handling tests

**Deliverables**:
- `results/biofast_validation/cross_platform.csv`
- Lab notebook Entry 024
- Confidence that library works as designed ✅

---

### Day 12 (Wed, Nov 20): Draft Methods + Results

**Goal**: Write manuscript sections (Methods, Results)

**Morning** (4 hours):
- [ ] **Methods section**:
  - DAG-based testing framework
  - Algorithm description
  - Pruning strategy
  - Hardware tested (Mac M4, Graviton 3)

- [ ] Draft 2,000-2,500 words

**Afternoon** (4 hours):
- [ ] **Results section**:
  - Four pillars validated (1,640 experiments)
  - Economic: 40-80× speedup
  - Environmental: 1.95-3.27× energy efficiency
  - Portability: Perfect Mac→Graviton transfer
  - Data Access: Streaming <100 MB vs 12 TB load-all

- [ ] Draft 2,500-3,000 words
- [ ] Create results tables (4-5 tables)

**Evening**:
- [ ] Review and revise Methods + Results
- [ ] Share draft sections (if collaboration)

**Deliverables**:
- Methods section draft (2,000-2,500 words)
- Results section draft (2,500-3,000 words)
- 4-5 results tables

---

### Day 13 (Thu, Nov 21): Draft Implementation + Discussion

**Goal**: Complete manuscript sections

**Morning** (4 hours):
- [ ] **Implementation section**:
  - `biofast` library design
  - Streaming architecture
  - Auto-optimization
  - Cross-platform support

- [ ] Draft 1,500-2,000 words
- [ ] Code examples (3-4 examples)

**Afternoon** (4 hours):
- [ ] **Discussion section**:
  - Impact on underserved researchers
  - Before: 5TB requires $50K server
  - After: 5TB on $1.4K laptop with `biofast`
  - Democratization achieved

- [ ] Draft 1,500-2,000 words
- [ ] Future work (1-2 paragraphs)

**Evening**:
- [ ] Create figures (5-7 figures):
  - Figure 1: DAG framework diagram
  - Figure 2: Four pillars validation (bar chart)
  - Figure 3: Energy efficiency (scatter plot)
  - Figure 4: Cross-platform portability (heatmap)
  - Figure 5: Memory usage (line plot: load-all vs streaming)
  - Figure 6: Auto-optimization example
  - Figure 7: Speedup vs complexity

**Deliverables**:
- Implementation section draft (1,500-2,000 words)
- Discussion section draft (1,500-2,000 words)
- 5-7 publication-quality figures

---

### Day 14 (Fri, Nov 22): Polish + Submit

**Goal**: Finalize manuscript, prepare submission

**Morning** (4 hours):
- [ ] **Introduction section**:
  - Four barriers to genomics access
  - Current state (ad-hoc testing, HPC gatekeepers)
  - Our contribution (DAG framework + biofast)

- [ ] Draft 1,500-2,000 words

- [ ] **Abstract**:
  - 250-300 words
  - Hit all key points (methodology, results, tool)

**Afternoon** (4 hours):
- [ ] Assemble complete manuscript:
  - Abstract
  - Introduction
  - Methods
  - Results
  - Implementation
  - Discussion
  - References

- [ ] Total: ~10,000-12,000 words

- [ ] Format for target journal:
  - GigaScience or BMC Bioinformatics
  - LaTeX template
  - Citation style

**Evening**:
- [ ] Final review:
  - Read through completely
  - Check figure references
  - Verify all citations
  - Proofread

- [ ] Prepare supplementary materials:
  - All raw data CSVs
  - Lab notebook entries
  - Code repository link
  - Reproducibility guide

**Deliverables**:
- Complete manuscript (10,000-12,000 words)
- 5-7 figures
- Supplementary materials
- Ready for submission ✅

---

## Post-Submission Tasks (Week 4+)

### Week 4 (Nov 25-29): Publication Preparation

- [ ] Publish `biofast` to crates.io (1.0.0 release)
- [ ] Create GitHub repository (public)
- [ ] Write comprehensive documentation site
- [ ] Create tutorial videos (optional)

### Week 5+: Community Engagement

- [ ] Respond to paper reviews
- [ ] Address biofast issues/PRs
- [ ] Plan additional platforms (RPi 5, Ampere Altra)
- [ ] Consider follow-up papers (streaming architecture deep-dive)

---

## Success Metrics

### Scientific Excellence ✅

- [ ] 1,640 experiments executed and analyzed
- [ ] Novel methodology (DAG framework) documented
- [ ] All 4 pillars validated experimentally
- [ ] Manuscript submitted to Q1 journal

### Practical Impact ✅

- [ ] `biofast` published to crates.io
- [ ] Researchers can install and use immediately
- [ ] Documentation comprehensive
- [ ] CLI tools work out-of-box

### Democratization Mission ✅

- [ ] Economic barrier removed (validated)
- [ ] Environmental barrier removed (validated)
- [ ] Portability barrier removed (validated)
- [ ] Data Access barrier removed (validated)

---

## Risk Mitigation

### Risk 1: DAG Experiments Take Longer

**Mitigation**:
- Build harness carefully (Day 1)
- Test on 2 operations before full run
- Have automated error recovery
- If >4 days: Reduce scale variations (8 → 6 scales)

### Risk 2: `biofast` Implementation Blocked

**Mitigation**:
- Start simple (streaming architecture first)
- Add operations incrementally
- MVP: 5 operations is sufficient
- Remaining 5 operations can be post-publication

### Risk 3: Paper Draft Too Long

**Mitigation**:
- Follow word limits strictly
- Move details to supplementary materials
- Focus on novel contributions (DAG + tool)
- Prioritize clarity over completeness

### Risk 4: Validation Fails

**Mitigation**:
- Streaming overhead >10%: Profile and optimize
- Auto-selection wrong: Adjust thresholds
- Memory >100 MB: Investigate leaks
- Have buffer time (Days 10-11 for fixes)

---

## Daily Schedule Template

**Morning** (9am-1pm): 4 hours focused work
- Deep work on primary task
- Minimize context switching

**Afternoon** (2pm-6pm): 4 hours execution
- Implementation, testing, analysis
- More interrupts acceptable

**Evening** (7pm-9pm): 2 hours optional
- Documentation, lab notebook
- Review, planning, cleanup

**Total**: 8-10 hours per day (sustainable)

---

## Checkpoints

### End of Week 1 (Nov 8)

**Required**:
- ✅ 740 experiments complete
- ✅ DAG framework documented
- ✅ Per-operation rules derived
- ✅ Lab notebook Entry 022

**Go/No-Go**: Can proceed to Week 2 if all complete

### End of Week 2 (Nov 14)

**Required**:
- ✅ `biofast` library functional
- ✅ 10 operations implemented
- ✅ CLI tools working
- ✅ Ready for validation

**Go/No-Go**: Can proceed to Week 3 if all complete

### End of Week 3 (Nov 22)

**Required**:
- ✅ All 4 pillars validated
- ✅ Manuscript draft complete
- ✅ Figures ready
- ✅ Ready for submission

**Success**: Paper submitted, library published

---

**Last Updated**: November 3, 2025
**Owner**: Scott Handley + Claude
**Timeline**: 15-18 working days (flexible)
**End Goal**: Comprehensive paper + production tool

**For questions**: See CURRENT_STATUS.md, BIOFAST_VISION.md, DAG_FRAMEWORK.md
