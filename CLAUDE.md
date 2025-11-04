# ASBB + biofast: Claude Development Guide

**Last Updated**: November 3, 2025

---

## 🚀 Current Phase: Evidence Base Complete → Implementation Starting

**Previous Phase** (Oct-Nov 2025): Systematic evidence gathering
- DAG hardware validation (307 experiments, statistically rigorous)
- Streaming characterization (72 experiments, 67% complete)
- I/O overhead analysis (48 experiments)
- **Total**: 1,100+ experiments backing biofast design

**Current Phase** (Nov 3-4, 2025): Complete streaming benchmarks
- Benchmark 1 v2: Memory footprint (RUNNING - corrected methodology)
- Benchmark 2: Streaming overhead (COMPLETE - 83-87% with record-by-record)
- Benchmark 3: End-to-end pipeline (PENDING)
- **Purpose**: Validate design choices before implementation

**Next Phase** (Nov 4 - Dec 20, 2025): biofast Development
- Week 1-2: Core infrastructure (v0.1.0 - local file streaming)
- Week 3-4: Network streaming (v0.2.0 - HTTP/SRA)
- Week 5-6: Python + ML integration (v0.3.0 - BERT-ready)
- Week 7+: Production polish (v1.0.0 - crates.io)

**Key Documents**:
- **CURRENT_STATUS.md**: Current state, evidence summary
- **BIOFAST_VISION.md**: Library design, network streaming, BERT integration
- **ROADMAP.md**: Detailed 6-7 week development timeline
- **NETWORK_STREAMING_VISION.md**: HTTP/SRA streaming architecture
- **DAG_FRAMEWORK.md**: Novel testing methodology

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

**Delivery Vehicle**: **biofast** - production Rust library with Python bindings

---

## Current Status (November 3, 2025)

### Evidence Base: COMPLETE ✅

**1,127 total experiments** (N=30 repetitions each = 33,810 measurements):

1. **DAG Hardware Validation**: 307 experiments (✅ COMPLETE)
   - Finding: NEON provides 16-25× speedup for element-wise operations
   - Finding: GPU/AMX/2-bit don't help (negative findings documented)
   - Publication: DAG framework paper (BMC Bioinformatics, in prep)

2. **I/O Overhead**: 48 experiments (✅ COMPLETE)
   - Finding: 92.5% I/O overhead with batch + gzip
   - Finding: 16× compute speedup → 2.16× end-to-end

3. **Streaming Characterization**: 72 experiments (⏳ 67% COMPLETE)
   - Benchmark 1 v2: Memory footprint (RUNNING)
   - Benchmark 2: Streaming overhead (COMPLETE - 83-87% with record-by-record)
   - Benchmark 3: E2E pipeline (PENDING)
   - **Key insight**: Block-based (10K chunks) required, not record-by-record

4. **Cross-platform**: 27 experiments (✅ COMPLETE)
   - Mac M4 vs AWS Graviton 3 validation
   - Portability pillar validated

5. **Power consumption**: 24 experiments (✅ COMPLETE)
   - Environmental pillar validated

### Four-Pillar Status

| Pillar | Status | Evidence |
|--------|--------|----------|
| 💰 Economic | ✅ Validated | 307 experiments, 16-25× NEON speedup |
| 🌱 Environmental | ✅ Validated | ARM efficiency (portability validates this) |
| 🔄 Portability | ✅ Validated | Cross-platform testing (Mac, Graviton) |
| 📊 Data Access | ⏳ In Progress | Streaming benchmarks 67% complete |

**Target**: All 4 pillars validated by biofast v0.3.0 (Dec 13, 2025)

### Implementation Status

**biofast library**: 📋 Design complete, implementation starts Nov 4
- Evidence-based design (1,100+ experiments)
- Streaming architecture validated (Benchmark 1-3)
- Block-based processing (10K chunks from Benchmark 2)
- Network streaming design (HTTP/SRA)
- BERT integration design (PyO3 bindings)

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

## Development Workflow (biofast Implementation)

### Phase: Implementation (Nov 4 - Dec 20)

**Not**: More experiments (evidence base complete)
**Now**: Building production library based on evidence

### Typical Implementation Flow

**Week 1-2** (Core Infrastructure):
1. Create `crates/biofast/` package
2. Implement streaming infrastructure
   - `FastqStream` with auto-compression detection
   - Block-based processing (10K default)
   - Progress bars
3. Implement 10 core operations
   - Naive + NEON variants
   - Auto-config logic (thresholds from DAG)
4. Build CLI tools
5. Write documentation

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

Before committing biofast code, verify:

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
- Focus on biofast implementation (evidence → code)
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

### Paper 2: biofast Library (Bioinformatics or JOSS)

**Status**: Design complete, implementation starting

**Target**: February 2026 submission

**Novel contributions**:
1. Evidence-based design (1,100+ experiments)
2. Streaming architecture (memory + network)
3. BERT integration (eliminates preprocessing bottleneck)
4. Production implementation (not prototype)

**Framing**: Democratization tool, not just performance

**Impact statement**:
> "biofast enables 5TB genomic analysis on $1,400 laptops without downloading data, eliminating economic, storage, and accessibility barriers. Validated by 1,100+ experiments across ARM platforms."

---

## References

**Current Status**: `CURRENT_STATUS.md` (always up-to-date)
**Library Design**: `BIOFAST_VISION.md`
**Development Timeline**: `ROADMAP.md`
**Network Streaming**: `NETWORK_STREAMING_VISION.md`
**Testing Methodology**: `DAG_FRAMEWORK.md`
**Lab Notebook**: `lab-notebook/INDEX.md`

---

## Quick Command Reference

### Development

```bash
# Build biofast library
cargo build --release -p biofast

# Run tests
cargo test -p biofast

# Run CLI tool
cargo run --release --bin biofast -- gc-content data.fq.gz

# Build Python bindings
cd biofast-py && maturin develop

# Run streaming benchmarks
cargo run --release --bin streaming-memory-benchmark-v2
cargo run --release --bin streaming-overhead-benchmark
cargo run --release --bin streaming-e2e-benchmark
```

### Documentation

```bash
# Generate rustdoc
cargo doc --open -p biofast

# Check documentation coverage
cargo +nightly rustdoc -p biofast -- -Z unstable-options --show-coverage
```

### Validation

```bash
# Memory leak testing
valgrind --leak-check=full cargo run --release --bin biofast -- gc-content large.fq.gz

# Cross-platform testing (Graviton)
scp -r biofast/ ec2-graviton:~/
ssh ec2-graviton "cd biofast && cargo test"
```

---

**Last Updated**: November 3, 2025 (20:00 PST)
**Current Phase**: Streaming benchmarks (67% complete)
**Next Milestone**: biofast v0.1.0 development starts (Nov 4)
**Owner**: Scott Handley + Claude
**Timeline**: 6-7 weeks to v1.0.0 (quality over deadline)

**For implementation guidance**: See BIOFAST_VISION.md and ROADMAP.md
**For experimental background**: See CURRENT_STATUS.md
**For network streaming**: See NETWORK_STREAMING_VISION.md (to be created)
