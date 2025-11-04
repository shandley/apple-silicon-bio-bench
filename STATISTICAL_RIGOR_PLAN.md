# Statistical Rigor Implementation Plan - DAG Framework

**Date**: November 3, 2025
**Goal**: Achieve publication-quality statistical rigor for all DAG experiments
**Timeline**: 3-5 days (no rush - get it right)

---

## Executive Summary

**Current status**: 307 experiments with single measurements (exploratory analysis)

**Target**: ~9,000+ measurements with full statistical rigor (publication-ready)

**Approach**:
1. Fix harness (add repetitions, warmup, statistics)
2. Expand datasets (add VeryLarge 1M, Huge 10M)
3. Re-run all batches (N=30 repetitions each)
4. Statistical analysis (CI, significance tests, effect sizes)
5. I/O overhead characterization
6. Documentation & validation

---

## PHASE 1: Fix DAG Harness for Statistical Rigor

**Duration**: 4-6 hours
**Complexity**: Medium (architectural changes to harness)

### Task 1.1: Add Repetitions Parameter

**File**: `crates/asbb-cli/src/dag_traversal.rs`

**Changes needed**:
```rust
pub struct DAGConfig {
    pub operations: Vec<String>,
    pub scales: Vec<Scale>,
    pub repetitions: usize,          // NEW: default 30
    pub warmup_runs: usize,          // NEW: default 3
    pub outlier_threshold: f64,      // NEW: IQR multiplier (1.5)
}

struct ExperimentMeasurements {
    measurements: Vec<f64>,          // All repetition timings
    warmup_measurements: Vec<f64>,   // Discarded warmup timings
    outliers_removed: Vec<f64>,      // Removed outliers
}

struct ExperimentStatistics {
    mean: f64,
    median: f64,
    std_dev: f64,
    min: f64,
    max: f64,
    q1: f64,
    q3: f64,
    iqr: f64,
    ci_95_lower: f64,
    ci_95_upper: f64,
    n_valid: usize,                  // After outlier removal
    n_outliers: usize,
}
```

**Implementation approach**:
1. Modify `run_experiment_impl()` to loop N times
2. Store all measurements
3. Run warmup iterations first (discard)
4. Detect and remove outliers (IQR method)
5. Calculate statistics from remaining measurements

**Testing**: Run single operation with N=5 to validate

---

### Task 1.2: Add Warmup Runs

**Purpose**: Eliminate cold-start effects
- CPU frequency scaling
- Cache cold start
- OS scheduler learning

**Implementation**:
```rust
fn run_experiment_impl(...) -> Result<ExperimentResult> {
    let mut warmup_times = Vec::new();
    let mut measurements = Vec::new();

    // Warmup phase
    for _ in 0..config.warmup_runs {
        let start = Instant::now();
        let _output = execute_operation(...)?;
        warmup_times.push(start.elapsed().as_secs_f64());
    }

    // Actual measurements
    for _ in 0..config.repetitions {
        let start = Instant::now();
        let _output = execute_operation(...)?;
        measurements.push(start.elapsed().as_secs_f64());
    }

    // Statistical processing
    let stats = calculate_statistics(&measurements, config.outlier_threshold)?;

    // Use median for throughput (more robust than mean)
    let throughput = scale.num_sequences as f64 / stats.median;

    ...
}
```

**Rationale**: Median is robust to outliers, mean is traditional but sensitive

---

### Task 1.3: Fix Timer Precision

**Problem**: Operations < 100 µs round to 0.000000 seconds

**Solution**: Batch iterations for sub-millisecond operations

```rust
fn run_experiment_impl(...) -> Result<ExperimentResult> {
    // Estimate single run time
    let start = Instant::now();
    let _output = execute_operation(...)?;
    let single_run_time = start.elapsed().as_secs_f64();

    // If < 1ms, batch to achieve ~10-100ms total
    let batch_size = if single_run_time < 0.001 {
        ((0.01 / single_run_time).ceil() as usize).max(10)
    } else {
        1
    };

    // Run with batching
    for _ in 0..config.repetitions {
        let start = Instant::now();
        for _ in 0..batch_size {
            let _output = execute_operation(...)?;
        }
        let elapsed = start.elapsed().as_secs_f64();
        measurements.push(elapsed / batch_size as f64);  // Average per iteration
    }

    ...
}
```

**Alternative**: Use `std::time::Instant` with nanosecond precision (already available)

**Testing**: Run Tiny scale operations, verify non-zero timings

---

### Task 1.4: Add Outlier Detection

**Method**: IQR (Interquartile Range) - standard statistical method

```rust
fn remove_outliers(measurements: &[f64], threshold: f64) -> (Vec<f64>, Vec<f64>) {
    let mut sorted = measurements.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());

    let n = sorted.len();
    let q1 = sorted[n / 4];
    let q3 = sorted[3 * n / 4];
    let iqr = q3 - q1;

    let lower_bound = q1 - threshold * iqr;
    let upper_bound = q3 + threshold * iqr;

    let valid: Vec<f64> = sorted.iter()
        .copied()
        .filter(|&x| x >= lower_bound && x <= upper_bound)
        .collect();

    let outliers: Vec<f64> = sorted.iter()
        .copied()
        .filter(|&x| x < lower_bound || x > upper_bound)
        .collect();

    (valid, outliers)
}
```

**Threshold**: 1.5× IQR (standard), or 3.0× IQR (conservative)

**Reporting**: Document # outliers removed per experiment

---

### Task 1.5: Calculate Statistics

**Standard metrics**:

```rust
fn calculate_statistics(measurements: &[f64], outlier_threshold: f64)
    -> Result<ExperimentStatistics>
{
    let (valid, outliers) = remove_outliers(measurements, outlier_threshold);

    if valid.len() < 5 {
        anyhow::bail!("Too many outliers removed: {} / {}",
                      outliers.len(), measurements.len());
    }

    let n = valid.len() as f64;
    let mean = valid.iter().sum::<f64>() / n;

    let variance = valid.iter()
        .map(|x| (x - mean).powi(2))
        .sum::<f64>() / (n - 1.0);
    let std_dev = variance.sqrt();

    // Median
    let mut sorted = valid.clone();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let median = if sorted.len() % 2 == 0 {
        (sorted[sorted.len() / 2 - 1] + sorted[sorted.len() / 2]) / 2.0
    } else {
        sorted[sorted.len() / 2]
    };

    // Quartiles
    let q1 = sorted[sorted.len() / 4];
    let q3 = sorted[3 * sorted.len() / 4];
    let iqr = q3 - q1;

    // 95% Confidence Interval (t-distribution)
    let t_critical = t_critical_value(n as usize - 1, 0.05);  // 95% CI
    let margin_of_error = t_critical * (std_dev / n.sqrt());
    let ci_95_lower = mean - margin_of_error;
    let ci_95_upper = mean + margin_of_error;

    Ok(ExperimentStatistics {
        mean,
        median,
        std_dev,
        min: *sorted.first().unwrap(),
        max: *sorted.last().unwrap(),
        q1,
        q3,
        iqr,
        ci_95_lower,
        ci_95_upper,
        n_valid: valid.len(),
        n_outliers: outliers.len(),
    })
}

// Simple t-critical value lookup (use statrs crate for production)
fn t_critical_value(df: usize, alpha: f64) -> f64 {
    // For 95% CI (alpha=0.05), df=29 (N=30): t ≈ 2.045
    // For df > 30: approximately 1.96 (normal distribution)
    match (df, alpha) {
        (_, 0.05) if df >= 30 => 1.96,
        (29, 0.05) => 2.045,
        (19, 0.05) => 2.093,
        (9, 0.05) => 2.262,
        (4, 0.05) => 2.776,
        _ => 2.0,  // Conservative default
    }
}
```

**Output CSV format** (expanded):
```csv
operation,config_name,scale,num_sequences,
median_throughput,mean_throughput,std_dev_throughput,
ci_95_lower,ci_95_upper,
median_speedup,mean_speedup,std_dev_speedup,
q1,q3,iqr,
n_valid,n_outliers,
elapsed_median,elapsed_mean,elapsed_std_dev
```

---

### Task 1.6: Update CSV Output

**Enhanced output** with statistics:

```rust
fn write_results_to_csv(results: &[ExperimentResult], path: &str) -> Result<()> {
    let mut wtr = Writer::from_path(path)?;

    // Enhanced header
    wtr.write_record(&[
        "operation", "config_name", "config_type", "threads", "affinity",
        "scale", "num_sequences",
        "median_throughput", "mean_throughput", "std_dev_throughput",
        "ci_95_lower_throughput", "ci_95_upper_throughput",
        "median_speedup", "mean_speedup", "std_dev_speedup",
        "ci_95_lower_speedup", "ci_95_upper_speedup",
        "q1_elapsed", "q3_elapsed", "iqr_elapsed",
        "min_elapsed", "max_elapsed",
        "n_valid", "n_outliers", "pruned"
    ])?;

    for result in results {
        wtr.write_record(&[
            &result.operation,
            &result.config_name,
            &format!("{:?}", result.config_type),
            &result.threads.to_string(),
            &result.affinity,
            &result.scale,
            &result.num_sequences.to_string(),
            &result.stats.median_throughput.to_string(),
            &result.stats.mean_throughput.to_string(),
            &result.stats.std_dev_throughput.to_string(),
            // ... (all statistics fields)
        ])?;
    }

    wtr.flush()?;
    Ok(())
}
```

---

## PHASE 2: Expand Dataset Scales

**Duration**: 1-2 hours
**Complexity**: Low (dataset generation)

### Task 2.1: Generate VeryLarge Dataset (1M sequences)

```bash
# Generate 1M sequences × 150 bp = 150 MB
python scripts/generate_fastq.py \
  --num-sequences 1000000 \
  --seq-length 150 \
  --output datasets/verylarge_1m_150bp.fq \
  --seed 42
```

**Purpose**: Test behavior closer to production scale

---

### Task 2.2: Generate Huge Dataset (10M sequences)

```bash
# Generate 10M sequences × 150 bp = 1.5 GB
python scripts/generate_fastq.py \
  --num-sequences 10000000 \
  --seq-length 150 \
  --output datasets/huge_10m_150bp.fq \
  --seed 42
```

**Purpose**: Test memory pressure, cache effects at GB scale

**Consideration**: 1.5 GB dataset on 24 GB RAM (safe, ~6% of total)

---

### Task 2.3: Update Scale Definitions

```rust
// crates/asbb-cli/src/dag_traversal.rs
pub fn default_scales() -> Vec<Scale> {
    vec![
        Scale {
            name: "Tiny".to_string(),
            path: "datasets/tiny_100_150bp.fq".to_string(),
            num_sequences: 100,
        },
        Scale {
            name: "Small".to_string(),
            path: "datasets/small_1k_150bp.fq".to_string(),
            num_sequences: 1_000,
        },
        Scale {
            name: "Medium".to_string(),
            path: "datasets/medium_10k_150bp.fq".to_string(),
            num_sequences: 10_000,
        },
        Scale {
            name: "Large".to_string(),
            path: "datasets/large_100k_150bp.fq".to_string(),
            num_sequences: 100_000,
        },
        Scale {
            name: "VeryLarge".to_string(),
            path: "datasets/verylarge_1m_150bp.fq".to_string(),
            num_sequences: 1_000_000,
        },
        Scale {
            name: "Huge".to_string(),
            path: "datasets/huge_10m_150bp.fq".to_string(),
            num_sequences: 10_000_000,
        },
    ]
}
```

**Coverage**: 100 → 10,000,000 (5 orders of magnitude!)

---

## PHASE 3: Re-run All 3 Batches with Rigor

**Duration**: 4-8 hours (depending on compute time)
**Complexity**: Low (execute with new harness)

### Expected Runtime Estimates

**With N=30 repetitions + 3 warmup**:
- Each experiment: 33 runs (3 warmup + 30 measured)
- Batch 1: 87 experiments × 33 runs = 2,871 runs
- Batch 2: 60 experiments × 33 runs = 1,980 runs
- Batch 3 (now 6 scales): 10 ops × 4 configs × 6 scales = 240 experiments × 33 runs = 7,920 runs
- **Total**: ~12,800 individual runs

**Estimated time**:
- Previous: 3.79 seconds total for 307 experiments (single runs)
- Now: 3.79 sec × 33 (repetitions) × 2 (batching for precision) = ~250 seconds = **~4 minutes**

**With 2 new scales (VeryLarge, Huge)**:
- VeryLarge (1M seqs): ~10× slower than Large (100K)
- Huge (10M seqs): ~100× slower than Large
- Estimated additional time: 30-60 minutes

**Total estimated runtime**: 30-90 minutes for all batches

---

### Task 3.1: Batch 1 Re-run (NEON+Parallel)

```bash
cargo run --release -p asbb-cli --bin asbb-dag-traversal -- \
  --batch neon_parallel \
  --repetitions 30 \
  --warmup 3 \
  --output results/dag_rigorous/dag_neon_parallel_rigorous.csv
```

**Expected output**:
- 87 experiments × 6 scales = ~130 experiments (with pruning)
- Each with mean ± std dev, 95% CI
- Total measurements: ~130 × 30 = 3,900 runs

---

### Task 3.2: Batch 2 Re-run (Core Affinity)

```bash
cargo run --release -p asbb-cli --bin asbb-dag-traversal -- \
  --batch core_affinity \
  --repetitions 30 \
  --warmup 3 \
  --output results/dag_rigorous/dag_core_affinity_rigorous.csv
```

**Expected output**:
- 60 experiments × 6 scales = 360 experiments (no pruning)
- Total measurements: 360 × 30 = 10,800 runs

---

### Task 3.3: Batch 3 Re-run (Scale Thresholds)

```bash
cargo run --release -p asbb-cli --bin asbb-dag-traversal -- \
  --batch scale_thresholds \
  --repetitions 30 \
  --warmup 3 \
  --output results/dag_rigorous/dag_scale_thresholds_rigorous.csv
```

**Expected output**:
- 10 ops × 4 configs × 6 scales = 240 experiments
- Total measurements: 240 × 30 = 7,200 runs

---

## PHASE 4: Statistical Analysis

**Duration**: 3-4 hours
**Complexity**: Medium (statistical programming)

### Task 4.1: Confidence Intervals

**Already calculated** in harness (95% CI using t-distribution)

**Reporting**: Include in all summaries and plots

---

### Task 4.2: Statistical Significance Testing

**Purpose**: Determine if observed differences are statistically significant

**Method**: Welch's t-test (unequal variances)

```python
# analysis/statistical_significance.py
import pandas as pd
from scipy import stats

def test_significance(group1, group2, alpha=0.05):
    """
    Welch's t-test for two groups with unequal variances

    Returns:
        t_statistic: test statistic
        p_value: probability of null hypothesis
        significant: True if p < alpha
        effect_size: Cohen's d
    """
    t_stat, p_value = stats.ttest_ind(group1, group2, equal_var=False)

    # Cohen's d effect size
    pooled_std = np.sqrt((np.var(group1) + np.var(group2)) / 2)
    cohens_d = (np.mean(group1) - np.mean(group2)) / pooled_std

    return {
        't_statistic': t_stat,
        'p_value': p_value,
        'significant': p_value < alpha,
        'effect_size': cohens_d,
        'effect_interpretation': interpret_effect_size(cohens_d)
    }

def interpret_effect_size(d):
    """Cohen's d interpretation"""
    abs_d = abs(d)
    if abs_d < 0.2:
        return "negligible"
    elif abs_d < 0.5:
        return "small"
    elif abs_d < 0.8:
        return "medium"
    else:
        return "large"
```

**Key comparisons to test**:
1. NEON vs naive (all operations)
2. NEON+4t vs NEON (composition)
3. P-cores vs E-cores vs default (core affinity)
4. Tiny vs Small vs Medium scales (cache effects)

---

### Task 4.3: Effect Size Calculation

**Purpose**: Quantify magnitude of differences (not just significance)

**Method**: Cohen's d

**Interpretation**:
- d = 0.2: Small effect
- d = 0.5: Medium effect
- d = 0.8: Large effect
- d > 1.2: Very large effect

**Example finding**:
> "NEON provides 23× speedup for base_counting at Tiny scale (t=45.2, p<0.001, Cohen's d=3.8, very large effect)"

---

### Task 4.4: Publication-Quality Plots

**Tool**: Python (matplotlib/seaborn)

**Plots needed**:

1. **Speedup vs Scale** (with error bars)
```python
# Mean speedup with 95% CI error bars
plt.errorbar(scales, mean_speedups, yerr=ci_95_errors,
             fmt='o-', capsize=5, label='NEON')
plt.xlabel('Dataset Scale (log)')
plt.ylabel('Speedup vs Naive')
plt.title('NEON Speedup Across Scales (N=30)')
```

2. **Box plots** (show distribution)
```python
# Box plot showing median, quartiles, outliers
sns.boxplot(x='scale', y='speedup', hue='config', data=df)
```

3. **Violin plots** (show full distribution)
```python
sns.violinplot(x='operation', y='speedup', hue='config', data=df)
```

**All plots must have**:
- Error bars (95% CI)
- N= annotations
- Statistical significance markers (* p<0.05, ** p<0.01, *** p<0.001)

---

## PHASE 5: I/O Overhead Characterization

**Duration**: 4-6 hours
**Complexity**: Medium (new harness)

### Task 5.1: End-to-End Pipeline

**New binary**: `asbb-dag-traversal-e2e`

**Implementation**:
```rust
fn run_end_to_end_experiment(
    operation: &str,
    node: &DAGNode,
    dataset_path: &str,
) -> Result<E2ETimings> {
    let start_total = Instant::now();

    // Phase 1: FASTQ parsing (I/O)
    let start_io = Instant::now();
    let sequences = parse_fastq(dataset_path)?;
    let io_elapsed = start_io.elapsed();

    // Phase 2: Computation
    let start_compute = Instant::now();
    let result = execute_operation(operation, &sequences, node)?;
    let compute_elapsed = start_compute.elapsed();

    // Phase 3: Output writing (optional)
    let start_output = Instant::now();
    write_output(&result)?;
    let output_elapsed = start_output.elapsed();

    let total_elapsed = start_total.elapsed();

    Ok(E2ETimings {
        total: total_elapsed,
        io: io_elapsed,
        compute: compute_elapsed,
        output: output_elapsed,
        compute_fraction: compute_elapsed.as_secs_f64() / total_elapsed.as_secs_f64(),
    })
}
```

**Output**: Breakdown of time spent in each phase

---

### Task 5.2: Test Compression Formats

**Datasets needed**:
```bash
# Compress existing datasets
gzip -k datasets/large_100k_150bp.fq  # -> .fq.gz
zstd -k datasets/large_100k_150bp.fq  # -> .fq.zst
```

**Test**: Same operation on uncompressed vs gzip vs zstd

**Expected finding** (from Entry 016):
- Uncompressed: Fastest
- zstd: ~1.5-2× slower
- gzip: ~2-3× slower

---

### Task 5.3: Amdahl's Law Analysis

**Purpose**: Calculate maximum achievable speedup with I/O overhead

**Formula**:
```
Speedup_total = 1 / (f_io + (f_compute / Speedup_compute))
```

Where:
- f_io = fraction of time in I/O
- f_compute = fraction of time in compute
- Speedup_compute = speedup from NEON/parallel optimization

**Example**:
- I/O: 50% of runtime (f_io = 0.5)
- Compute: 50% of runtime (f_compute = 0.5)
- NEON speedup: 20× (Speedup_compute = 20)
- **Total speedup**: 1 / (0.5 + 0.5/20) = 1 / 0.525 = **1.90×**

**Implication**: Even with 20× compute speedup, only 1.9× end-to-end speedup if I/O is 50%

**Recommendation**: Streaming architecture to hide I/O (overlap I/O with compute)

---

## PHASE 6: Documentation & Validation

**Duration**: 4-6 hours
**Complexity**: Medium (writing + validation)

### Task 6.1: Update All Batch Summaries

**New format**:

```markdown
## Key Finding 1: NEON Peak at Tiny Scale

**Claim**: NEON provides highest speedup at smallest scales (cache effects)

**Evidence**:
- base_counting @ Tiny (100 seq): 23.07× ± 1.2× (95% CI: 21.8-24.3×, N=30)
- base_counting @ Small (1K seq): 13.64× ± 0.8× (95% CI: 12.9-14.4×, N=30)
- base_counting @ Medium (10K seq): 17.33× ± 1.0× (95% CI: 16.4-18.3×, N=30)

**Statistical significance**:
- Tiny vs Small: t=15.2, p<0.001, Cohen's d=2.8 (very large effect)
- Tiny vs Medium: t=12.8, p<0.001, Cohen's d=2.4 (very large effect)

**Conclusion**: Tiny scale speedup is significantly higher (p<0.001)
```

**All claims must have**:
- Mean ± std dev
- 95% confidence interval
- Sample size (N=)
- Statistical test results (t, p, Cohen's d)

---

### Task 6.2: Create Statistical Validation Report

**File**: `results/dag_rigorous/STATISTICAL_VALIDATION.md`

**Contents**:
1. Methodology (repetitions, warmup, outlier removal)
2. Data quality checks (outlier rates, variance)
3. Key statistical tests
4. Publication-ready claims
5. Comparison to single-measurement results (old vs new)

---

### Task 6.3: Write Methodology Section for Paper

**File**: `paper/METHODOLOGY.md`

**Structure**:
```markdown
# Methods

## Experimental Design

We conducted systematic experiments across 3 hardware dimensions...

### Hardware Configurations
- **Baseline**: Naive scalar implementation
- **NEON**: ARM NEON SIMD (128-bit vector operations)
- **Parallel**: Multi-threaded execution (2, 4 threads)
- **Composition**: NEON + Parallel

### Datasets
Six scales spanning 5 orders of magnitude...

### Statistical Rigor
Each experiment repeated 30 times with 3 warmup runs...

Outliers identified using IQR method (1.5× threshold) and removed...

Statistical significance tested using Welch's t-test (α=0.05)...

Effect sizes calculated using Cohen's d...

### Measurements
Timing measured using std::time::Instant (nanosecond precision)...

For operations <1ms, batched 10-100× iterations...

Reported metrics: median (robust to outliers), mean ± SD, 95% CI...

## Results

### Cache Effects
We observed peak NEON performance at smallest scales...
[Include plot with error bars]

Statistical tests confirm...
```

---

### Task 6.4: Archive Old Results

**Create**: `results/dag_exploratory/` directory

**Move**:
- `results/dag_complete/dag_neon_parallel.csv` → `results/dag_exploratory/`
- `results/dag_complete/dag_core_affinity.csv` → `results/dag_exploratory/`
- `results/dag_complete/dag_scale_thresholds.csv` → `results/dag_exploratory/`
- All batch summaries

**Add README**:
```markdown
# Exploratory Analysis Results (Single Measurements)

These results represent initial exploratory analysis with single measurements.

**Limitations**:
- No statistical repetitions
- No uncertainty quantification
- Not publication-ready

**Purpose**: Pattern discovery, hypothesis generation

**Superseded by**: results/dag_rigorous/ (N=30 repetitions)

Kept for reference and comparison.
```

---

## PHASE 7: Cross-Platform Validation (Optional)

**Duration**: 3-4 hours
**Complexity**: Low (reuse harness on different platform)

### Task 7.1: Run on AWS Graviton

**Platform**: c7g.xlarge instance (4 vCPUs, 8GB RAM)

**Execute**: Batch 3 (Scale Thresholds) with N=30 repetitions

**Purpose**: Validate that thresholds are platform-independent

**Expected outcome**: Similar patterns, possibly different absolute thresholds

---

## Timeline & Effort Estimates

### Phase-by-Phase Breakdown

| Phase | Duration | Tasks | Complexity |
|-------|----------|-------|------------|
| 1. Fix Harness | 4-6 hours | 6 tasks | Medium |
| 2. Expand Datasets | 1-2 hours | 3 tasks | Low |
| 3. Re-run Batches | 1-2 hours | 3 tasks | Low (mostly compute) |
| 4. Statistical Analysis | 3-4 hours | 4 tasks | Medium |
| 5. I/O Overhead | 4-6 hours | 3 tasks | Medium |
| 6. Documentation | 4-6 hours | 4 tasks | Medium |
| 7. Cross-Platform (Optional) | 3-4 hours | 2 tasks | Low |

**Total**: 20-30 hours (3-5 days at moderate pace)

**Critical path**: Phase 1 → Phase 2 → Phase 3 → Phase 4 → Phase 6

**Parallelizable**: Phase 5 (I/O) can run alongside Phase 4

---

## Deliverables (Publication-Ready)

### Data Files
- `results/dag_rigorous/dag_neon_parallel_rigorous.csv`
- `results/dag_rigorous/dag_core_affinity_rigorous.csv`
- `results/dag_rigorous/dag_scale_thresholds_rigorous.csv`
- `results/dag_rigorous/dag_io_overhead.csv`

### Analysis & Reports
- `results/dag_rigorous/BATCH1_STATISTICAL_ANALYSIS.md`
- `results/dag_rigorous/BATCH2_STATISTICAL_ANALYSIS.md`
- `results/dag_rigorous/BATCH3_STATISTICAL_ANALYSIS.md`
- `results/dag_rigorous/STATISTICAL_VALIDATION.md`

### Plots (with error bars)
- `plots/speedup_vs_scale_with_ci.png`
- `plots/cache_effects_boxplot.png`
- `plots/thread_overhead_violin.png`
- `plots/core_affinity_comparison.png`

### Lab Notebook Entries
- Entry 026: Statistical Rigor Implementation
- Entry 027: Rigorous Batch 1 Results
- Entry 028: Rigorous Batch 2 Results
- Entry 029: Rigorous Batch 3 Results
- Entry 030: I/O Overhead Characterization

### Paper Sections
- `paper/METHODOLOGY.md` (complete, rigorous)
- `paper/RESULTS.md` (with statistics, CI, significance tests)
- `paper/FIGURES.md` (publication-quality plots)

---

## Success Criteria

**Minimum requirements for publication**:
- ✅ N ≥ 30 repetitions per experiment
- ✅ Warmup runs (discard first 3)
- ✅ Outlier detection and removal
- ✅ Statistical significance testing (p-values)
- ✅ Effect size calculation (Cohen's d)
- ✅ 95% confidence intervals reported
- ✅ Publication-quality plots with error bars
- ✅ Comprehensive methodology section
- ✅ Data quality validation

**Bonus (highly desirable)**:
- ✅ Cross-platform validation (Graviton)
- ✅ I/O overhead characterization
- ✅ End-to-end performance (not just compute)
- ✅ 6 scales (5 orders of magnitude)

---

## Next Steps

**Immediate** (this session):
1. Review this plan
2. Confirm approach
3. Start Phase 1 (fix harness)

**Short-term** (Week 1 Day 3-4):
1. Complete Phases 1-2 (harness + datasets)
2. Run Phase 3 (all batches)
3. Start Phase 4 (analysis)

**Medium-term** (Week 1 Day 5 - Week 2 Day 2):
1. Complete Phases 4-6 (analysis + documentation)
2. Optional: Phase 7 (Graviton validation)

**Long-term** (Week 2 Day 3+):
1. Begin biofast implementation (with validated rules)
2. Paper writing (with rigorous methodology)

---

**Ready to proceed?** Let me know if you want to adjust any part of this plan, or we can start with Phase 1!
