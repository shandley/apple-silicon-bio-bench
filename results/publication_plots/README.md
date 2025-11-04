# Publication-Quality Validation Plots (Artifact 3)

**Created**: November 4, 2025
**Purpose**: Publication figures for manuscript preparation
**Format**: 300 DPI PNG + vector PDF (publication-ready)
**Evidence Base**: 1,357 experiments, 40,710 measurements

---

## Plot Catalog

### Plot 1: NEON Speedup by Operation
**File**: `plot1_neon_speedup_by_operation.{png,pdf}`
**Data Source**: `results/dag_statistical/batch1_neon_parallel_n30.csv`
**Evidence**: [Lab Notebook Entry 020-025](../../lab-notebook/2025-11/)

**Purpose**: Demonstrate 16-25× NEON SIMD speedup range across operations

**Key Statistics**:
- **base_counting**: 16.7× speedup (element-wise, high benefit)
- **gc_content**: 20.3× speedup (element-wise, very high benefit)
- **at_content**: 18.9× speedup (element-wise, very high benefit)
- **quality_aggregation**: 9.2× speedup (aggregation, high benefit)
- **n_content**: 5.1× speedup (medium complexity, medium benefit)
- **reverse_complement**: 1.03× speedup (transform, negligible benefit)

**Interpretation**:
- Green bars (≥10×): High NEON benefit (element-wise operations)
- Blue bars (5-10×): Medium NEON benefit (aggregation operations)
- Gray bars (<5×): Low NEON benefit (transform/metadata operations)

**Manuscript Usage**: Results section (Figure 1 or 2)

---

### Plot 2: Streaming Memory Footprint
**File**: `plot2_streaming_memory_footprint.{png,pdf}`
**Data Source**: `results/streaming/streaming_memory_v2_n30.csv`
**Evidence**: [Lab Notebook Entry 026](../../lab-notebook/2025-11/20251103-026-EXPERIMENT-streaming-memory-footprint-v2.md)

**Purpose**: Demonstrate 99.5% memory reduction via streaming architecture

**Key Statistics**:
- **Medium (10K seqs)**: 36 MB → 5.2 MB (-85.6%)
- **Large (100K seqs)**: 360 MB → 5.1 MB (-98.6%)
- **VeryLarge (1M seqs)**: 1,344 MB → 5.0 MB (**-99.5%**)

**Critical Finding**: Streaming memory is **CONSTANT (~5 MB)** regardless of dataset size

**Impact on 5TB Dataset**:
- Batch approach: 12-24 TB RAM required (impossible on consumer hardware)
- Streaming approach: <100 MB RAM required (fits on $1,400 laptop)

**Manuscript Usage**: Results section (Figure 2 or 3), Data Access pillar

---

### Plot 3: I/O Optimization Stack
**File**: `plot3_io_optimization_stack.{png,pdf}`
**Data Source**: Lab notebook entries [029-032](../../lab-notebook/2025-11/)
**Evidence**: `results/bgzip_parallel/` and `results/io_optimization/`

**Purpose**: Show layered optimization benefits (6.5× + 2.5× = 16.3×)

**Key Statistics**:

**Small Files (<50 MB)**:
- Sequential baseline: 646 MB/s
- Parallel bgzip: **3,541 MB/s (5.5× speedup)**
- mmap doesn't help: overhead dominates

**Large Files (≥50 MB)**:
- Sequential baseline: 718 MB/s
- Parallel bgzip: **4,669 MB/s (6.5× speedup)**
- + smart mmap: **15,694 MB/s (16.3× total speedup!)**

**Layered Optimization Insight**: Individual optimizations multiply when layered correctly
- Parallel bgzip: 6.5× (all platforms, portable)
- Smart mmap: 2.5× additional (macOS APFS, threshold-based)
- Combined: **16.3×** (multiplicative, not additive)

**Manuscript Usage**: Methods section (optimization approach), Results section (Figure 3 or 4)

---

### Plot 4: Block Size Impact (Streaming Overhead)
**File**: `plot4_block_size_impact.{png,pdf}`
**Data Source**: `results/streaming/streaming_overhead_n30.csv`
**Evidence**: [Lab Notebook Entry 027](../../lab-notebook/2025-11/20251103-027-EXPERIMENT-streaming-overhead.md)

**Purpose**: Demonstrate 82-86% record-by-record overhead and block-based solution

**Key Statistics**:
- **Record-by-record overhead**: 82-86% performance loss with NEON
- **Root cause**: SIMD requires batches for vectorization
- **Solution**: Process 10K sequence blocks (reduces overhead to <10%)

**Problem Identified**:
- Streaming one sequence at a time prevents NEON from vectorizing
- NEON speedup: 16-25× (batch) → 3-4× (record-by-record)
- Unacceptable 82-86% overhead

**Solution Validated**:
- Block-based streaming: Process 10K sequences per block
- Preserves 90% of NEON speedup (14-23× vs 16-25×)
- Maintains streaming memory benefits (constant ~5 MB)

**Manuscript Usage**: Methods section (streaming architecture design), Discussion section (trade-offs)

---

### Plot 5: mmap Threshold Effect
**File**: `plot5_mmap_threshold_effect.{png,pdf}`
**Data Source**: `results/io_optimization/MMAP_FINDINGS.md`
**Evidence**: [Lab Notebook Entry 032](../../lab-notebook/2025-11/20251104-032-EXPERIMENT-mmap-apfs-optimization.md)

**Purpose**: Show file-size-dependent mmap benefit (threshold at 50 MB)

**Key Statistics**:

**Small Files (<50 MB) - Don't use mmap**:
- 0.54 MB: **0.66× (34% slower!)** - overhead dominates
- 5.4 MB: **0.99× (neutral)** - break-even

**Large Files (≥50 MB) - Use mmap**:
- 54 MB: **2.30× faster** - prefetching helps
- 544 MB: **2.55× faster** - prefetching dominates

**Threshold Effect Discovery**:
- Small files: mmap overhead (page setup, syscalls) > benefit
- Large files: APFS prefetching > overhead
- **Optimal threshold**: 50 MB (validated experimentally)

**Design Decision**: Threshold-based approach
```rust
if file_size >= 50_MB {
    use_mmap_with_madvise()  // 2.3-2.5× faster
} else {
    use_standard_io()        // Avoid overhead
}
```

**Manuscript Usage**: Methods section (I/O optimization), Results section (Figure 4 or 5)

---

## Usage Guidelines

### For Manuscript Preparation

**LaTeX Figure Environment**:
```latex
\begin{figure}[htbp]
    \centering
    \includegraphics[width=0.8\textwidth]{plot1_neon_speedup_by_operation.pdf}
    \caption{NEON SIMD speedup by operation at medium scale (10K sequences, N=30 repetitions). Green bars indicate high benefit (≥10×), blue bars indicate medium benefit (5-10×), and gray bars indicate low benefit (<5×). Element-wise operations (base\_counting, gc\_content, at\_content) show highest speedup (16-20×).}
    \label{fig:neon_speedup}
\end{figure}
```

**Caption Guidelines**:
- Start with one-sentence description
- Include key statistics (N=30, scales, effect sizes)
- Explain color coding or visual elements
- Highlight main finding
- Keep under 100 words

### For Presentations

**PowerPoint/Keynote**:
- Use **PNG files** (300 DPI ensures crisp display)
- Resize to fit slide (high resolution preserves quality)
- Add slide titles matching plot titles
- Consider enlarging font sizes for visibility

**Poster Sessions**:
- Use **PDF files** for vector graphics (infinite zoom)
- Print at high resolution (300+ DPI)
- Ensure plots are readable from 3-5 feet away

### For Supplementary Materials

**Online Supplements**:
- Include both PNG (preview) and PDF (download)
- Provide data sources (CSV files linked)
- Include generation script (`analysis/generate_publication_plots.py`)

---

## Regeneration Instructions

To regenerate plots (e.g., after data updates or style changes):

```bash
# From repository root
python3 analysis/generate_publication_plots.py

# Output location
ls -lh results/publication_plots/
```

**Requirements**:
```bash
pip install pandas matplotlib seaborn numpy
```

**Customization**: Edit `analysis/generate_publication_plots.py`
- Colors: Modify `COLORS` dictionary
- Fonts: Update `plt.rcParams` at top of script
- DPI: Change `'figure.dpi'` and `'savefig.dpi'`
- Layout: Adjust figure sizes in each `plt.subplots()` call

---

## Plot Specifications

**Format**: PNG (raster) + PDF (vector)
**Resolution**: 300 DPI (publication quality)
**Color Space**: RGB
**Font Family**: Arial/Helvetica (sans-serif)
**Font Sizes**:
- Title: 14-16 pt
- Axis labels: 12 pt
- Tick labels: 10 pt
- Legend: 10 pt
- Annotations: 9-11 pt

**Dimensions**:
- Single column: 10×6 inches (fits journal single column)
- Two-panel: 14×6 inches (fits journal double column)

**File Sizes**:
- PNG: 200-500 KB (compressed)
- PDF: 50-150 KB (vector, smaller)

---

## Citation Information

**Data Source**: Apple Silicon Bio Bench (ASBB)
**Evidence Base**: 1,357 experiments, 40,710 measurements (N=30)
**Period**: October 30 - November 4, 2025
**Lab Notebook**: 33 entries documenting all experimental work
**Repository**: https://github.com/shandley/apple-silicon-bio-bench

**Data Availability**: All CSVs publicly available in `results/` directory

**Recommended Citation** (for manuscripts):
> "All plots were generated from experimental data (1,357 experiments, N=30 repetitions) available at https://github.com/shandley/apple-silicon-bio-bench. Plot generation script: `analysis/generate_publication_plots.py`."

---

## Version History

**v1.0** (November 4, 2025): Initial creation
- Plot 1: NEON Speedup by Operation
- Plot 2: Streaming Memory Footprint
- Plot 3: I/O Optimization Stack
- Plot 4: Block Size Impact (Streaming Overhead)
- Plot 5: mmap Threshold Effect
- Format: 300 DPI PNG + vector PDF

---

**Document Status**: ✅ Complete (Artifact 3 of 4)
**Next Artifact**: PUBLICATION_SUMMARY.md (Artifact 4)
**Companion Documents**:
- Artifact 1: OPTIMIZATION_RULES.md (complete)
- Artifact 2: EXPERIMENTAL_SUMMARY.md (complete)
- Artifact 4: PUBLICATION_SUMMARY.md (pending)

**Owner**: Scott Handley
**Project**: Apple Silicon Bio Bench (ASBB)
**Purpose**: Democratizing Bioinformatics Compute
