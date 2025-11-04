# biofast: 6-7 Week Development Roadmap

**Goal**: Transform evidence base (1,100+ experiments) into production library

**Timeline**: November 4 - December 20, 2025

---

## Overview

**Current Phase** (Nov 3-4): Streaming Benchmarks (67% complete)
- Validating streaming architecture design choices
- Measuring memory reduction, performance overhead
- Informing block-based processing implementation

**Next Phase** (Nov 4 - Dec 20): biofast Development (6-7 weeks)
- Week 1-2: Core Infrastructure (v0.1.0 - local file streaming)
- Week 3-4: Network Streaming (v0.2.0 - HTTP/SRA)
- Week 5-6: Python + ML Integration (v0.3.0 - BERT-ready)
- Week 7+: Production Polish (v1.0.0 - crates.io)

---

## Phase 0: Streaming Benchmarks (Nov 3-4, 2025)

### Status: 67% Complete

**Purpose**: Validate streaming architecture design before implementation

**Benchmark 1: Memory Footprint** (⏳ RUNNING)
- Question: How much memory does streaming actually save?
- Status: Running corrected v2 (fork-per-experiment, real FASTQ I/O)
- Expected completion: Nov 3 evening
- Early results: 60-70% reduction

**Benchmark 2: Streaming Overhead** (✅ COMPLETE)
- Question: What's the performance cost of streaming?
- Status: Complete (48 experiments, N=30)
- **Finding**: 83-87% overhead with record-by-record NEON
- **Design decision**: Use block-based (10K chunks), not record-by-record

**Benchmark 3: End-to-End Pipeline** (⏸️ PENDING)
- Question: Real-world performance with file I/O + filtering?
- Status: Implemented, ready to run
- Expected completion: Nov 4 morning

**Deliverables**:
- `results/streaming/streaming_memory_v2_n30.csv`
- `results/streaming/streaming_overhead_n30.csv`
- `results/streaming/streaming_e2e_n30.csv`
- `results/streaming/FINDINGS.md` (comprehensive analysis)
- Performance plots (memory, overhead, E2E)

**Timeline**: Complete by Nov 4, 12pm

---

## Week 1-2: Core Infrastructure + I/O Optimization (Nov 4-15, 2025)

### Goal: biofast v0.1.0 - Local File Streaming with I/O Optimization

**Focus**: Production-quality streaming architecture with 16.3× I/O speedup for large files
- CPU parallel bgzip decompression (6.5×, all platforms)
- Smart mmap + APFS optimization (2.5× additional, macOS, threshold-based)

### Day 1-3 (Nov 4-6): Streaming Foundation

**Morning Sessions**:
- [ ] Create `crates/biofast/` package structure
- [ ] Implement core types:
  - `FastqRecord` (id, sequence, quality)
  - `FastaRecord` (id, sequence)
  - `SequenceQuality` (Phred33/64 handling)

- [ ] Implement `FastqStream` with I/O optimization stack:
  - Auto-detect compression (gzip, bgzip, zstd, uncompressed)
  - **I/O optimization stack** (16.3× speedup for large files):
    - CPU parallel bgzip decompression (6.5×, all platforms)
    - Smart mmap + APFS optimization (2.5× additional, macOS, threshold-based 50 MB)
  - Buffered reading (optimal buffer size)
  - Progress bar support (optional)
  - Error handling (malformed records, corrupted compression)

**Afternoon Sessions**:
- [ ] Implement block-based processing:
  - Default block size: 10,000 sequences (from Benchmark 2)
  - Configurable for special cases
  - `.blocks(size)` iterator method

- [ ] Add compression support with I/O optimization:
  - flate2 for gzip
  - zstd for Zstandard
  - rayon for parallel bgzip decompression
  - memmap2 + libc for smart mmap (macOS)
  - Auto-detection from file extension + magic bytes

**Evening**:
- [ ] Unit tests:
  - Test with gzip/zstd/uncompressed
  - Test malformed FASTQ
  - Test edge cases (empty, single record)

**Deliverable**: Streaming infrastructure passing all tests

### Day 4-6 (Nov 7-9): Core Operations

**Implementation order** (10 operations):
1. `gc_content` - GC percentage calculation
2. `base_counting` - Count A/C/G/T bases
3. `quality_filter` - Filter by mean quality
4. `quality_aggregation` - Quality statistics
5. `length_filter` - Filter by sequence length
6. `sequence_length` - Calculate lengths
7. `n_content` - Count ambiguous bases
8. `reverse_complement` - Reverse complement
9. `at_content` - AT percentage
10. `adapter_trimming` - Remove adapter sequences

**Each operation includes**:
- Naive implementation (portable)
- NEON implementation (ARM SIMD)
- Auto-config logic (thresholds from DAG validation)
- Unit tests (correctness + performance)

**Pattern**:
```rust
pub struct GcContentOp {
    config: GcConfig,
}

pub enum GcConfig {
    Naive,              // < 1K sequences
    Neon,               // 1K-10K sequences
    NeonParallel(usize), // >10K sequences
}

impl GcContentOp {
    pub fn auto_config(dataset_size: usize) -> GcConfig {
        // Thresholds from DAG validation
        match dataset_size {
            n if n < 1_000 => GcConfig::Naive,
            n if n < 10_000 => GcConfig::Neon,
            _ => GcConfig::NeonParallel(optimal_threads())
        }
    }
}
```

**Deliverable**: 10 operations implemented with auto-optimization

### Day 7-10 (Nov 10-13): API Design + CLI

**Streaming API**:
```rust
biofast::stream("data.fq.gz")?
    .filter_quality(30)
    .gc_content()
    .compute()?
```

**CLI Tool**:
```bash
biofast gc-content data.fq.gz
biofast filter --min-quality 30 input.fq.gz -o filtered.fq.gz
```

**Tasks**:
- [ ] Implement streaming API (builder pattern)
- [ ] Implement CLI with clap
- [ ] Add progress bars (indicatif)
- [ ] Add statistics output (--stats flag)
- [ ] Documentation (rustdoc)
- [ ] Examples (examples/ directory)

**Deliverable**: biofast v0.1.0 ready for release

### Day 11 (Nov 14): Testing + Documentation

- [ ] Integration tests (real FASTQ files)
- [ ] Performance validation (compare to benchmarks)
- [ ] Documentation review (README, API docs)
- [ ] Examples polish

**Deliverable**: biofast v0.1.0 RELEASED (local file streaming)

---

## Week 3-4: Network Streaming (Nov 18-29, 2025)

### Goal: biofast v0.2.0 - HTTP/SRA Streaming

**Focus**: Eliminate storage barrier (analyze 5TB datasets with 50GB cache)

### Day 12-14 (Nov 18-20): HTTP Streaming

**Implementation**:
- [ ] `StreamSource` abstraction:
  ```rust
  enum StreamSource {
      LocalFile(PathBuf),
      Http(Url),
      Sra(String),
  }
  ```

- [ ] HTTP source with range requests:
  - Use `reqwest` with byte-range support
  - Chunked downloads (1MB chunks)
  - Resume on network failure
  - Progress tracking

**Features**:
- [ ] Smart caching:
  - LRU cache (user-controlled size)
  - Cache to `~/.biofast/cache/` by default
  - `--cache-size` flag (e.g., `--cache-size 50GB`)
  - Automatic eviction when full

- [ ] Background prefetching:
  - Download next chunks while processing current
  - Overlap I/O with compute
  - Configurable prefetch distance

**Testing**:
- [ ] Test with real HTTP URLs
- [ ] Test network interruptions (simulated)
- [ ] Test cache behavior (LRU eviction)

**Deliverable**: HTTP streaming working

### Day 15-18 (Nov 21-26): SRA Integration

**Implementation**:
- [ ] SRA Toolkit integration:
  - Use `fasterq-dump` for SRA downloads
  - Stream from SRA accessions (SRR*, ERR*, DRR*)
  - Handle SRA metadata (read length, quality format)

- [ ] API:
  ```rust
  biofast::stream::from_sra("SRR12345678")?
      .filter_quality(30)
      .gc_content()
      .compute()?
  ```

**Features**:
- [ ] Auto-download SRA toolkit if missing
- [ ] Smart caching (don't re-download)
- [ ] Progress tracking (SRA downloads can be large)

**Testing**:
- [ ] Test with real SRA accessions
- [ ] Test caching behavior
- [ ] Cross-platform (Mac, Linux)

**Deliverable**: SRA streaming working

### Day 19-20 (Nov 27-28): Polish + Release

- [ ] Documentation (NETWORK_STREAMING_VISION.md)
- [ ] Examples (HTTP + SRA workflows)
- [ ] Performance validation
- [ ] Integration tests

**Deliverable**: biofast v0.2.0 RELEASED (network streaming)

---

## Week 5-6: Python + ML Integration (Dec 2-13, 2025)

### Goal: biofast v0.3.0 - BERT-Ready

**Focus**: Eliminate preprocessing bottleneck for ML workflows

### Day 21-23 (Dec 2-4): PyO3 Bindings

**Implementation**:
- [ ] Create `biofast-py` crate (PyO3)
- [ ] Python API:
  ```python
  from biofast import FastqStream

  gc = FastqStream("data.fq.gz").gc_content()
  ```

- [ ] Core bindings:
  - `FastqStream` class
  - All 10 operations
  - Progress bars (tqdm integration)
  - Error handling (Python exceptions)

**Type hints**:
- [ ] MyPy-compatible type stubs
- [ ] Sphinx documentation

**Testing**:
- [ ] pytest test suite
- [ ] Example notebooks (Jupyter)

**Deliverable**: Python bindings working

### Day 24-26 (Dec 5-9): BERT Preprocessing

**Implementation**:
- [ ] K-mer extraction:
  - `.kmers(k)` method (k=3 to k=12)
  - Returns numpy arrays (zero-copy)
  - Batch iterator (`.batch(32)`)

- [ ] PyTorch integration:
  ```python
  for batch in FastqStream("data.fq.gz") \
      .filter_quality(30) \
      .kmers(k=6) \
      .batch(32):

      tokens = torch.from_numpy(batch)
      predictions = dnabert_model(tokens)
  ```

**Features**:
- [ ] Automatic batching for GPU efficiency
- [ ] Progress bars (tqdm)
- [ ] Memory-efficient (streaming)

**Example notebooks**:
- [ ] DNABert preprocessing workflow
- [ ] Metagenomic classification
- [ ] Quality control for ML datasets

**Deliverable**: BERT preprocessing working

### Day 27-28 (Dec 10-12): SRA Streaming (Python)

**Implementation**:
- [ ] `stream_from_sra()` function:
  ```python
  from biofast import stream_from_sra

  for batch in stream_from_sra("SRR12345678") \
      .filter_quality(30) \
      .kmers(k=6) \
      .batch(32):

      predictions = model(batch)
  ```

**Testing**:
- [ ] Real SRA accessions
- [ ] Large-scale datasets (1M+ reads)
- [ ] Notebook examples

**Deliverable**: Python SRA streaming working

### Day 29 (Dec 13): Release Prep

- [ ] Documentation (Python API docs)
- [ ] PyPI packaging
- [ ] Example notebooks polished
- [ ] Tutorial (DNABert workflow)

**Deliverable**: biofast v0.3.0 RELEASED (ML-ready)

---

## Week 7+: Production Polish (Dec 16-20, 2025)

### Goal: biofast v1.0.0 - Production Release

**Focus**: Production quality, comprehensive documentation

### Day 30-32 (Dec 16-18): Quality Assurance

**Testing**:
- [ ] Large-scale testing (1M+ sequences)
- [ ] Real datasets (not synthetic)
- [ ] Cross-platform (Mac M4, Linux Graviton)
- [ ] Error handling (malformed files, network failures)
- [ ] Memory leak testing (valgrind)

**Performance validation**:
- [ ] Verify speedups match DAG predictions
- [ ] Verify memory usage <100 MB constant
- [ ] Verify network streaming works at scale

**Bug fixes**:
- [ ] Address any issues found
- [ ] Edge case handling
- [ ] Error message improvements

### Day 33-34 (Dec 19-20): Documentation + Release

**Documentation**:
- [ ] Comprehensive README (crates.io)
- [ ] API documentation (rustdoc)
- [ ] Usage guide (examples)
- [ ] Tutorial (getting started)
- [ ] FAQ (common questions)

**Release preparation**:
- [ ] Version bump (1.0.0)
- [ ] Changelog (all versions)
- [ ] License (Apache 2.0)
- [ ] CONTRIBUTING.md

**Publication**:
- [ ] Publish to crates.io (Rust)
- [ ] Publish to PyPI (Python)
- [ ] Publish to Conda (bioconda)

**Deliverable**: biofast v1.0.0 RELEASED (production-ready)

---

## Post-Release (Dec 21+, 2025)

### Week 8: Community Engagement

**Tasks**:
- [ ] Monitor issues/PRs
- [ ] Respond to user questions
- [ ] Fix critical bugs
- [ ] Gather feedback

**Metrics**:
- Track downloads (crates.io, PyPI)
- Track GitHub stars
- Track citations

### Week 9-10: Paper Writing

**Manuscript** (biofast library paper):
- Target: Bioinformatics or JOSS
- Target submission: February 2026

**Sections**:
1. Introduction (bioinformatics accessibility crisis)
2. Methods (evidence-based design, DAG validation)
3. Implementation (streaming architecture, auto-optimization)
4. Results (performance validation, case studies)
5. Discussion (democratization impact)

**Figures**:
- Figure 1: Architecture diagram (streaming + network)
- Figure 2: Performance validation (speedups)
- Figure 3: Memory usage (streaming vs load-all)
- Figure 4: BERT workflow (before/after biofast)
- Figure 5: Case studies (real datasets)

### Month 2-3: Extensions

**Additional operations**:
- [ ] minhash_sketching (for genome similarity)
- [ ] translation (6-frame translation)
- [ ] hamming_distance (error correction)
- [ ] edit_distance (alignment-free comparison)

**Additional platforms**:
- [ ] Raspberry Pi 5 validation
- [ ] Ampere Altra testing
- [ ] Azure Cobalt testing

**Community contributions**:
- [ ] Accept PRs for new operations
- [ ] Accept PRs for new platforms
- [ ] Build community around biofast

---

## Success Metrics

### Technical Excellence
- [ ] All tests passing (unit + integration)
- [ ] Performance matches DAG predictions (±10%)
- [ ] Memory usage <100 MB constant
- [ ] Network streaming works at 5TB scale
- [ ] Cross-platform (Mac, Linux, future RPi)

### Adoption
- [ ] 1,000+ downloads (first month)
- [ ] 100+ GitHub stars (first 3 months)
- [ ] 10+ community contributions (first 6 months)
- [ ] 5+ citations (first year)

### Impact
- [ ] Enables 5TB analysis on consumer hardware
- [ ] Eliminates preprocessing bottleneck for ML
- [ ] Removes storage barrier (network streaming)
- [ ] Democratizes bioinformatics (LMIC, students, small labs)

---

## Risk Mitigation

### Risk 1: Implementation Takes Longer Than Expected

**Mitigation**:
- MVP: 5 operations is sufficient for v0.1.0 (not all 10)
- Network streaming can be v0.2.0 (not required for v0.1.0)
- Python bindings can be v0.3.0 (not required for Rust users)

**Fallback plan**: Release early, iterate based on feedback

### Risk 2: Network Streaming Challenges

**Mitigation**:
- Start with HTTP (simpler than SRA)
- Use existing libraries (reqwest, well-tested)
- Test incrementally (small files → large files)

**Fallback plan**: Local file streaming (v0.1.0) is still valuable

### Risk 3: Python Bindings Complexity

**Mitigation**:
- PyO3 is mature, well-documented
- Start simple (core operations only)
- Expand based on user demand

**Fallback plan**: Rust-only release is still valuable for developers

### Risk 4: Performance Not Matching Predictions

**Mitigation**:
- Block-based processing (validated by Benchmark 2)
- Auto-optimization (validated by DAG)
- Streaming overhead measured (Benchmark 2)

**Contingency**: Profile and optimize if needed (buffer in timeline)

---

## Dependencies

### Critical Path

1. Streaming Benchmarks MUST complete first (Nov 3-4)
   - Validates design choices
   - Informs block size selection
   - Measures overhead

2. Core Infrastructure MUST work before network streaming (Week 1-2)
   - Foundation for all features
   - Local files simpler to debug

3. Rust Library MUST work before Python bindings (Week 1-4 before Week 5-6)
   - Python wraps Rust
   - Can't bind non-existent library

### Non-Critical Path

- Python bindings can proceed in parallel with network streaming
- Documentation can be written alongside implementation
- Testing can be incremental

---

## Checkpoints

### Checkpoint 1: Week 2 (Nov 15)
**Required**:
- ✅ biofast v0.1.0 released
- ✅ 10 operations working
- ✅ Streaming architecture validated
- ✅ CLI tools functional

**Go/No-Go**: Can proceed to network streaming if core working

### Checkpoint 2: Week 4 (Nov 29)
**Required**:
- ✅ biofast v0.2.0 released
- ✅ HTTP streaming working
- ✅ SRA streaming working
- ✅ Smart caching functional

**Go/No-Go**: Can proceed to Python bindings if network working

### Checkpoint 3: Week 6 (Dec 13)
**Required**:
- ✅ biofast v0.3.0 released
- ✅ Python bindings working
- ✅ BERT preprocessing functional
- ✅ Example notebooks ready

**Go/No-Go**: Can proceed to v1.0.0 release if all features working

### Checkpoint 4: Week 7 (Dec 20)
**Required**:
- ✅ biofast v1.0.0 released
- ✅ Published to crates.io + PyPI
- ✅ Comprehensive documentation
- ✅ All tests passing

**Success**: Production-ready library available

---

**Last Updated**: November 3, 2025
**Status**: Phase 0 (Streaming Benchmarks) 67% complete
**Next Milestone**: Complete streaming benchmarks (Nov 4)
**First Release**: biofast v0.1.0 (Nov 15, 2025)
**Production Release**: biofast v1.0.0 (Dec 20, 2025)

**Owner**: Scott Handley + Claude
**Timeline**: 6-7 weeks (flexible based on quality, not deadline)
