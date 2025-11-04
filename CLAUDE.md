# ASBB: Claude Development Guide

**Last Updated**: November 4, 2025

---

## 🎉 Current Phase: Experimentation COMPLETE → Publication Prep

**Experimentation Phase** (Oct 30 - Nov 4, 2025): ✅ COMPLETE
- DAG hardware validation (307 experiments, statistically rigorous)
- Streaming characterization (72 experiments, 100% complete)
- I/O optimization (parallel bgzip + mmap, 100% complete)
- **Total**: 1,357 experiments (40,710 measurements, N=30)
- **Status**: All 4 democratization pillars validated

**Current Phase** (Nov 4, 2025): Publication Artifacts Complete ✅
- ✅ Artifact 1: OPTIMIZATION_RULES.md (590 lines, evidence-based)
- ✅ Artifact 2: EXPERIMENTAL_SUMMARY.md (450+ lines, publication stats)
- ✅ Artifact 3: Validation Plots (5 plots, PNG + PDF, 300 DPI)
- ✅ Artifact 4: PUBLICATION_SUMMARY.md (550+ lines, quick-reference)

**Parallel Track** (Nov 4 - Dec 15, 2025): biometal Development (separate session)
- biometal repo: https://github.com/shandley/biometal
- Week 1-2: Core infrastructure (v0.1.0 - local file streaming)
- Week 3-4: Network streaming (v0.2.0 - HTTP/SRA)
- Week 5-6: Python + ML integration (v0.3.0 - BERT-ready)
- Week 7+: Production polish (v1.0.0 - crates.io)

**Publication Track** (Nov 4, 2025 → Jan-Feb 2026): Manuscript Drafting
- ✅ All 4 artifacts complete (ready for Week 9-10 manuscript drafting)
- Paper 1: DAG Framework (BMC Bioinformatics, Dec 2025 target)
- Paper 2: biometal Library (Bioinformatics/JOSS, Feb 2026 target)
- Paper 3: Four-Pillar Democratization (GigaScience, Mar 2026 target)

**Key Documents**:
- **CURRENT_STATUS.md**: Current state, evidence summary
- **ROADMAP.md**: Development timeline
- **OPTIMIZATION_RULES.md**: Artifact 1 (590 lines) ✅
- **EXPERIMENTAL_SUMMARY.md**: Artifact 2 (450+ lines) ✅
- **results/publication_plots/**: Artifact 3 (5 plots) ✅
- **PUBLICATION_SUMMARY.md**: Artifact 4 (550+ lines) ✅
- **biometal repo**: https://github.com/shandley/biometal

---

## Publication Artifacts (✅ ALL COMPLETE, November 4, 2025)

All 4 artifacts created and ready for manuscript drafting (Week 9-10, Jan-Feb 2026).

### Artifact 1: OPTIMIZATION_RULES.md ✅
- **Status**: Complete (590 lines)
- **Location**: Both ASBB and biometal repos
- **Contents**: 6 optimization rules with evidence links, implementation patterns
- **Serves**: biometal development + publication Methods sections

### Artifact 2: EXPERIMENTAL_SUMMARY.md ✅
- **Status**: Complete (450+ lines)
- **Contents**: Executive summary, 3 publication-quality tables, 11 key findings with effect sizes, statistical rigor documentation, reproducibility information
- **Serves**: All 3 papers (statistics, tables, findings)

### Artifact 3: Validation Plots ✅
- **Status**: Complete (5 plots, PNG + PDF, 300 DPI)
- **Location**: `results/publication_plots/`
- **Contents**: NEON speedup, streaming memory, I/O stack, block size impact, mmap threshold
- **Serves**: All 3 papers (Results section figures)

### Artifact 4: PUBLICATION_SUMMARY.md ✅
- **Status**: Complete (550+ lines)
- **Contents**: Quick-reference with abstracts, contributions, figure/table specs, sample text, submission checklists for all 3 papers
- **Serves**: Rapid manuscript drafting (one-stop reference)

---

## Mission

**Democratizing Bioinformatics** by breaking down FOUR barriers:

1. 💰 **Economic**: Consumer hardware ($1.4K laptop) replaces $50K+ HPC servers
2. 🌱 **Environmental**: ARM efficiency inherent (validated via portability)
3. 🔄 **Portability**: ARM NEON works across Mac, Graviton, Ampere, RPi
4. 📊 **Data Access**: Network streaming enables 5TB analysis without download

**Target Audiences**:
- LMIC researchers (limited storage/bandwidth)
- Small academic labs (no HPC clusters)
- ML practitioners (BERT preprocessing bottleneck)
- Field researchers (portable, low power)
- Students (accessible hardware)

**Delivery Vehicle**: **biometal** - production Rust library with Python bindings

---

## Current Status (November 4, 2025)

### Evidence Base: ✅ COMPLETE

**🎉 1,357 total experiments** (N=30 repetitions each = 40,710 measurements):

1. **DAG Hardware Validation**: 307 experiments (✅ COMPLETE)
   - Finding: NEON provides 16-25× speedup for element-wise operations
   - Finding: GPU/AMX/2-bit don't help (negative findings documented)
   - Publication: DAG framework paper (BMC Bioinformatics, in prep)

2. **Streaming Characterization**: 72 experiments (✅ COMPLETE, 100%)
   - Benchmark 1 v2: Memory footprint ✅ (99.5% reduction, constant ~5 MB)
   - Benchmark 2: Streaming overhead ✅ (82-86% with record-by-record)
   - Benchmark 3: E2E pipeline ✅ (I/O dominates 264-352×)
   - **Key insight**: Block-based (10K chunks) required, not record-by-record

3. **I/O Optimization**: 6 experiments (✅ COMPLETE)
   - Parallel bgzip: 6.5× speedup (CPU, production-ready)
   - Smart mmap: 2.5× additional (≥50 MB threshold, macOS)
   - **Combined**: 16.3× I/O speedup for large files
   - **Decision**: Skip GPU (100% dynamic Huffman = 7-10 days)

4. **Cross-platform**: 27 experiments (✅ COMPLETE)
   - Mac M4 vs AWS Graviton 3 validation
   - Portability pillar validated

5. **DAG Statistical Rigor**: 307 experiments (✅ COMPLETE)
   - Week 1 Day 2 validation (3 batches)
   - N=30 statistical rigor, 95% CI, Cohen's d

### Four-Pillar Status: ✅ ALL VALIDATED

| Pillar | Status | Evidence |
|--------|--------|----------|
| 💰 Economic | ✅ Validated | 307 experiments, 16-25× NEON speedup |
| 🌱 Environmental | ✅ Validated | ARM efficiency (cross-platform validates) |
| 🔄 Portability | ✅ Validated | Cross-platform testing (Mac, Graviton) |
| 📊 Data Access | ✅ Validated | 72 streaming experiments, 99.5% memory reduction |

**Achievement**: 🎉 All 4 pillars validated experimentally! (November 4, 2025)

### Implementation Status

**biometal library**: ✅ Repository created, ready for Week 1-2
- Repository: https://github.com/shandley/biometal
- Evidence-based design (1,357 experiments → 6 rules)
- OPTIMIZATION_RULES.md (590 lines, dual-purpose)
- Clean structure: ~2,500 lines (vs ASBB's 50k+)
- Week 1-2: Core infrastructure (Nov 4-15, 2025)

---

## Core Development Principles

### 1. Evidence-Based Design (Not Intuition)

**Every design decision must be validated by experimental data.**

**Good example**:
```rust
// Block size: 10,000 sequences (from Benchmark 2)
// Evidence: Record-by-record causes 83-87% overhead
const DEFAULT_BLOCK_SIZE: usize = 10_000;
```

**Bad example**:
```rust
// Block size: 1,000 sequences (feels right?)
const DEFAULT_BLOCK_SIZE: usize = 1_000;  // NOT validated!
```

**Principle**: If we don't have experimental evidence, either:
1. Run a small pilot to measure (preferred)
2. Use conservative defaults and document assumption
3. Make it user-configurable with sensible default

### 2. Streaming-First Architecture

**Memory streaming is non-negotiable** for Data Access pillar.

**Implementation pattern**:
```rust
// GOOD: Streaming (constant memory)
pub fn process_fastq(path: &str) -> Result<Stats> {
    let mut stats = Stats::default();
    for block in FastqStream::open(path)?.blocks(10_000) {
        stats.update(process_block(&block)?);
    }
    Ok(stats)
}

// BAD: Load-all (12-24 TB RAM for 5TB dataset)
pub fn process_fastq(path: &str) -> Result<Stats> {
    let all_records = load_all(path)?;  // OOM!
    Ok(process_all(&all_records)?)
}
```

**Validation**: Benchmark 1 must show <100 MB constant memory

### 3. Block-Based Processing (Not Record-by-Record)

**Critical insight from Benchmark 2**: Record-by-record streaming destroys SIMD performance.

**Implementation pattern**:
```rust
// GOOD: Block-based (preserves NEON benefit)
impl FastqStream {
    pub fn blocks(&self, size: usize) -> BlockIterator {
        BlockIterator::new(self, size)  // Default: 10K
    }
}

for block in stream.blocks(10_000) {
    neon_process_batch(&block);  // Called 100 times for 1M seqs
}

// BAD: Record-by-record (83-87% overhead)
for record in stream.records() {
    neon_process_single(&record);  // Called 1M times - overhead dominates!
}
```

### 4. Network Streaming (Data Access Pillar)

**Problem**: 5TB SRA dataset requires 5TB local storage ($150-1500 cost)

**Solution**: Stream from HTTP/SRA with smart caching

**Implementation targets**:
- HTTP/HTTPS range requests (byte-range downloads)
- Smart LRU caching (user-controlled size budget)
- Background prefetching (overlap I/O with compute)
- Resume on failure (network interruptions handled)
- SRA toolkit integration (NCBI databases)

**See**: NETWORK_STREAMING_VISION.md for detailed architecture

### 5. Production Quality (Not Research Prototype)

**Non-negotiable requirements**:
- ✅ Comprehensive error handling (no panics on invalid input)
- ✅ Clear error messages (not "thread 'main' panicked...")
- ✅ Progress bars (operations >30 seconds)
- ✅ Memory safety (Rust guarantees + validation)
- ✅ Cross-platform (Mac, Linux, future RPi)
- ✅ Documentation (rustdoc + examples)

**Testing requirements**:
- Unit tests (correctness)
- Integration tests (real FASTQ files)
- Performance tests (verify speedups match DAG predictions)
- Error handling tests (malformed files, network failures)
- Memory leak tests (large-scale runs)

### 6. Lab Notebook Discipline

**ALL experimental work requires lab notebook documentation**:

1. Create entry BEFORE experiments
2. Document objective, methods, expected results
3. Run experiments
4. Document actual results, analysis
5. Update `lab-notebook/INDEX.md`
6. Commit entry + INDEX + results together

**Git pre-commit hook enforces this** - no commits without proper documentation.

**Entry naming**: `lab-notebook/YYYY-MM/YYYYMMDD-NNN-TYPE-name.md`

**Types**:
- EXP: Experimental validation
- IMPL: Implementation work
- DOC: Documentation updates
- REFACTOR: Code refactoring

---

## Development Workflow (biometal Implementation)

### Phase: Implementation (Nov 4 - Dec 20)

**Not**: More experiments (evidence base complete)
**Now**: Building production library based on evidence in separate repository (https://github.com/shandley/biometal)

### Typical Implementation Flow

**Week 1-2** (Core Infrastructure):
1. Implement streaming infrastructure in biometal repo
   - `FastqStream` with auto-compression detection
   - Block-based processing (10K default)
   - I/O optimization stack (parallel bgzip + smart mmap)
   - Progress bars
2. Implement 10 core operations
   - Naive + NEON variants
   - Auto-config logic (thresholds from DAG)
3. Build CLI tools
4. Write documentation

**Week 3-4** (Network Streaming):
1. Implement `StreamSource` abstraction
2. HTTP streaming with range requests
3. Smart LRU caching
4. Background prefetching
5. SRA toolkit integration

**Week 5-6** (Python + ML):
1. PyO3 bindings (`biofast-py`)
2. K-mer extraction for BERT
3. Batching utilities
4. Example notebooks (DNABert workflow)

**Week 7+** (Production Polish):
1. Large-scale testing
2. Cross-platform validation
3. Documentation completion
4. Publish to crates.io + PyPI

### Code Review Checklist

Before committing biometal code, verify:

- [ ] Evidence-based (cite experiment/benchmark if applicable)
- [ ] Streaming-first (constant memory)
- [ ] Block-based (if using SIMD)
- [ ] Error handling (no panics)
- [ ] Tests passing (unit + integration)
- [ ] Documentation (rustdoc comments)
- [ ] Examples (if public API)

---

## For Claude: Session Guidelines

### Current Phase Focus

**DO**:
- Focus on biometal implementation (evidence → code)
- Prioritize production quality over speed
- Validate design choices with existing benchmarks
- Document everything (rustdoc + examples)
- Test incrementally (don't build everything then test)

**DON'T**:
- Suggest more experiments (evidence base complete)
- Skip error handling ("we'll add it later")
- Guess at optimal values (use experimental data)
- Build features without validation plan

### Decision Framework

**When user asks "what's next?"**:
1. Check CURRENT_STATUS.md (streaming benchmarks → biofast v0.1.0)
2. Follow ROADMAP.md timeline (Week 1-2: core infrastructure)
3. NOT: More hardware dimensions, more experiments

**When implementing feature**:
1. What evidence backs this design?
2. Is it streaming-first?
3. Is it production-quality?
4. How will we validate it works?

**When user proposes design**:
1. Check if experimental evidence supports it
2. If no evidence, suggest small pilot first
3. Document assumptions clearly
4. Make it configurable if uncertain

### Common Scenarios

**Scenario 1: "Should we use X or Y for this?"**
- Check experimental results (did we test this?)
- If tested: Use evidence-backed choice
- If not tested: Suggest pilot OR use conservative default
- Document decision rationale

**Scenario 2: "This feature is taking longer than expected"**
- Check if it's MVP-critical (refer to ROADMAP.md)
- If not critical: Defer to later version
- If critical: Suggest simplified version for v0.1.0
- Quality over deadline (6-7 week timeline is flexible)

**Scenario 3: "Tests are failing"**
- DO NOT skip tests to make progress
- Investigate root cause
- Fix properly (don't band-aid)
- Production quality is non-negotiable

---

## Key Experimental Findings (Reference)

### Hardware Optimizations (DAG Validation)

**What works** (use these):
- NEON SIMD: 16-25× speedup for element-wise operations
- Parallelization: 4-21× speedup for >10K sequences
- NEON × Parallel: Multiplicative (combine them!)

**What doesn't work** (don't waste time):
- GPU Metal: Only helps for complexity >0.55 (rare)
- 2-bit encoding: 2-4× SLOWER (conversion overhead)
- AMX matrix engine: 0.92× vs NEON (stick with NEON)

### Streaming Insights (Benchmarks 1-3)

**Validated**:
- Memory reduction: 60-70% with streaming (Benchmark 1)
- Performance cost: 83-87% overhead with record-by-record (Benchmark 2)
- **Solution**: Block-based processing (10K chunks)

**Design implications**:
- Default block size: 10,000 sequences
- Process blocks with NEON (preserves speedup)
- Keep blocks in memory only during processing

### I/O Overhead Insights

**Finding**: 92.5% I/O overhead with batch + gzip
**Implication**: Streaming + smart caching critical for production
**Design choice**: Network streaming with prefetching (Week 3-4)

---

## Common Pitfalls to Avoid

### 1. Premature Optimization

**Bad**: "Let's optimize this before we know it's slow"
**Good**: "Let's measure first, then optimize if needed"

**Principle**: Build correctly first, optimize second (with profiling)

### 2. Guessing at Optimal Values

**Bad**: "10K block size sounds good"
**Good**: "Benchmark 2 shows 10K block size preserves NEON speedup"

**Principle**: Use experimental evidence, not intuition

### 3. Skipping Error Handling

**Bad**: "We'll add error handling later"
**Good**: "Every function returns Result<T> with proper error types"

**Principle**: Production quality from day 1

### 4. Load-All Pattern

**Bad**:
```rust
let all_records = load_all("5TB_dataset.fq.gz")?;  // OOM!
```

**Good**:
```rust
for block in FastqStream::open("5TB_dataset.fq.gz")?.blocks(10_000) {
    process(block)?;
}
```

**Principle**: Streaming-first, always

### 5. Record-by-Record with SIMD

**Bad**:
```rust
for record in stream.records() {
    neon_process(&record);  // 83-87% overhead!
}
```

**Good**:
```rust
for block in stream.blocks(10_000) {
    neon_process_batch(&block);  // Preserves NEON speedup
}
```

**Principle**: Block-based processing for SIMD operations

---

## Publication Strategy

### Paper 1: DAG Framework (BMC Bioinformatics)

**Status**: In preparation (submit Dec 2025)

**Novel contribution**: Systematic methodology for hardware testing
- Not ad-hoc benchmarking
- Reproducible framework
- Community can extend (test RPi, Ampere, etc.)

### Paper 2: biometal Library (Bioinformatics or JOSS)

**Status**: Design complete, implementation starting

**Target**: February 2026 submission

**Novel contributions**:
1. Evidence-based design (1,357 experiments)
2. Streaming architecture (memory + network)
3. I/O optimization stack (16.3× speedup for large files)
4. Production implementation (not prototype)

**Framing**: Democratization tool, not just performance

**Impact statement**:
> "biometal enables 5TB genomic analysis on $1,400 laptops without downloading data, eliminating economic, storage, and accessibility barriers. Validated by 1,357 experiments across ARM platforms."

---

## References

**Current Status**: `CURRENT_STATUS.md` (always up-to-date)
**Optimization Rules**: `OPTIMIZATION_RULES.md` (distilled from 1,357 experiments)
**Development Timeline**: `ROADMAP.md`
**Testing Methodology**: `DAG_FRAMEWORK.md`
**Lab Notebook**: `lab-notebook/INDEX.md`
**biometal Repository**: https://github.com/shandley/biometal

---

## Quick Command Reference

### Development

```bash
# ASBB repository (experimental validation)
cargo run --release --bin asbb-dag-traversal
cargo run --release --bin streaming-memory-benchmark-v2
cargo run --release --bin streaming-overhead-benchmark
cargo run --release --bin streaming-e2e-benchmark

# biometal repository (production library)
cd ../biometal
cargo build --release
cargo test
cargo run --release --bin biometal -- gc-content data.fq.gz
```

### Documentation

```bash
# Generate rustdoc (in biometal repo)
cd ../biometal
cargo doc --open

# Check documentation coverage
cargo +nightly rustdoc -- -Z unstable-options --show-coverage
```

### Validation

```bash
# Memory leak testing (in biometal repo)
cd ../biometal
valgrind --leak-check=full cargo run --release --bin biometal -- gc-content large.fq.gz

# Cross-platform testing (Graviton)
scp -r ../biometal/ ec2-graviton:~/
ssh ec2-graviton "cd biometal && cargo test"
```

---

**Last Updated**: November 4, 2025
**Current Phase**: Experimentation COMPLETE → biometal Development Starting
**Next Milestone**: biometal v0.1.0 (Nov 15, 2025)
**Owner**: Scott Handley + Claude
**Timeline**: 6-7 weeks to v1.0.0 (quality over deadline)

**For implementation**: Work in biometal repository (https://github.com/shandley/biometal)
**For optimization rules**: See OPTIMIZATION_RULES.md (Artifact 1)
**For development timeline**: See ROADMAP.md
**For experimental background**: See CURRENT_STATUS.md
