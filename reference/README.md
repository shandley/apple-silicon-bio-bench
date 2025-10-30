# ASBB Reference Data

**Purpose**: Static reference tables containing factual, unchanging information used throughout ASBB analysis and experimentation.

**Format**: Tab-separated values (TSV) for easy parsing and human readability

**Status**: Reference data only - NOT modified by experimental results

---

## Directory Structure

```
reference/
├── hardware/               # Apple Silicon specifications
│   ├── apple_silicon_chips.tsv    (17 chip variants)
│   └── mac_models.tsv             (14 Mac/iPad models)
├── bioinformatics/        # Sequence data formats and encodings
│   ├── dna_encodings.tsv          (9 DNA encodings)
│   ├── quality_scores.tsv         (4 quality encodings)
│   ├── sequence_file_formats.tsv  (13 file formats)
│   ├── illumina_adapters.tsv      (24 adapter sequences)
│   ├── contaminants.tsv           (17 contaminant sequences)
│   └── kmer_parameters.tsv        (12 k-mer sizes)
├── performance/           # Algorithm complexity reference
│   └── operation_complexity.tsv   (26 operations)
└── README.md             # This file
```

---

## Hardware Reference

### `hardware/apple_silicon_chips.tsv`

**Comprehensive Apple Silicon chip specifications** (M1 through M5, all variants)

**Columns**:
- `chip`: Chip family (M1, M2, M3, M4, M5)
- `variant`: Base, Pro, Max, Ultra
- `release_date`: Year-month of announcement
- `process_node`: Manufacturing process (5nm, 3nm variants)
- `cpu_cores_total`, `cpu_p_cores`, `cpu_e_cores`: CPU core counts
- `gpu_cores_max`: Maximum GPU cores
- `neural_engine_cores`: Neural Engine core count (always 16 for base/Pro/Max, 32 for Ultra)
- `neural_engine_tops`: TOPS (trillion operations per second)
- `memory_max_gb`: Maximum unified memory
- `memory_bandwidth_gbps`: Memory bandwidth (GB/s)
- `transistors_billion`: Transistor count
- `special_features`: Key hardware features
- `notes`: Additional context

**Chips covered**:
- M1: Base, Pro, Max, Ultra
- M2: Base, Pro, Max, Ultra
- M3: Base, Pro, Max, Ultra
- M4: Base, Pro, Max
- M5: Base (M5 Pro/Max/Ultra not yet released as of October 2025)

**Key for ASBB**:
- Memory bandwidth determines memory-bound operation thresholds
- Neural Engine TOPS indicates ML inference capability
- M5 introduces GPU Neural Accelerators (4× AI performance)

---

### `hardware/mac_models.tsv`

**Mac and iPad models with Apple Silicon**

**Columns**:
- `model`: Product line (MacBook Air, MacBook Pro, Mac mini, etc.)
- `screen_size`: Display size (laptops/tablets only)
- `form_factor`: Laptop, Desktop, All-in-One, Tablet
- `chips_available`: Which chips are available for this model
- `memory_min_gb`, `memory_max_gb`: RAM ranges
- `storage_min_gb`, `storage_max_gb`: Storage ranges
- `ports`: Connectivity (Thunderbolt, USB, etc.)
- `release_date`: Initial release and chip updates
- `current_availability`: Current, Discontinued
- `notes`: Key features

**Models covered**:
- MacBook Air (13", 15")
- MacBook Pro (13", 14", 16")
- Mac mini
- Mac Studio
- Mac Pro
- iMac
- iPad Pro (11", 13")
- iPad Air (11", 13")

**Key for ASBB**:
- Identifies which hardware is available for testing
- Form factor affects thermal characteristics (fanless vs active cooling)
- Memory/storage limits determine dataset sizes

---

## Bioinformatics Reference

### `bioinformatics/dna_encodings.tsv`

**DNA sequence encoding schemes**

**Columns**:
- `encoding`: Encoding name
- `base_A`, `base_C`, `base_G`, `base_T`, `base_N`, `base_U`: Bit/byte representations
- `bytes_per_base`: Storage efficiency
- `bits_per_base`: Minimum bits required
- `description`: Encoding description
- `simd_friendly`: Whether encoding is vectorization-friendly
- `notes`: Implementation notes

**Encodings covered**:
- ASCII: Standard text representation (1 byte/base)
- 2-bit: Compact binary (0.25 bytes/base)
- 2-bit-extended: 2-bit with N-mask
- 4-bit: IUPAC ambiguity codes as bit flags
- IUPAC-nibble: Sequential IUPAC encoding
- UTF-8: Unicode (generally same as ASCII for DNA)
- Base-index: Lookup table indices
- Hashed-kmer: Integer k-mer hashes

**Key for ASBB**:
- 2-bit encoding: 4× memory reduction, better cache locality
- SIMD-friendly encodings enable NEON vectorization
- Encoding choice affects memory bandwidth utilization

---

### `bioinformatics/quality_scores.tsv`

**FASTQ quality score encoding schemes**

**Columns**:
- `encoding`: Encoding name
- `offset`: ASCII offset value
- `ascii_range_start`, `ascii_range_end`: ASCII character range
- `quality_range_min`, `quality_range_max`: Numeric quality score range
- `probability_error_min`, `probability_error_max`: Error probability range
- `description`: Encoding description
- `platforms`: Sequencing platforms using this encoding
- `notes`: Historical context and conversion info

**Encodings covered**:
- Phred+33: Modern standard (Illumina 1.8+, Ion Torrent)
- Phred+64: Legacy Illumina (1.3-1.7)
- Solexa+64: Very old Illumina (1.0-1.2)
- Phred+0: Internal binary representation

**Key for ASBB**:
- Phred+33 is current standard
- Quality filtering must handle correct offset
- Conversion between encodings sometimes necessary

---

### `bioinformatics/illumina_adapters.tsv`

**Illumina adapter sequences for TruSeq and Nextera library prep**

**Columns**:
- `adapter_name`: Adapter identifier
- `library_prep`: Library prep kit (TruSeq, Nextera, Nextera XT, Small RNA, etc.)
- `read_type`: Read 1, Read 2, Universal, Index, PCR Primer, Flow cell
- `sequence`: DNA sequence (5' to 3')
- `length_bp`: Sequence length in base pairs
- `description`: Adapter description and function
- `use_case`: Primary use case (adapter trimming, demultiplexing, etc.)
- `notes`: Additional implementation details

**Adapters covered**:
- **TruSeq**: Universal, Indexed, Read 1/2, i7 index, short forms
- **Nextera**: Transposase sequence, Read 1/2, i7/i5 index, short forms
- **Nextera XT**: Read 1/2 adapters
- **Small RNA**: 3'/5' adapters
- **Multiplex**: Legacy Read 1/2 adapters
- **PCR Primers**: Primer 1/2
- **Flow cell**: P5/P7 adapters

**Key for ASBB**:
- Adapter detection experiments use these sequences
- TruSeq adapters: 13-58 bp (most common: 33-34 bp)
- Nextera transposase sequence: 19 bp (CTGTCTCTTATACACATCT)
- Short forms define minimum detectable adapter length

---

### `bioinformatics/contaminants.tsv`

**Common sequencing contaminants and quality control sequences**

**Columns**:
- `contaminant_name`: Contaminant identifier
- `type`: Virus, Bacteria, rRNA, Artifact, Plasmid, Vector
- `source`: Origin (Illumina Control, Contamination, Library prep, etc.)
- `length_bp`: Genome/sequence length
- `accession`: NCBI accession number (if applicable)
- `sequence_file`: Reference FASTA filename
- `description`: Detailed description
- `spike_in_percent`: Typical spike-in percentage (for controls)
- `use_case`: Primary use case (QC, contamination detection, filtering)
- `notes`: Additional context

**Contaminants covered**:
- **PhiX174**: Illumina sequencing control (5386 bp, 1-5% spike-in)
- **E. coli K-12**: Common lab contaminant (4.6 Mbp)
- **Human rRNA**: rRNA contamination (18S, 28S, 5.8S, 5S)
- **Mitochondrial DNA**: Organellar contamination (16.6 kbp)
- **Adapter dimers**: TruSeq and Nextera adapter-adapter products (~120 bp)
- **Primer dimers**: PCR artifacts (~60 bp)
- **Poly-A/T/G/C**: Homopolymer artifacts
- **Vectors**: Lambda, pUC19, UniVec database
- **Mycoplasma**: Cell culture contaminant
- **Yeast rRNA**: Lab contaminant

**Key for ASBB**:
- PhiX control used in 1-5% spike-in for lane balance
- Adapter detection tests use adapter dimer sequences
- Contamination screening tests use E. coli, rRNA, vectors
- Poly-G artifacts specific to Illumina 2-color chemistry (NextSeq/NovaSeq)

---

### `bioinformatics/kmer_parameters.tsv`

**K-mer size parameters and memory requirements**

**Columns**:
- `kmer_size`: K-mer size (k)
- `num_possible_kmers`: Total k-mer space (4^k)
- `bits_for_2bit`: Bits required for 2-bit encoding (2*k)
- `bytes_for_2bit`: Bytes for 2-bit encoding (ceil(2*k / 8))
- `bytes_for_u64`: Bytes for u64 integer storage (8)
- `hash_collisions_likely`: Whether hash collisions are expected
- `memory_for_table_mb`: Memory for full k-mer hash table (MB)
- `typical_use_case`: Primary application
- `tools_using`: Common tools using this k-mer size
- `notes`: Memory/implementation considerations

**K-mer sizes covered**:
- **k=3-7**: Small k-mer space, lookup table friendly
- **k=11-13**: Adapter detection, contamination screening (Kraken, Trimmomatic)
- **k=15-17**: Short read assembly (SPAdes, ABySS, requires 8-128 GB RAM)
- **k=19-25**: Long read assembly (Canu, Flye, requires probabilistic structures)
- **k=31**: Genome comparison (Mash, Sourmash, MinHash/sketching)
- **k=51**: Unique sequence identification (requires 128-bit integers)

**Key for ASBB**:
- k=13 is optimal for adapter detection (full adapter coverage)
- k≤15 fits in RAM (≤8 GB), k≥17 requires probabilistic structures
- K-mer extraction time complexity: O(n*k)
- K-mer space grows exponentially: 4^k (k=21 → 4.4 trillion k-mers)

---

### `bioinformatics/sequence_file_formats.tsv`

**Sequence file formats and characteristics**

**Columns**:
- `format`: Format name
- `extension`: Common file extensions
- `structure`: File structure description
- `quality_scores`: Whether format includes quality scores
- `compression_common`: Typical compression methods
- `line_length`: Line wrapping conventions
- `indexed`: Whether format supports indexing
- `streamable`: Whether format supports streaming I/O
- `description`: Format description
- `use_cases`: Typical applications
- `typical_size_range`: Expected file sizes

**Formats covered**:
- FASTA: Simple sequences
- FASTQ: Sequences with quality scores
- SAM/BAM/CRAM: Alignment formats
- GFF3/GTF: Annotations
- BED: Genomic intervals
- VCF/BCF: Variants
- Compressed variants (.gz, .bam, etc.)

**Key for ASBB**:
- Streaming support determines I/O strategy
- Compression affects decompression performance (hardware acceleration on M5)
- Typical sizes inform dataset generation

---

## Performance Reference

### `performance/operation_complexity.tsv`

**Algorithm complexity and performance characteristics**

**Columns**:
- `operation`: Operation name
- `category`: Operation category (Element-wise, Filter, Search, etc.)
- `time_complexity`: Big-O time complexity
- `space_complexity`: Big-O space complexity
- `vectorizable`: Degree of SIMD vectorization potential (Yes/No/Partial)
- `parallelizable`: Thread parallelization potential
- `memory_bound`: Whether operation is limited by memory bandwidth
- `compute_bound`: Whether operation is limited by CPU/GPU compute
- `typical_bottleneck`: Primary performance limitation
- `notes`: Implementation details and optimization opportunities

**Operations covered**:
- Element-wise: base_counting, gc_content, reverse_complement, quality_mean, etc.
- Filter: quality_filter, length_filter, complexity_filter
- Search: kmer_extraction, kmer_counting, kmer_exact_match, kmer_fuzzy_match
- Pairwise: hamming_distance, edit_distance, deduplication, clustering
- Aggregation: statistics, histogram, minhash_sketch, random_sampling
- I/O: parsing, decompression, format_conversion, writing

**Key for ASBB**:
- Time complexity informs scaling experiments (how performance changes with data size)
- Vectorizable operations are prime candidates for NEON optimization
- Memory-bound operations benefit from increased bandwidth (M5: 153 GB/s)
- Compute-bound operations benefit from more cores or GPU

---

## Usage Guidelines

### For Experiments

1. **Hardware Configuration**: Reference `apple_silicon_chips.tsv` to set correct memory bandwidth, core counts, etc.
2. **Data Generation**: Use `dna_encodings.tsv` and `quality_scores.tsv` for correct sequence/quality encoding
3. **File I/O**: Reference `sequence_file_formats.tsv` for format-specific handling
4. **Operation Design**: Check `operation_complexity.tsv` to understand expected complexity and bottlenecks

### For Analysis

1. **Cross-generation comparison**: Compare experimental results against chip specifications
2. **Encoding validation**: Ensure data generation matches reference encodings
3. **Complexity validation**: Verify measured complexity matches theoretical (if not, investigate!)
4. **Hardware detection**: Auto-detect chip generation and capabilities

### Data Integrity

**These files are REFERENCE DATA**:
- ✅ READ by experiments and analysis
- ❌ NEVER MODIFIED by experimental results
- ✅ Updated only for corrections or new hardware releases
- ✅ Version-controlled in git

**If you need to add experimental results**, create separate files in `results/` or `analysis/` directories.

---

## Maintenance

### When to Update

**Hardware files** (`hardware/`):
- New Apple Silicon chips announced (e.g., M5 Pro/Max/Ultra)
- New Mac models released
- Specification corrections from Apple

**Bioinformatics files** (`bioinformatics/`):
- New encoding schemes emerge
- New file format standards adopted
- Corrections to existing standards

**Performance files** (`performance/`):
- New operation categories added to ASBB
- Complexity analysis corrections
- Additional operations implemented

### Update Process

1. Research accurate specifications (Apple official sources preferred)
2. Update TSV file maintaining column structure
3. Test parsability (ensure tab-separated, no extra whitespace)
4. Git commit with clear message: "ref: Update M5 specifications"
5. Update this README if columns change

---

## File Format Notes

**TSV Format**:
- Tab-separated values (\\t delimiter)
- First row: column headers
- UTF-8 encoding
- No quotes around fields (except if containing tabs)
- Empty fields allowed (use empty string, not "N/A" unless semantic)

**Parsing**:
```rust
// Example Rust parsing
use std::fs::File;
use std::io::{BufRead, BufReader};

let file = File::open("reference/hardware/apple_silicon_chips.tsv")?;
let reader = BufReader::new(file);
for line in reader.lines().skip(1) { // Skip header
    let line = line?;
    let fields: Vec<&str> = line.split('\t').collect();
    // fields[0] = chip, fields[1] = variant, etc.
}
```

**Viewing**:
```bash
# View file in terminal
column -t -s $'\t' reference/hardware/apple_silicon_chips.tsv | less -S

# Open in spreadsheet app
open reference/hardware/apple_silicon_chips.tsv
```

---

## References

**Hardware specifications**:
- Apple Newsroom: Official chip announcements
- Apple Tech Specs: Product specification pages
- MacRumors, 9to5Mac: Technical analysis
- Wikipedia: Apple silicon articles

**Bioinformatics standards**:
- FASTQ format: Cock et al. (2010) Nucleic Acids Research
- SAM/BAM specification: https://samtools.github.io/hts-specs/
- VCF specification: https://samtools.github.io/hts-specs/VCFv4.3.pdf
- IUPAC codes: International Union of Pure and Applied Chemistry

**Algorithm complexity**:
- Biological Sequence Analysis (Durbin et al., 1998)
- Bioinformatics Algorithms (Compeau & Pevzner, 2015)
- Published bioinformatics tool papers (cited in ASBB methodology)

---

**Status**: Reference data complete for M1-M5 (base variants)
**Last Updated**: October 30, 2025
**Maintainer**: ASBB Project
