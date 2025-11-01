# Level 1/2 Automated Harness: Design Document

**Date**: November 1, 2025
**Status**: Design Phase
**Goal**: Automated testing framework for ~3,000 experiments across 20 operations

---

## Executive Summary

**Objective**: Build automated harness to validate Phase 1 optimization rules across expanded operation set (20 operations) with full hardware configuration matrix.

**Scope**:
- 20 operations (10 current + 10 new)
- 25 hardware configurations
- 6 data scales (100 → 10M sequences)
- **Total**: ~3,000 experiments

**Timeline**: 3 weeks
- Week 1: Harness implementation + operation expansion
- Week 2-3: Execution + analysis

**Expected runtime**: 1-2 days automated execution (parallelized)

---

## Current State (Phase 1)

### What We Have

**Operations Implemented** (10):
1. base_counting (0.40 complexity)
2. gc_content (0.315 complexity)
3. at_content (0.35 complexity)
4. n_content (0.25 complexity)
5. sequence_length (0.20 complexity)
6. reverse_complement (0.45 complexity)
7. quality_aggregation (0.50 complexity)
8. quality_filter (0.55 complexity)
9. length_filter (0.55 complexity)
10. complexity_score (0.61 complexity)

**Backends Implemented**:
- Naive (scalar)
- NEON SIMD
- Parallel (Rayon, 1/2/4/8 threads)
- GPU Metal (7 kernels for various operations)
- 2-bit encoding (4 operations)
- QoS-based thread assignment (P-cores, E-cores, default)

**Infrastructure**:
- `asbb-core`: Core types (SequenceRecord, DataCharacteristics, etc.)
- `asbb-ops`: Operation implementations
- `asbb-gpu`: Metal backend
- `asbb-cli`: Pilot programs (manual execution)

**Pilot Programs** (manual):
- `pilot_scales.rs` - Multi-scale NEON testing
- `pilot_2bit.rs` - Encoding comparison
- `pilot_gpu*.rs` - GPU dimension testing
- `pilot_parallel.rs` - Threading dimension testing

**Results Storage** (Phase 1):
- CSV files (one per pilot)
- Manual analysis in Markdown
- ~824 experiments total

### What We Need

**Automation**:
- Experiment configuration (declarative)
- Automated execution (parallel)
- Progress tracking and checkpointing
- Error recovery
- Structured result storage (Parquet)

**Operations**:
- 10 additional operations (diverse categories)
- Coverage of all operation types (element-wise, search, pairwise, aggregation, I/O)

**Analysis**:
- Statistical modeling (automated)
- Cross-validation framework
- Decision tree extraction
- Rule refinement

---

## Architecture Design

### Component Overview

```
┌─────────────────────────────────────────────────────────────┐
│                  Level 1/2 Automated Harness                 │
├─────────────────────────────────────────────────────────────┤
│                                                               │
│  ┌──────────────┐   ┌──────────────┐   ┌──────────────┐    │
│  │  Experiment  │──>│  Execution   │──>│   Result     │    │
│  │    Config    │   │    Engine    │   │   Storage    │    │
│  │  (TOML/JSON) │   │   (Rayon)    │   │  (Parquet)   │    │
│  └──────────────┘   └──────────────┘   └──────────────┘    │
│         │                   │                   │            │
│         v                   v                   v            │
│  ┌──────────────┐   ┌──────────────┐   ┌──────────────┐    │
│  │  Operation   │   │   Progress   │   │  Statistical │    │
│  │   Registry   │   │   Tracker    │   │   Analysis   │    │
│  │  (20 ops)    │   │  (Checkpoint)│   │ (Regression) │    │
│  └──────────────┘   └──────────────┘   └──────────────┘    │
│                                                               │
└─────────────────────────────────────────────────────────────┘
```

### Core Components

#### 1. Experiment Configuration (`experiments/level1_primitives/config.toml`)

**Purpose**: Declarative experiment specification

**Format**: TOML (human-readable, version-controllable)

**Structure**:
```toml
[metadata]
name = "Level 1 Primitives"
description = "Systematic testing across 20 operations × 25 configs × 6 scales"
version = "1.0"
total_experiments = 3000

[datasets]
scales = [
    { name = "tiny", sequences = 100 },
    { name = "small", sequences = 1000 },
    { name = "medium", sequences = 10000 },
    { name = "large", sequences = 100000 },
    { name = "vlarge", sequences = 1000000 },
    { name = "huge", sequences = 10000000 }
]
sequence_length = 150
format = "FASTQ"

[operations]
# All 20 operations listed with metadata
[[operations.list]]
name = "base_counting"
category = "element_wise"
complexity = 0.40
has_neon = true
has_gpu = true
has_2bit = true

[[operations.list]]
name = "kmer_extract"  # New operation
category = "search"
complexity = 0.48
has_neon = true
has_gpu = false
has_2bit = false

# ... (all 20 operations)

[hardware_configs]
# 25 configurations defined
[[hardware_configs.list]]
name = "baseline"
backend = "naive"
threads = 1
encoding = "ascii"
qos = "default"

[[hardware_configs.list]]
name = "neon_1t"
backend = "neon"
threads = 1
encoding = "ascii"
qos = "default"

[[hardware_configs.list]]
name = "neon_parallel_8t_pcores"
backend = "neon"
threads = 8
encoding = "ascii"
qos = "user_initiated"  # P-cores

# ... (all 25 configurations)

[execution]
parallel_experiments = 8  # Run 8 experiments concurrently
warmup_iterations = 5
measurement_iterations = 20
outlier_removal = "iqr"  # Interquartile range method
timeout_seconds = 300  # Per experiment

[output]
format = "parquet"
path = "results/level1_primitives_complete.parquet"
checkpoint_interval = 100  # Save every 100 experiments
```

**Advantages**:
- Human-readable configuration
- Version controllable (git)
- Easy to modify for custom experiments
- Self-documenting

---

#### 2. Operation Registry (`crates/asbb-core/src/operation_registry.rs`)

**Purpose**: Centralized operation metadata and backend selection

**Design**:
```rust
/// Metadata for a single operation
#[derive(Clone, Debug)]
pub struct OperationMetadata {
    pub name: String,
    pub category: OperationCategory,
    pub complexity: f64,
    pub has_neon: bool,
    pub has_gpu: bool,
    pub has_2bit: bool,
    pub description: String,
}

/// Registry of all available operations
pub struct OperationRegistry {
    operations: HashMap<String, Box<dyn Operation>>,
    metadata: HashMap<String, OperationMetadata>,
}

impl OperationRegistry {
    /// Create registry with all 20 operations
    pub fn new() -> Self {
        let mut registry = Self {
            operations: HashMap::new(),
            metadata: HashMap::new(),
        };

        // Register all operations
        registry.register(Box::new(BaseCountingOp::new()));
        registry.register(Box::new(GcContentOp::new()));
        // ... (all 20 operations)

        registry
    }

    /// Register a single operation
    fn register(&mut self, op: Box<dyn Operation>) {
        let metadata = op.metadata();
        self.metadata.insert(metadata.name.clone(), metadata.clone());
        self.operations.insert(metadata.name.clone(), op);
    }

    /// Get operation by name
    pub fn get(&self, name: &str) -> Option<&Box<dyn Operation>> {
        self.operations.get(name)
    }

    /// Get metadata by name
    pub fn get_metadata(&self, name: &str) -> Option<&OperationMetadata> {
        self.metadata.get(name)
    }

    /// List all operation names
    pub fn list_operations(&self) -> Vec<String> {
        self.operations.keys().cloned().collect()
    }
}

/// Trait all operations must implement
pub trait Operation: Send + Sync {
    /// Operation metadata
    fn metadata(&self) -> OperationMetadata;

    /// Execute with specified configuration
    fn execute(&self, config: &ExecutionConfig, data: &[SequenceRecord]) -> Result<OperationResult>;
}

/// Execution configuration for an operation
#[derive(Clone, Debug)]
pub struct ExecutionConfig {
    pub backend: Backend,
    pub threads: usize,
    pub encoding: Encoding,
    pub qos: QualityOfService,
    pub use_gpu: bool,
}

#[derive(Clone, Debug)]
pub enum Backend {
    Naive,
    Neon,
    Parallel,
    Gpu,
}

#[derive(Clone, Debug)]
pub enum Encoding {
    Ascii,
    TwoBit,
}

#[derive(Clone, Debug)]
pub enum QualityOfService {
    Default,
    UserInitiated,  // P-cores
    Background,     // E-cores
}
```

**Advantages**:
- Centralized operation management
- Type-safe backend selection
- Easy to add new operations
- Metadata queryable for analysis

---

#### 3. Execution Engine (`crates/asbb-explorer/src/execution_engine.rs`)

**Purpose**: Parallel experiment execution with progress tracking

**Design**:
```rust
use rayon::prelude::*;
use indicatif::{ProgressBar, MultiProgress};
use std::sync::{Arc, Mutex};

/// Manages execution of all experiments
pub struct ExecutionEngine {
    config: ExperimentConfig,
    registry: Arc<OperationRegistry>,
    results: Arc<Mutex<Vec<ExperimentResult>>>,
    checkpoint_manager: CheckpointManager,
}

impl ExecutionEngine {
    /// Create new execution engine from config
    pub fn new(config: ExperimentConfig) -> Result<Self> {
        Ok(Self {
            config,
            registry: Arc::new(OperationRegistry::new()),
            results: Arc::new(Mutex::new(Vec::new())),
            checkpoint_manager: CheckpointManager::new("checkpoints/level1")?,
        })
    }

    /// Generate all experiment combinations
    pub fn generate_experiments(&self) -> Vec<ExperimentDefinition> {
        let mut experiments = Vec::new();

        for operation in &self.config.operations {
            for hardware_config in &self.config.hardware_configs {
                for scale in &self.config.datasets.scales {
                    experiments.push(ExperimentDefinition {
                        operation: operation.name.clone(),
                        hardware: hardware_config.clone(),
                        scale: scale.sequences,
                        id: format!("{}_{}_{}", operation.name, hardware_config.name, scale.name),
                    });
                }
            }
        }

        experiments
    }

    /// Execute all experiments in parallel
    pub fn run_all(&mut self) -> Result<()> {
        let experiments = self.generate_experiments();
        let total = experiments.len();

        println!("Generated {} experiments", total);
        println!("Running {} experiments in parallel", self.config.execution.parallel_experiments);

        // Check for existing checkpoint
        let completed = self.checkpoint_manager.load_completed()?;
        let remaining: Vec<_> = experiments.into_iter()
            .filter(|e| !completed.contains(&e.id))
            .collect();

        println!("Resuming: {} completed, {} remaining", completed.len(), remaining.len());

        // Set up progress bar
        let progress = Arc::new(ProgressBar::new(remaining.len() as u64));
        progress.set_style(
            indicatif::ProgressStyle::default_bar()
                .template("[{elapsed_precise}] {bar:40.cyan/blue} {pos}/{len} {msg}")
                .unwrap()
        );

        // Configure Rayon thread pool
        let pool = rayon::ThreadPoolBuilder::new()
            .num_threads(self.config.execution.parallel_experiments)
            .build()?;

        // Execute experiments in parallel
        pool.install(|| {
            remaining.par_iter().try_for_each(|experiment| {
                let result = self.execute_single(experiment)?;

                // Store result
                {
                    let mut results = self.results.lock().unwrap();
                    results.push(result);

                    // Checkpoint periodically
                    if results.len() % self.config.execution.checkpoint_interval == 0 {
                        self.checkpoint_manager.save(&results)?;
                    }
                }

                // Update progress
                progress.inc(1);
                progress.set_message(format!("Latest: {}", experiment.id));

                Ok::<(), Error>(())
            })
        })?;

        progress.finish_with_message("All experiments complete");

        // Final save
        let results = self.results.lock().unwrap();
        self.checkpoint_manager.save(&results)?;

        Ok(())
    }

    /// Execute a single experiment
    fn execute_single(&self, experiment: &ExperimentDefinition) -> Result<ExperimentResult> {
        // Load dataset
        let dataset = self.load_dataset(experiment.scale)?;

        // Get operation
        let operation = self.registry.get(&experiment.operation)
            .ok_or_else(|| anyhow!("Operation not found: {}", experiment.operation))?;

        // Configure execution
        let exec_config = ExecutionConfig {
            backend: experiment.hardware.backend,
            threads: experiment.hardware.threads,
            encoding: experiment.hardware.encoding,
            qos: experiment.hardware.qos,
            use_gpu: experiment.hardware.backend == Backend::Gpu,
        };

        // Warmup iterations
        for _ in 0..self.config.execution.warmup_iterations {
            operation.execute(&exec_config, &dataset)?;
        }

        // Measurement iterations
        let mut timings = Vec::new();
        for _ in 0..self.config.execution.measurement_iterations {
            let start = Instant::now();
            let result = operation.execute(&exec_config, &dataset)?;
            let duration = start.elapsed();

            timings.push(duration.as_secs_f64());

            // Validate correctness (compare to baseline)
            if !self.validate_result(&result, experiment)? {
                return Err(anyhow!("Result validation failed for {}", experiment.id));
            }
        }

        // Remove outliers (IQR method)
        let timings = remove_outliers(&timings);

        // Compute statistics
        let mean_time = timings.iter().sum::<f64>() / timings.len() as f64;
        let std_time = standard_deviation(&timings);
        let min_time = timings.iter().cloned().fold(f64::INFINITY, f64::min);
        let max_time = timings.iter().cloned().fold(f64::NEG_INFINITY, f64::max);

        Ok(ExperimentResult {
            experiment_id: experiment.id.clone(),
            operation: experiment.operation.clone(),
            hardware_config: experiment.hardware.name.clone(),
            scale: experiment.scale,
            mean_time_seconds: mean_time,
            std_time_seconds: std_time,
            min_time_seconds: min_time,
            max_time_seconds: max_time,
            throughput_seqs_per_sec: experiment.scale as f64 / mean_time,
            num_iterations: timings.len(),
            timestamp: chrono::Utc::now(),
        })
    }

    /// Load dataset for given scale
    fn load_dataset(&self, scale: usize) -> Result<Vec<SequenceRecord>> {
        // Load or generate dataset
        let path = format!("datasets/level1_{}_150bp.fq", scale);
        if std::path::Path::new(&path).exists() {
            crate::io::load_fastq(&path)
        } else {
            // Generate on-the-fly
            Ok(crate::datagen::generate_synthetic_fastq(scale, 150))
        }
    }

    /// Validate result correctness
    fn validate_result(&self, result: &OperationResult, experiment: &ExperimentDefinition) -> Result<bool> {
        // Compare to baseline (naive implementation)
        // Implementation depends on operation type
        Ok(true)  // Placeholder
    }
}

/// Checkpoint manager for resuming interrupted runs
struct CheckpointManager {
    path: PathBuf,
}

impl CheckpointManager {
    fn new(path: impl Into<PathBuf>) -> Result<Self> {
        let path = path.into();
        std::fs::create_dir_all(&path)?;
        Ok(Self { path })
    }

    /// Load list of completed experiment IDs
    fn load_completed(&self) -> Result<HashSet<String>> {
        let checkpoint_file = self.path.join("completed.json");
        if checkpoint_file.exists() {
            let json = std::fs::read_to_string(&checkpoint_file)?;
            Ok(serde_json::from_str(&json)?)
        } else {
            Ok(HashSet::new())
        }
    }

    /// Save results and update completed list
    fn save(&self, results: &[ExperimentResult]) -> Result<()> {
        // Save results to Parquet (implemented in next section)

        // Update completed list
        let completed: HashSet<String> = results.iter()
            .map(|r| r.experiment_id.clone())
            .collect();
        let json = serde_json::to_string_pretty(&completed)?;
        std::fs::write(self.path.join("completed.json"), json)?;

        Ok(())
    }
}
```

**Key Features**:
- **Parallel execution**: Uses Rayon to run multiple experiments concurrently
- **Progress tracking**: Real-time progress bar showing completion
- **Checkpointing**: Resume from interruptions
- **Outlier removal**: Statistical rigor (IQR method)
- **Validation**: Correctness checks against baseline
- **Error handling**: Graceful failure recovery

**Expected Performance**:
- ~3,000 experiments × ~2 seconds average = 6,000 seconds serial
- Divided by 8 parallel workers = 750 seconds = **12.5 minutes** minimum
- With overhead: **1-2 hours** realistic runtime

---

#### 4. Result Storage (`crates/asbb-explorer/src/result_storage.rs`)

**Purpose**: Efficient structured storage using Parquet format

**Why Parquet?**:
- Columnar format (efficient for analysis)
- Compression (10-100× smaller than CSV)
- Schema enforcement (type safety)
- Ecosystem support (Python pandas, R arrow, DuckDB)

**Schema Design**:
```rust
use arrow::datatypes::{DataType, Field, Schema};
use parquet::arrow::ArrowWriter;

/// Parquet schema for experiment results
pub fn result_schema() -> Schema {
    Schema::new(vec![
        Field::new("experiment_id", DataType::Utf8, false),
        Field::new("operation", DataType::Utf8, false),
        Field::new("hardware_config", DataType::Utf8, false),
        Field::new("backend", DataType::Utf8, false),
        Field::new("threads", DataType::Int32, false),
        Field::new("encoding", DataType::Utf8, false),
        Field::new("qos", DataType::Utf8, false),
        Field::new("scale", DataType::Int64, false),
        Field::new("mean_time_seconds", DataType::Float64, false),
        Field::new("std_time_seconds", DataType::Float64, false),
        Field::new("min_time_seconds", DataType::Float64, false),
        Field::new("max_time_seconds", DataType::Float64, false),
        Field::new("throughput_seqs_per_sec", DataType::Float64, false),
        Field::new("num_iterations", DataType::Int32, false),
        Field::new("timestamp", DataType::Timestamp(arrow::datatypes::TimeUnit::Millisecond, None), false),

        // Operation metadata
        Field::new("operation_category", DataType::Utf8, false),
        Field::new("operation_complexity", DataType::Float64, false),

        // Computed fields for analysis
        Field::new("speedup_vs_baseline", DataType::Float64, true),
        Field::new("speedup_vs_naive", DataType::Float64, true),
    ])
}

/// Write results to Parquet file
pub fn write_results(results: &[ExperimentResult], path: impl AsRef<Path>) -> Result<()> {
    let schema = Arc::new(result_schema());
    let file = File::create(path)?;
    let mut writer = ArrowWriter::try_new(file, schema.clone(), None)?;

    // Convert results to Arrow RecordBatch
    let batch = results_to_record_batch(results, &schema)?;
    writer.write(&batch)?;
    writer.close()?;

    Ok(())
}

/// Read results from Parquet file
pub fn read_results(path: impl AsRef<Path>) -> Result<Vec<ExperimentResult>> {
    let file = File::open(path)?;
    let reader = ParquetFileReader::try_new(file)?;

    // Convert Arrow batches to ExperimentResult
    // Implementation details...

    Ok(results)
}
```

**File Size Estimate**:
- ~3,000 experiments × ~20 columns × ~8 bytes/value = 480 KB uncompressed
- With Parquet compression: **~50-100 KB**
- Much smaller than CSV (~5-10 MB)

---

#### 5. Statistical Analysis (`crates/asbb-analysis/src/lib.rs`)

**Purpose**: Automated statistical modeling and rule extraction

**Components**:
```rust
/// Statistical analysis pipeline
pub struct AnalysisPipeline {
    data: DataFrame,
}

impl AnalysisPipeline {
    /// Load results from Parquet
    pub fn from_parquet(path: impl AsRef<Path>) -> Result<Self> {
        // Load into polars DataFrame for analysis
        let df = LazyFrame::scan_parquet(path, Default::default())?.collect()?;
        Ok(Self { data: df })
    }

    /// Regression analysis: Predict NEON speedup from complexity
    pub fn regression_neon_speedup(&self) -> Result<RegressionModel> {
        // Filter to NEON vs naive comparisons
        let neon_data = self.data.clone()
            .lazy()
            .filter(col("backend").eq(lit("neon")))
            .collect()?;

        // Build linear regression model
        // y = speedup, x = [complexity, log10(scale)]

        let model = linfa::linear_regression::LinearRegression::new()
            .fit(&dataset)?;

        Ok(RegressionModel {
            coefficients: model.params().clone(),
            r_squared: model.r2(&dataset),
            predictions: model.predict(&dataset),
        })
    }

    /// Decision tree: GPU usage rules
    pub fn decision_tree_gpu(&self) -> Result<DecisionTree> {
        // Extract features: neon_speedup, complexity, scale
        // Target: gpu_faster (boolean)

        let tree = linfa::decision_tree::DecisionTree::params()
            .max_depth(5)
            .min_samples_leaf(10)
            .fit(&dataset)?;

        Ok(tree)
    }

    /// Cross-validation: Test prediction accuracy
    pub fn cross_validate(&self, model: &RegressionModel) -> Result<CrossValidationResults> {
        // 80/20 train/test split
        let (train, test) = self.split_train_test(0.8)?;

        // Train on training set
        let trained_model = model.retrain(&train)?;

        // Predict on test set
        let predictions = trained_model.predict(&test)?;
        let actuals = test.get_column("speedup")?;

        // Compute metrics
        let mae = mean_absolute_error(&predictions, &actuals);
        let rmse = root_mean_squared_error(&predictions, &actuals);
        let within_20_pct = percent_within_threshold(&predictions, &actuals, 0.20);

        Ok(CrossValidationResults {
            mae,
            rmse,
            within_20_pct,
            train_size: train.len(),
            test_size: test.len(),
        })
    }

    /// Generate summary statistics
    pub fn summary_stats(&self) -> Result<SummaryStats> {
        // Compute mean, median, std for each operation/config combination
        let stats = self.data.groupby(["operation", "hardware_config"])?
            .agg(&[
                ("mean_time_seconds", &["mean", "std", "min", "max"]),
                ("throughput_seqs_per_sec", &["mean"]),
            ])?;

        Ok(SummaryStats::from_dataframe(stats))
    }
}
```

**Analysis Outputs**:
1. **Regression models**: NEON speedup prediction (target R² > 0.6)
2. **Decision trees**: GPU/parallel/encoding rules
3. **Cross-validation**: Prediction accuracy metrics
4. **Summary statistics**: Per-operation/config summaries

---

## 10 New Operations to Add

### Selection Criteria
- **Diverse categories**: Cover all operation types
- **Complementary complexity**: Fill gaps in 0.20-0.70 range
- **Real-world relevance**: Common in bioinformatics workflows
- **Testable backends**: Can implement naive, NEON, parallel

### New Operations (10)

#### 1. **K-mer Extraction** (Search category)
- **Complexity**: 0.48 (moderate - loop with bitwise operations)
- **Description**: Extract all k-mers (k=21) from sequences
- **Backends**: Naive, NEON (parallel k-mer extraction), Parallel
- **Why**: Tests search operation category, moderate complexity

#### 2. **K-mer Counting** (Search category)
- **Complexity**: 0.52 (hash table operations)
- **Description**: Count frequency of each k-mer
- **Backends**: Naive (HashMap), NEON (vectorized hashing), Parallel
- **Why**: Memory-intensive, tests hash table performance

#### 3. **Hamming Distance** (Pairwise category)
- **Complexity**: 0.35 (simple XOR + popcount)
- **Description**: Compute Hamming distance between sequence pairs
- **Backends**: Naive, NEON (vectorized XOR + popcnt), Parallel, GPU
- **Why**: Very vectorizable, tests pairwise operations

#### 4. **Edit Distance** (Pairwise category)
- **Complexity**: 0.68 (dynamic programming)
- **Description**: Levenshtein distance between sequences
- **Backends**: Naive (DP matrix), Parallel (batched)
- **Why**: Complex, less vectorizable, tests upper complexity bound

#### 5. **Sequence Masking** (Element-wise category)
- **Complexity**: 0.30 (conditional replacement)
- **Description**: Mask low-complexity regions with 'N'
- **Backends**: Naive, NEON (vectorized thresholding), Parallel
- **Why**: Element-wise with conditionals

#### 6. **Translation** (Element-wise category)
- **Complexity**: 0.42 (codon table lookup)
- **Description**: DNA → Protein translation (6-frame)
- **Backends**: Naive, NEON (lookup table), Parallel
- **Why**: Tests lookup table performance

#### 7. **Adapter Trimming** (Filtering category)
- **Complexity**: 0.58 (string matching + trimming)
- **Description**: Remove adapter sequences from ends
- **Backends**: Naive (string search), Parallel
- **Why**: Practical operation, string matching

#### 8. **Quality Statistics** (Aggregation category)
- **Complexity**: 0.38 (mean/median/std calculation)
- **Description**: Compute quality score statistics per sequence
- **Backends**: Naive, NEON (vectorized stats), Parallel
- **Why**: Tests reduction operations

#### 9. **MinHash Sketching** (Aggregation category)
- **Complexity**: 0.55 (hashing + min heap)
- **Description**: Compute MinHash signature for each sequence
- **Backends**: Naive, Parallel
- **Why**: Modern sketching algorithm, tests heap operations

#### 10. **FASTQ Parsing** (I/O category)
- **Complexity**: 0.44 (parsing + validation)
- **Description**: Parse FASTQ format into SequenceRecord
- **Backends**: Naive, SIMD (vectorized parsing), Parallel
- **Why**: I/O benchmark, practical bottleneck

### Coverage Analysis

**By Category**:
- Element-wise: 7 total (4 existing + 3 new) ✅
- Filtering: 3 total (2 existing + 1 new) ✅
- Search: 3 total (0 existing + 3 new) ✅
- Pairwise: 2 total (0 existing + 2 new) ✅
- Aggregation: 4 total (2 existing + 2 new) ✅
- I/O: 1 total (0 existing + 1 new) ✅

**By Complexity** (after adding 10 new):
- 0.20-0.30: 3 operations (trivial)
- 0.30-0.40: 5 operations (simple)
- 0.40-0.50: 5 operations (moderate)
- 0.50-0.60: 5 operations (complex)
- 0.60-0.70: 2 operations (very complex)

**Excellent coverage** across both categories and complexity spectrum.

---

## Hardware Configuration Matrix (25 configs)

### Configuration Categories

**Baseline** (1):
1. Naive, 1 thread, ASCII, Default QoS

**NEON variants** (4):
2. NEON, 1 thread
3. NEON, 2 threads
4. NEON, 4 threads
5. NEON, 8 threads

**Parallel variants** (3):
6. Naive, 2 threads
7. Naive, 4 threads
8. Naive, 8 threads

**Core assignment** (6):
9. NEON, 2 threads, P-cores
10. NEON, 2 threads, E-cores
11. NEON, 4 threads, P-cores
12. NEON, 4 threads, E-cores
13. NEON, 8 threads, P-cores
14. NEON, 8 threads, E-cores

**Encoding** (2):
15. NEON, 8 threads, 2-bit encoding (for supported ops)
16. Naive, 1 thread, 2-bit encoding

**GPU** (2):
17. GPU, small batch (1K sequences)
18. GPU, large batch (full dataset)

**Combined optimizations** (7):
19. NEON + 8 threads + P-cores + ASCII
20. NEON + 8 threads + E-cores + ASCII
21. NEON + 8 threads + Default QoS + ASCII
22. NEON + 4 threads + P-cores + 2-bit (if supported)
23. NEON + 8 threads + P-cores + 2-bit (if supported)
24. GPU + NEON preprocessing
25. Best predicted config (from Phase 1 rules)

**Total**: 25 configurations

---

## Experiment Matrix

**Dimensions**:
- 20 operations
- 25 hardware configurations
- 6 data scales

**Total**: 20 × 25 × 6 = **3,000 experiments**

**Notes**:
- Some configurations only apply to specific operations (e.g., 2-bit encoding for 4 operations)
- GPU configurations only for GPU-capable operations
- Total may be slightly less due to inapplicable combinations (~2,800-2,900 actual experiments)

---

## Implementation Plan

### Week 1: Infrastructure + Operations (5 days)

**Days 1-2**: Core Infrastructure
- [ ] Create `experiments/level1_primitives/config.toml`
- [ ] Implement `OperationRegistry`
- [ ] Implement `ExecutionEngine`
- [ ] Implement `CheckpointManager`
- [ ] Implement Parquet storage

**Days 3-5**: Add 10 New Operations
- [ ] K-mer extraction
- [ ] K-mer counting
- [ ] Hamming distance
- [ ] Edit distance
- [ ] Sequence masking
- [ ] Translation
- [ ] Adapter trimming
- [ ] Quality statistics
- [ ] MinHash sketching
- [ ] FASTQ parsing

Each operation: Naive + NEON (if applicable) + tests

### Week 2: Execution (2 days)

**Day 1**: Run Experiments
- [ ] Generate datasets (if needed)
- [ ] Launch automated execution
- [ ] Monitor progress
- [ ] **Expected runtime**: 1-2 days

**Day 2**: Validate Results
- [ ] Check for failures
- [ ] Validate correctness
- [ ] Spot-check performance metrics

### Week 3: Analysis + Publication (5 days)

**Days 1-3**: Statistical Analysis
- [ ] Implement regression models
- [ ] Implement decision trees
- [ ] Cross-validation
- [ ] Generate summary statistics

**Days 4-5**: Publication Preparation
- [ ] Draft methodology paper
- [ ] Create figures
- [ ] Prepare supplementary materials
- [ ] Update documentation

---

## Success Criteria

**Technical**:
- [ ] All 3,000 experiments execute successfully
- [ ] <5% failure rate
- [ ] All results validate correctly
- [ ] Parquet file generated and loadable

**Scientific**:
- [ ] R² > 0.6 for NEON regression model
- [ ] Prediction accuracy >80% within 20% error
- [ ] Decision tree accuracy >90%
- [ ] Phase 1 rules validated or refined

**Timeline**:
- [ ] Infrastructure complete: Day 2
- [ ] Operations complete: Day 5
- [ ] Execution complete: Day 7
- [ ] Analysis complete: Day 12
- [ ] Publication draft: Day 14

---

## Next Steps

1. **Review this design** - Ensure approach is sound
2. **Create config.toml** - Define all 3,000 experiments
3. **Implement OperationRegistry** - Centralize operation management
4. **Implement ExecutionEngine** - Automated parallel execution
5. **Add first new operation** - Validate approach before scaling

**Ready to proceed?** Let me know if you want to:
- Modify the design
- Start implementation (which component first?)
- Define the operations in more detail

---

**Status**: Design Complete ✅
**Next**: Implementation (awaiting approval)
**Timeline**: 3 weeks from start to publication draft
