# biofast: Production Library Vision

**Fast, Memory-Efficient Bioinformatics on Consumer Hardware**

---

## Mission

Deliver the democratization promise through a production-ready library that enables 5TB analysis on $1.4K laptops.

**Not**: Research prototype or proof-of-concept
**But**: Production tool researchers can use today

---

## Design Principles

### 1. Auto-Optimization (No Manual Tuning)

**Problem**: Existing tools require manual configuration
- "Should I use SIMD?" → User doesn't know
- "How many threads?" → User guesses
- "When does GPU help?" → Trial and error

**Solution**: Empirically validated auto-selection
```rust
// Automatically selects optimal config based on:
// - Operation type (base_counting, gc_content, etc.)
// - Dataset size (estimated from file size or counted)
// - Available hardware (cores, SIMD support)
biofast::stream("data.fq.gz")?
    .gc_content()  // Auto-selects: naive/NEON/NEON+parallel
    .compute()?
```

**Implementation**:
- Thresholds from DAG traversal (1,640 experiments)
- Per-operation optimal configs
- Runtime hardware detection
- No user configuration required

### 2. Streaming Architecture (Memory Efficient)

**Problem**: Traditional tools load entire dataset into RAM
- 5TB dataset → 12-24 TB RAM required
- Excludes consumer hardware ($1.4K laptops)
- Prevents field work, LMIC research

**Solution**: Constant memory streaming
```rust
// Processes 5TB with <100 MB RAM
biofast::stream("5TB_data.fq.gz")?
    .filter_quality(20)
    .gc_content()
    .write_to("filtered.fq.gz")?
```

**Memory guarantee**: <100 MB constant (vs 12 TB load-all)

**Validation**: Entry 023 will measure actual memory usage

### 3. Cross-Platform Portability

**Problem**: Tools optimized for specific platforms (x86, specific cloud provider)

**Solution**: ARM-first, cross-platform design
- Mac M1/M2/M3/M4 (development, local analysis)
- AWS Graviton (cloud bursting)
- Raspberry Pi 5 (field work, education)
- Future: Ampere Altra, Azure Cobalt

**Guarantee**: Same code, validated performance across ARM platforms

### 4. Production Quality

**Not acceptable**: Research prototype that crashes
- Silent failures
- Cryptic error messages
- No progress indication
- Memory leaks

**Required**: Production standards
- ✅ Comprehensive error handling
- ✅ Progress bars for long operations
- ✅ Compressed I/O (gzip, zstd)
- ✅ Graceful degradation (fallback to naive if SIMD unavailable)
- ✅ Memory safety (Rust guarantees + validation)
- ✅ CLI tools for common tasks
- ✅ Documentation with examples

---

## Architecture

### Core Abstraction: Streaming Operations

```rust
pub trait StreamingOperation {
    type Input;
    type Output;
    type Config;

    // Auto-select optimal config based on dataset characteristics
    fn auto_config(dataset_size: usize, num_cpus: usize) -> Self::Config;

    // Process one record at a time (constant memory)
    fn process_one(&self, input: Self::Input) -> Self::Output;

    // Optional: Batch processing for operations that benefit
    fn process_batch(&self, inputs: &[Self::Input]) -> Vec<Self::Output> {
        inputs.iter().map(|i| self.process_one(i)).collect()
    }
}
```

### Example: GC Content

```rust
pub struct GcContentOp {
    config: GcConfig,
}

pub enum GcConfig {
    Naive,              // < 1K sequences
    Neon,               // 1K-10K sequences
    NeonParallel(usize), // >10K sequences, N threads
}

impl StreamingOperation for GcContentOp {
    type Input = FastqRecord;
    type Output = f64;
    type Config = GcConfig;

    fn auto_config(dataset_size: usize, num_cpus: usize) -> GcConfig {
        // Thresholds from Entry 022 (DAG completion)
        match dataset_size {
            n if n < 1_000 => GcConfig::Naive,
            n if n < 10_000 => GcConfig::Neon,
            _ => {
                let threads = (num_cpus / 2).max(4).min(8);
                GcConfig::NeonParallel(threads)
            }
        }
    }

    fn process_one(&self, record: FastqRecord) -> f64 {
        match self.config {
            GcConfig::Naive => gc_content_naive(&record.seq),
            GcConfig::Neon => gc_content_neon(&record.seq),
            // Parallel handled at higher level (batching)
            GcConfig::NeonParallel(_) => gc_content_neon(&record.seq),
        }
    }
}
```

### Streaming Pipeline

```rust
pub struct FastqStream<R: Read> {
    reader: BufReader<R>,
    decompressor: Option<Decompressor>,
    progress: Option<ProgressBar>,
}

impl<R: Read> FastqStream<R> {
    pub fn open<P: AsPath>(path: P) -> Result<Self> {
        // Auto-detect compression (gzip, zstd, uncompressed)
        // Setup progress bar
        // Validate file format
    }

    pub fn gc_content(self) -> GcContentStream<R> {
        let config = GcContentOp::auto_config(
            self.estimate_size(),
            num_cpus::get()
        );
        GcContentStream::new(self, config)
    }

    pub fn filter_quality(self, min_q: u8) -> QualityFilterStream<R> {
        // Chain operations (streaming)
    }

    pub fn write_to<P: AsPath>(self, path: P) -> Result<()> {
        // Stream records to output
    }
}
```

---

## Operations

### Phase 1: 10 Core Operations (Week 2)

**Element-wise (simple)**:
1. `gc_content` - GC percentage calculation
2. `base_counting` - Count A/C/G/T bases
3. `sequence_length` - Calculate lengths
4. `n_content` - Count ambiguous bases

**Filtering**:
5. `quality_filter` - Filter by mean quality
6. `length_filter` - Filter by sequence length

**Aggregation**:
7. `quality_aggregation` - Mean/min/max quality stats
8. `quality_statistics` - Comprehensive quality metrics

**Transform**:
9. `reverse_complement` - Reverse complement sequences

**Complex**:
10. `adapter_trimming` - Remove adapter sequences

### Phase 2: 10 Additional Operations (Future)

- kmer_counting
- kmer_extraction
- translation
- minhash_sketching
- hamming_distance
- edit_distance
- sequence_masking
- complexity_score
- at_content
- fastq_parsing (format conversion)

---

## CLI Tools

### biofast Command

```bash
# GC content calculation
biofast gc-content data.fq.gz

# Quality filtering
biofast filter --min-quality 20 input.fq.gz -o filtered.fq.gz

# Adapter trimming
biofast trim-adapters --adapter AGATCGGAAGAG input.fq.gz -o trimmed.fq.gz

# Multiple operations (streaming pipeline)
biofast trim-adapters input.fq.gz | biofast filter --min-quality 20 > clean.fq

# Progress and stats
biofast gc-content --progress --stats data.fq.gz
```

### Batch Processing

```bash
# Process multiple files
biofast gc-content --batch *.fq.gz -o results.csv

# Parallel file processing
find . -name "*.fq.gz" | parallel biofast gc-content
```

---

## API Examples

### Simple: GC Content

```rust
use biofast::stream::FastqStream;

fn main() -> Result<()> {
    let gc = FastqStream::open("data.fq.gz")?
        .gc_content()
        .compute()?;

    println!("GC content: {:.2}%", gc * 100.0);
    Ok(())
}
```

### Pipeline: Filter + Trim + Stats

```rust
use biofast::stream::FastqStream;

fn main() -> Result<()> {
    FastqStream::open("raw.fq.gz")?
        .filter_quality(20)           // Remove low-quality reads
        .trim_adapters("AGATCGGAAGAG") // Remove adapters
        .quality_statistics()          // Calculate stats
        .write_to("clean.fq.gz")?;

    Ok(())
}
```

### Advanced: Custom Operation

```rust
use biofast::stream::{FastqStream, StreamingOperation};

struct CustomFilter { /* ... */ }

impl StreamingOperation for CustomFilter {
    // Implement trait
}

fn main() -> Result<()> {
    FastqStream::open("data.fq.gz")?
        .apply(CustomFilter::new())  // Custom operation
        .write_to("filtered.fq.gz")?;

    Ok(())
}
```

---

## Performance Guarantees

### Based on 1,640 Experiments

**Speedups** (compared to naive implementations):
- Simple operations (gc_content, base_counting): 40-80×
- Complex operations (quality_aggregation): 16-30×
- Filtering operations: 10-20×

**Memory usage**:
- Constant: <100 MB regardless of dataset size
- Validated: Entry 023 (streaming validation)

**Energy efficiency**:
- 1.95-3.27× more efficient than naive
- Validated: Entry 020 (24 experiments)

**Cross-platform**:
- Mac M4: Reference performance
- AWS Graviton 3: 0.8-1.14× relative (validated)
- Raspberry Pi 5: TBD (future validation)

---

## Validation Strategy

### Entry 023: Streaming Architecture Validation

**Experiments** (~50):
1. Memory usage measurement
   - Load-all vs streaming
   - Target: <100 MB constant
   - Scales: 1K, 10K, 100K, 1M, 10M sequences

2. Performance overhead
   - Streaming vs load-all
   - Target: <10% overhead
   - All 10 operations

3. Auto-selection accuracy
   - Verify auto-config chooses optimal
   - Test at threshold boundaries (900, 1100, 9000, 11000 sequences)

### Entry 024: Production Validation

**Experiments** (~30):
1. Cross-platform performance
   - Mac M4 vs AWS Graviton 3
   - Verify speedups match DAG predictions

2. Large-scale tests
   - 1M+ sequence datasets
   - Real compressed FASTQ (not synthetic)

3. Error handling
   - Malformed files
   - Out of memory scenarios
   - Corrupted compression

---

## Success Criteria

### Scientific Validation ✅
- Memory usage <100 MB (measured, not calculated)
- Performance overhead <10% vs load-all
- Auto-selection accuracy >95%
- Cross-platform speedups match predictions

### Production Quality ✅
- Zero crashes on valid input
- Graceful failures with clear error messages
- Progress bars for operations >30 seconds
- Comprehensive documentation with examples

### Usability ✅
- Install: `cargo add biofast`
- CLI tools work out-of-box
- No configuration required (auto-optimization)
- Examples cover common use cases

### Impact ✅
- Enables 5TB analysis on $1.4K laptop
- Researchers adopt (track downloads, citations)
- Community contributions (issues, PRs)

---

## Comparison to Existing Tools

### vs. Traditional Tools (samtools, seqtk)

**Traditional**:
- ❌ Load-all pattern (high memory)
- ❌ No auto-optimization
- ❌ Single-threaded or manual threading
- ✅ Mature, widely used

**biofast**:
- ✅ Streaming (constant memory)
- ✅ Auto-optimization (no configuration)
- ✅ Empirically validated thresholds
- ⚠️ New (needs adoption)

### vs. GPU Tools (NVIDIA Clara, GPU-accelerated)

**GPU tools**:
- ❌ Requires expensive hardware ($1,000+ GPU)
- ❌ Vendor lock-in (CUDA)
- ❌ High power consumption
- ✅ Very fast for some operations

**biofast**:
- ✅ Consumer hardware ($1.4K laptop)
- ✅ Cross-platform ARM
- ✅ Energy efficient (1.95-3.27×)
- ✅ Fast enough (40-80× vs naive)

### vs. Research Prototypes

**Research code**:
- ❌ Crashes on edge cases
- ❌ No error handling
- ❌ Poor documentation
- ❌ Unmaintained

**biofast**:
- ✅ Production quality
- ✅ Comprehensive error handling
- ✅ Full documentation
- ✅ Active maintenance

---

## Non-Goals

**What biofast is NOT**:
1. ❌ General-purpose FASTQ parser (use rust-bio for that)
2. ❌ Alignment tool (use minimap2, BWA)
3. ❌ Variant caller (use bcftools, GATK)
4. ❌ GPU-first tool (ARM NEON is the focus)
5. ❌ x86-specific (ARM-first design, x86 fallback)

**What biofast IS**:
1. ✅ Fast, memory-efficient common operations
2. ✅ Production-ready library for ARM platforms
3. ✅ Auto-optimizing (no manual tuning)
4. ✅ Streaming architecture (constant memory)
5. ✅ Democratization tool (enables underserved researchers)

---

## Roadmap

### Week 2: Core Implementation

**Day 1-2**: Streaming architecture
- FastqStream abstraction
- Auto-detection (compression, format)
- Progress bars

**Day 3**: 10 operations
- Implement StreamingOperation for each
- Auto-config logic (thresholds from DAG)
- Unit tests

**Day 4**: CLI tools
- biofast command with subcommands
- Error handling
- Documentation

### Week 3: Validation

**Day 1**: Performance validation
- Entry 023: Streaming validation
- Memory usage, overhead, auto-selection

**Day 2**: Production testing
- Entry 024: Large-scale tests
- Real data (not synthetic)
- Error handling edge cases

**Day 3-5**: Documentation + Polish
- Examples
- API documentation
- Usage guide
- README for crates.io

### Post-Launch: Community

**Month 1**: Adoption
- Track downloads
- Respond to issues
- Fix bugs

**Month 2-3**: Extensions
- Add remaining 10 operations
- Community-contributed operations
- Additional platforms (RPi, Ampere)

---

## License and Distribution

**License**: MIT or Apache 2.0 (permissive, research-friendly)

**Distribution**:
- crates.io (primary)
- Conda/Bioconda (for bioinformatics community)
- Docker images (for reproducibility)

**Citation**:
```
Handley et al. (2025). "Democratizing Bioinformatics with ARM SIMD:
Systematic Validation and Production Implementation." GigaScience.
```

---

## Contact and Contribution

**Repository**: github.com/shandley/biofast

**Issues**: github.com/shandley/biofast/issues

**Contributing**: See CONTRIBUTING.md

**Maintainers**:
- Scott Handley (primary)
- Community contributors (welcome!)

---

**Last Updated**: November 3, 2025
**Status**: Design Phase → Implementation Week 2
**First Release Target**: November 15, 2025 (alpha)
**Production Release**: November 22, 2025 (1.0.0)
