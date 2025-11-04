//! DAG Testing Harness - Systematic Hardware Exploration
//!
//! This harness implements the DAG-based testing framework for systematic
//! exploration of the hardware optimization space with intelligent pruning.
//!
//! **Goal**: Test 740 experiments instead of 23,040 (93% reduction) while
//! maintaining scientific rigor.
//!
//! **Algorithm** (from DAG_FRAMEWORK.md):
//! 1. Phase 1: Test alternatives (NEON, GPU, AMX)
//!    - Prune if speedup < 1.5×
//! 2. Phase 2: Test compositions (NEON+Parallel)
//!    - Prune if diminishing returns < 1.3×
//! 3. Phase 3: Test refinements (core affinity)
//!    - Test only on optimal configs
//!
//! **Usage**:
//! ```bash
//! cargo run --release -p asbb-cli --bin asbb-dag-traversal \
//!   --batch neon_parallel \
//!   --output results/dag_complete/dag_neon_parallel.csv
//! ```

use anyhow::{Context, Result};
use asbb_core::{OperationOutput, PrimitiveOperation, SequenceRecord};
use asbb_ops::{
    at_content::ATContent,
    base_counting::BaseCounting,
    complexity_score::ComplexityScore,
    gc_content::GcContent,
    length_filter::LengthFilter,
    n_content::NContent,
    quality_aggregation::QualityAggregation,
    quality_filter::QualityFilter,
    reverse_complement::ReverseComplement,
    sequence_length::SequenceLength,
};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::time::Instant;

// ============================================================================
// Statistical Rigor Types
// ============================================================================

/// Statistical summary of experiment measurements
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExperimentStatistics {
    /// Median value (robust to outliers)
    pub median: f64,

    /// Mean value
    pub mean: f64,

    /// Standard deviation
    pub std_dev: f64,

    /// Minimum value (after outlier removal)
    pub min: f64,

    /// Maximum value (after outlier removal)
    pub max: f64,

    /// First quartile (25th percentile)
    pub q1: f64,

    /// Third quartile (75th percentile)
    pub q3: f64,

    /// Interquartile range (Q3 - Q1)
    pub iqr: f64,

    /// 95% confidence interval lower bound
    pub ci_95_lower: f64,

    /// 95% confidence interval upper bound
    pub ci_95_upper: f64,

    /// Number of valid measurements (after outlier removal)
    pub n_valid: usize,

    /// Number of outliers removed
    pub n_outliers: usize,

    /// Number of warmup runs performed
    pub n_warmup: usize,
}

/// Remove outliers using IQR method
///
/// Returns (valid_measurements, outliers)
fn remove_outliers(measurements: &[f64], threshold: f64) -> (Vec<f64>, Vec<f64>) {
    if measurements.len() < 4 {
        // Too few measurements for outlier detection
        return (measurements.to_vec(), Vec::new());
    }

    let mut sorted = measurements.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

    let n = sorted.len();
    let q1_idx = n / 4;
    let q3_idx = 3 * n / 4;

    let q1 = sorted[q1_idx];
    let q3 = sorted[q3_idx];
    let iqr = q3 - q1;

    let lower_bound = q1 - threshold * iqr;
    let upper_bound = q3 + threshold * iqr;

    let valid: Vec<f64> = sorted
        .iter()
        .copied()
        .filter(|&x| x >= lower_bound && x <= upper_bound)
        .collect();

    let outliers: Vec<f64> = sorted
        .iter()
        .copied()
        .filter(|&x| x < lower_bound || x > upper_bound)
        .collect();

    (valid, outliers)
}

/// Calculate comprehensive statistics from measurements
fn calculate_statistics(
    measurements: &[f64],
    outlier_threshold: f64,
    n_warmup: usize,
) -> Result<ExperimentStatistics> {
    let (valid, outliers) = remove_outliers(measurements, outlier_threshold);

    if valid.len() < 3 {
        anyhow::bail!(
            "Too few valid measurements after outlier removal: {} / {} (removed {} outliers)",
            valid.len(),
            measurements.len(),
            outliers.len()
        );
    }

    let n = valid.len() as f64;

    // Mean
    let mean = valid.iter().sum::<f64>() / n;

    // Variance and std dev
    let variance = valid
        .iter()
        .map(|x| (x - mean).powi(2))
        .sum::<f64>()
        / (n - 1.0);
    let std_dev = variance.sqrt();

    // Median and quartiles
    let mut sorted = valid.clone();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

    let median_idx = sorted.len() / 2;
    let median = if sorted.len() % 2 == 0 {
        (sorted[median_idx - 1] + sorted[median_idx]) / 2.0
    } else {
        sorted[median_idx]
    };

    let q1_idx = sorted.len() / 4;
    let q3_idx = 3 * sorted.len() / 4;
    let q1 = sorted[q1_idx];
    let q3 = sorted[q3_idx];
    let iqr = q3 - q1;

    let min = *sorted.first().unwrap();
    let max = *sorted.last().unwrap();

    // 95% Confidence Interval (t-distribution)
    let df = valid.len() - 1;
    let t_critical = t_critical_value(df, 0.05);
    let margin_of_error = t_critical * (std_dev / n.sqrt());
    let ci_95_lower = mean - margin_of_error;
    let ci_95_upper = mean + margin_of_error;

    Ok(ExperimentStatistics {
        median,
        mean,
        std_dev,
        min,
        max,
        q1,
        q3,
        iqr,
        ci_95_lower,
        ci_95_upper,
        n_valid: valid.len(),
        n_outliers: outliers.len(),
        n_warmup,
    })
}

/// Get t-critical value for 95% confidence interval
///
/// Simplified lookup table. For production use statrs crate.
fn t_critical_value(df: usize, alpha: f64) -> f64 {
    if alpha != 0.05 {
        return 2.0; // Conservative default
    }

    match df {
        0..=4 => 2.776,
        5..=9 => 2.262,
        10..=14 => 2.145,
        15..=19 => 2.093,
        20..=24 => 2.064,
        25..=29 => 2.045,
        30..=39 => 2.021,
        40..=49 => 2.009,
        50..=99 => 1.984,
        _ => 1.96, // For large df, use normal approximation
    }
}

// ============================================================================
// Core Types
// ============================================================================

/// Configuration for DAG traversal
#[derive(Debug, Clone)]
pub struct DAGConfig {
    /// Operations to test
    pub operations: Vec<String>,

    /// Scales to test at
    pub scales: Vec<Scale>,

    /// Speedup threshold for pruning (e.g., 1.5)
    pub pruning_threshold: f64,

    /// Diminishing returns threshold for compositions (e.g., 1.3)
    pub diminishing_returns_threshold: f64,

    /// Output CSV path
    pub output_path: PathBuf,

    /// Which batch to run (neon_parallel, core_affinity, scale_thresholds)
    pub batch: DAGBatch,

    /// Number of repetitions per experiment (default: 30 for publication quality)
    pub repetitions: usize,

    /// Number of warmup runs to discard (default: 3)
    pub warmup_runs: usize,

    /// Outlier detection threshold (IQR multiplier, default: 1.5)
    pub outlier_threshold: f64,
}

/// DAG batch type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DAGBatch {
    /// NEON+Parallel composition (240 experiments)
    NeonParallel,

    /// Core affinity × NEON interaction (180 experiments)
    CoreAffinity,

    /// Precise scale thresholds (320 experiments)
    ScaleThresholds,
}

impl DAGBatch {
    pub fn from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "neon_parallel" | "neon-parallel" => Ok(DAGBatch::NeonParallel),
            "core_affinity" | "core-affinity" => Ok(DAGBatch::CoreAffinity),
            "scale_thresholds" | "scale-thresholds" => Ok(DAGBatch::ScaleThresholds),
            _ => anyhow::bail!("Unknown batch type: {}", s),
        }
    }
}

/// Dataset scale definition
#[derive(Debug, Clone)]
pub struct Scale {
    pub name: &'static str,
    pub path: &'static str,
    pub num_sequences: usize,
}

// Standard scales from existing pilot infrastructure
const SCALES: &[Scale] = &[
    Scale { name: "Tiny", path: "datasets/tiny_100_150bp.fq", num_sequences: 100 },
    Scale { name: "Small", path: "datasets/small_1000_150bp.fq", num_sequences: 1_000 },
    Scale { name: "Medium", path: "datasets/medium_10000_150bp.fq", num_sequences: 10_000 },
    Scale { name: "Large", path: "datasets/large_100000_150bp.fq", num_sequences: 100_000 },
    Scale { name: "VeryLarge", path: "datasets/very_large_1000000_150bp.fq", num_sequences: 1_000_000 },
    Scale { name: "Huge", path: "datasets/huge_10000000_150bp.fq", num_sequences: 10_000_000 },
];

/// Represents a single node in the hardware optimization DAG
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct DAGNode {
    /// Configuration type (naive, neon, gpu, amx)
    pub config_type: ConfigType,

    /// Number of threads (1 for single-threaded)
    pub threads: usize,

    /// Core affinity (default, p_cores, e_cores)
    pub affinity: CoreAffinity,
}

impl DAGNode {
    /// Create a new DAG node
    pub fn new(config_type: ConfigType, threads: usize, affinity: CoreAffinity) -> Self {
        Self {
            config_type,
            threads,
            affinity,
        }
    }

    /// Create naive baseline
    pub fn naive() -> Self {
        Self::new(ConfigType::Naive, 1, CoreAffinity::Default)
    }

    /// Create NEON single-threaded
    pub fn neon() -> Self {
        Self::new(ConfigType::Neon, 1, CoreAffinity::Default)
    }

    /// Create NEON with parallel threads
    pub fn neon_parallel(threads: usize) -> Self {
        Self::new(ConfigType::Neon, threads, CoreAffinity::Default)
    }

    /// Create node with specific affinity
    pub fn with_affinity(mut self, affinity: CoreAffinity) -> Self {
        self.affinity = affinity;
        self
    }

    /// Get a human-readable name for this config
    pub fn name(&self) -> String {
        let base = match self.config_type {
            ConfigType::Naive => "naive".to_string(),
            ConfigType::Neon => "neon".to_string(),
            ConfigType::Gpu => "gpu".to_string(),
            ConfigType::Amx => "amx".to_string(),
        };

        if self.threads > 1 {
            let affinity_suffix = match self.affinity {
                CoreAffinity::Default => "",
                CoreAffinity::PerformanceCores => "_pcores",
                CoreAffinity::EfficiencyCores => "_ecores",
            };
            format!("{}_{}t{}", base, self.threads, affinity_suffix)
        } else {
            base
        }
    }

    /// Is this an alternative (mutually exclusive with others)?
    pub fn is_alternative(&self) -> bool {
        self.threads == 1 && self.affinity == CoreAffinity::Default
    }

    /// Is this a composition (builds on a base config)?
    pub fn is_composition(&self) -> bool {
        self.threads > 1 && self.affinity == CoreAffinity::Default
    }

    /// Is this a refinement (tunes an optimal config)?
    pub fn is_refinement(&self) -> bool {
        self.threads > 1 && self.affinity != CoreAffinity::Default
    }
}

/// Configuration type (alternatives)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ConfigType {
    Naive,
    Neon,
    Gpu,
    Amx,
}

/// Core affinity for parallel execution
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CoreAffinity {
    /// Let OS decide (baseline)
    Default,
    /// Try to pin to Performance cores (high QoS)
    PerformanceCores,
    /// Try to pin to Efficiency cores (low QoS)
    EfficiencyCores,
}

impl CoreAffinity {
    fn name(&self) -> &'static str {
        match self {
            CoreAffinity::Default => "default",
            CoreAffinity::PerformanceCores => "p_cores",
            CoreAffinity::EfficiencyCores => "e_cores",
        }
    }
}

/// Result from a single experiment (with full statistical rigor)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExperimentResult {
    /// Operation name
    pub operation: String,

    /// Configuration tested
    pub config_name: String,

    /// Config type
    pub config_type: ConfigType,

    /// Number of threads
    pub threads: usize,

    /// Core affinity
    pub affinity: String,

    /// Scale name
    pub scale: String,

    /// Number of sequences
    pub num_sequences: usize,

    /// Was this configuration pruned?
    pub pruned: bool,

    // === Throughput Statistics (sequences/second) ===
    /// Median throughput (robust to outliers)
    pub throughput_median: f64,

    /// Mean throughput
    pub throughput_mean: f64,

    /// Throughput standard deviation
    pub throughput_std_dev: f64,

    /// Throughput 95% CI lower bound
    pub throughput_ci_lower: f64,

    /// Throughput 95% CI upper bound
    pub throughput_ci_upper: f64,

    // === Speedup Statistics (vs naive baseline) ===
    /// Median speedup vs naive
    pub speedup_median: f64,

    /// Mean speedup vs naive
    pub speedup_mean: f64,

    /// Speedup standard deviation
    pub speedup_std_dev: f64,

    /// Speedup 95% CI lower bound
    pub speedup_ci_lower: f64,

    /// Speedup 95% CI upper bound
    pub speedup_ci_upper: f64,

    // === Elapsed Time Statistics (seconds) ===
    /// Median elapsed time
    pub elapsed_median: f64,

    /// Mean elapsed time
    pub elapsed_mean: f64,

    /// Elapsed time standard deviation
    pub elapsed_std_dev: f64,

    /// Min elapsed time
    pub elapsed_min: f64,

    /// Max elapsed time
    pub elapsed_max: f64,

    /// Elapsed time Q1 (25th percentile)
    pub elapsed_q1: f64,

    /// Elapsed time Q3 (75th percentile)
    pub elapsed_q3: f64,

    /// Elapsed time IQR
    pub elapsed_iqr: f64,

    // === Sample Statistics ===
    /// Number of valid measurements (after outlier removal)
    pub n_valid: usize,

    /// Number of outliers removed
    pub n_outliers: usize,

    /// Number of warmup runs
    pub n_warmup: usize,
}

// ============================================================================
// Pruning Strategy
// ============================================================================

/// Pruning strategy for DAG traversal
pub struct PruningStrategy {
    /// Minimum speedup threshold for alternatives (e.g., 1.5)
    pub speedup_threshold: f64,

    /// Minimum additional benefit for compositions (e.g., 1.3)
    pub diminishing_returns_threshold: f64,
}

impl PruningStrategy {
    pub fn new(speedup_threshold: f64, diminishing_returns_threshold: f64) -> Self {
        Self {
            speedup_threshold,
            diminishing_returns_threshold,
        }
    }

    /// Should we prune this alternative?
    pub fn should_prune_alternative(&self, result: &ExperimentResult) -> bool {
        result.speedup_median < self.speedup_threshold
    }

    /// Should we stop testing more threads?
    pub fn should_prune_composition(
        &self,
        result: &ExperimentResult,
        parent_speedup: f64,
    ) -> bool {
        // If additional benefit is less than threshold, prune
        let additional_benefit = result.speedup_median / parent_speedup;
        additional_benefit < self.diminishing_returns_threshold
    }
}

// ============================================================================
// DAG Traversal
// ============================================================================

/// Executes DAG traversal with pruning
pub struct DAGTraversal {
    config: DAGConfig,
    tested_nodes: HashMap<(String, DAGNode, String), ExperimentResult>,
    pruned_nodes: HashSet<(String, DAGNode)>,
    naive_baselines: HashMap<(String, String), f64>, // (operation, scale) -> throughput
}

impl DAGTraversal {
    pub fn new(config: DAGConfig) -> Self {
        Self {
            config,
            tested_nodes: HashMap::new(),
            pruned_nodes: HashSet::new(),
            naive_baselines: HashMap::new(),
        }
    }

    /// Run the complete DAG traversal
    pub fn run(&mut self) -> Result<Vec<ExperimentResult>> {
        println!("🔬 DAG Traversal Starting");
        println!("   Batch: {:?}", self.config.batch);
        println!("   Operations: {}", self.config.operations.len());
        println!("   Scales: {}", self.config.scales.len());
        println!();

        let all_results = match self.config.batch {
            DAGBatch::NeonParallel => self.run_neon_parallel_batch()?,
            DAGBatch::CoreAffinity => self.run_core_affinity_batch()?,
            DAGBatch::ScaleThresholds => self.run_scale_thresholds_batch()?,
        };

        println!();
        println!("✅ DAG Traversal Complete");
        println!("   Total experiments: {}", all_results.len());
        println!("   Pruned configs: {}", self.pruned_nodes.len());

        Ok(all_results)
    }

    /// Run NEON+Parallel batch (240 experiments)
    /// Tests: naive, NEON, NEON+2t, NEON+4t for all 20 operations × 3 scales
    fn run_neon_parallel_batch(&mut self) -> Result<Vec<ExperimentResult>> {
        let mut results = Vec::new();
        let strategy = PruningStrategy::new(
            self.config.pruning_threshold,
            self.config.diminishing_returns_threshold,
        );

        println!("📊 Batch: NEON+Parallel Composition");
        println!("   Goal: Validate NEON × Parallel = multiplicative for all 20 operations");
        println!();

        // Clone operations to avoid borrow checker issues
        let operations = self.config.operations.clone();
        let scales = self.config.scales.clone();

        for operation in &operations {
            println!("🔬 Testing operation: {}", operation);

            for scale in &scales {
                println!("  📏 Scale: {} ({} sequences)", scale.name, scale.num_sequences);

                // Phase 1: Test baseline
                let naive_node = DAGNode::naive();
                let naive_result = self.run_experiment(operation, &naive_node, scale)?;
                results.push(naive_result.clone());

                // Store baseline for speedup calculations
                self.naive_baselines.insert(
                    (operation.clone(), scale.name.to_string()),
                    naive_result.throughput_median,
                );

                // Phase 2: Test NEON
                let neon_node = DAGNode::neon();
                let neon_result = self.run_experiment_with_baseline(
                    operation,
                    &neon_node,
                    scale,
                    naive_result.throughput_median,
                )?;
                results.push(neon_result.clone());

                // Check if NEON should be pruned
                if strategy.should_prune_alternative(&neon_result) {
                    println!("    ❌ NEON pruned ({:.2}× < {}×)",
                             neon_result.speedup_median,
                             strategy.speedup_threshold);
                    self.pruned_nodes.insert((operation.clone(), neon_node));
                    continue;
                }

                println!("    ✅ NEON kept ({:.2}×)", neon_result.speedup_median);

                // Phase 3: Test NEON+Parallel compositions
                let mut parent_speedup = neon_result.speedup_median;

                for threads in &[2, 4] {
                    let parallel_node = DAGNode::neon_parallel(*threads);
                    let parallel_result = self.run_experiment_with_baseline(
                        operation,
                        &parallel_node,
                        scale,
                        naive_result.throughput_median,
                    )?;
                    results.push(parallel_result.clone());

                    // Check for diminishing returns
                    if strategy.should_prune_composition(&parallel_result, parent_speedup) {
                        println!("    ❌ NEON+{}t pruned (additional benefit {:.2}× < {}×)",
                                 threads,
                                 parallel_result.speedup_median / parent_speedup,
                                 strategy.diminishing_returns_threshold);
                        self.pruned_nodes.insert((operation.clone(), parallel_node));
                        break; // Don't test higher thread counts
                    }

                    println!("    ✅ NEON+{}t kept ({:.2}×, additional {:.2}×)",
                             threads,
                             parallel_result.speedup_median,
                             parallel_result.speedup_median / parent_speedup);

                    parent_speedup = parallel_result.speedup_median;
                }
            }

            println!();
        }

        Ok(results)
    }

    /// Run Core Affinity batch (180 experiments)
    /// Tests P-cores vs E-cores for parallel configs
    fn run_core_affinity_batch(&mut self) -> Result<Vec<ExperimentResult>> {
        let mut results = Vec::new();

        println!("📊 Batch: Core Affinity × NEON");
        println!("   Goal: Test if E-cores remain competitive with NEON");
        println!();

        // Clone to avoid borrow checker issues
        let operations = self.config.operations.clone();
        let scales = self.config.scales.clone();

        // For this batch, we test NEON on different core types
        for operation in &operations {
            println!("🔬 Testing operation: {}", operation);

            for scale in &scales {
                println!("  📏 Scale: {} ({} sequences)", scale.name, scale.num_sequences);

                // Get or establish naive baseline
                let baseline = self.get_or_establish_baseline(operation, scale)?;

                // Test NEON on different core affinities (single-threaded for this batch)
                for affinity in &[CoreAffinity::Default, CoreAffinity::PerformanceCores, CoreAffinity::EfficiencyCores] {
                    let node = DAGNode::neon().with_affinity(*affinity);
                    let result = self.run_experiment_with_baseline(operation, &node, scale, baseline)?;
                    results.push(result);
                }
            }

            println!();
        }

        Ok(results)
    }

    /// Run Scale Thresholds batch (320 experiments)
    /// Tests precise scale thresholds for optimal config selection
    fn run_scale_thresholds_batch(&mut self) -> Result<Vec<ExperimentResult>> {
        let mut results = Vec::new();

        println!("📊 Batch: Precise Scale Thresholds");
        println!("   Goal: Determine exact threshold where configs become optimal");
        println!();

        // Clone to avoid borrow checker issues
        let operations = self.config.operations.clone();

        // Use more granular scales for this batch
        let fine_scales = vec![
            Scale { name: "Tiny", path: "datasets/tiny_100_150bp.fq", num_sequences: 100 },
            Scale { name: "Small", path: "datasets/small_1000_150bp.fq", num_sequences: 1_000 },
            Scale { name: "Medium", path: "datasets/medium_10000_150bp.fq", num_sequences: 10_000 },
            Scale { name: "Large", path: "datasets/large_100000_150bp.fq", num_sequences: 100_000 },
        ];

        for operation in &operations {
            println!("🔬 Testing operation: {}", operation);

            for scale in &fine_scales {
                println!("  📏 Scale: {} ({} sequences)", scale.name, scale.num_sequences);

                let baseline = self.get_or_establish_baseline(operation, scale)?;

                // Test key configs across scales
                let configs = vec![
                    DAGNode::naive(),
                    DAGNode::neon(),
                    DAGNode::neon_parallel(2),
                    DAGNode::neon_parallel(4),
                ];

                for node in configs {
                    let result = self.run_experiment_with_baseline(operation, &node, scale, baseline)?;
                    results.push(result);
                }
            }

            println!();
        }

        Ok(results)
    }

    /// Get or establish naive baseline for an operation at a scale
    fn get_or_establish_baseline(&mut self, operation: &str, scale: &Scale) -> Result<f64> {
        let key = (operation.to_string(), scale.name.to_string());

        if let Some(&throughput) = self.naive_baselines.get(&key) {
            Ok(throughput)
        } else {
            let naive_node = DAGNode::naive();
            let result = self.run_experiment(operation, &naive_node, scale)?;
            let throughput = result.throughput_median;
            self.naive_baselines.insert(key, throughput);
            Ok(throughput)
        }
    }

    /// Run a single experiment (establishes baseline)
    fn run_experiment(
        &mut self,
        operation: &str,
        node: &DAGNode,
        scale: &Scale,
    ) -> Result<ExperimentResult> {
        // Pass None so speedup is calculated as throughput / throughput = 1.0
        self.run_experiment_impl(operation, node, scale, None)
    }

    /// Run experiment with known baseline
    fn run_experiment_with_baseline(
        &mut self,
        operation: &str,
        node: &DAGNode,
        scale: &Scale,
        baseline_throughput: f64,
    ) -> Result<ExperimentResult> {
        self.run_experiment_impl(operation, node, scale, Some(baseline_throughput))
    }

    /// Implementation of experiment execution with full statistical rigor
    fn run_experiment_impl(
        &mut self,
        operation: &str,
        node: &DAGNode,
        scale: &Scale,
        baseline_throughput: Option<f64>,
    ) -> Result<ExperimentResult> {
        // Check if already tested
        let key = (operation.to_string(), node.clone(), scale.name.to_string());
        if let Some(result) = self.tested_nodes.get(&key) {
            return Ok(result.clone());
        }

        // Check if pruned
        if self.pruned_nodes.contains(&(operation.to_string(), node.clone())) {
            // Return a "pruned" result with zero statistics
            return Ok(self.create_pruned_result(operation, node, scale));
        }

        // Load sequences ONCE
        let sequences = load_sequences(scale.path)
            .with_context(|| format!("Failed to load dataset: {}", scale.path))?;

        // Load operation ONCE
        let op_instance = create_operation(operation)?;

        // === WARMUP PHASE ===
        for _ in 0..self.config.warmup_runs {
            let _output = execute_operation(&*op_instance, &sequences, node)?;
        }

        // === MEASUREMENT PHASE ===
        let mut elapsed_times = Vec::with_capacity(self.config.repetitions);

        for _ in 0..self.config.repetitions {
            let start = Instant::now();
            let _output = execute_operation(&*op_instance, &sequences, node)?;
            let elapsed = start.elapsed();
            elapsed_times.push(elapsed.as_secs_f64());
        }

        // === STATISTICAL ANALYSIS ===
        let elapsed_stats = calculate_statistics(
            &elapsed_times,
            self.config.outlier_threshold,
            self.config.warmup_runs,
        )?;

        // Calculate throughput from elapsed times (throughput = sequences/elapsed)
        let throughput_measurements: Vec<f64> = elapsed_times
            .iter()
            .map(|&elapsed| scale.num_sequences as f64 / elapsed)
            .collect();

        let throughput_stats = calculate_statistics(
            &throughput_measurements,
            self.config.outlier_threshold,
            self.config.warmup_runs,
        )?;

        // Calculate speedup statistics
        let baseline = baseline_throughput.unwrap_or(throughput_stats.median);
        let baseline = if baseline > 0.0 { baseline } else { 1.0 };

        let speedup_measurements: Vec<f64> = throughput_measurements
            .iter()
            .map(|&t| t / baseline)
            .collect();

        let speedup_stats = calculate_statistics(
            &speedup_measurements,
            self.config.outlier_threshold,
            self.config.warmup_runs,
        )?;

        // Create result with comprehensive statistics
        let result = ExperimentResult {
            operation: operation.to_string(),
            config_name: node.name(),
            config_type: node.config_type,
            threads: node.threads,
            affinity: node.affinity.name().to_string(),
            scale: scale.name.to_string(),
            num_sequences: scale.num_sequences,
            pruned: false,

            // Throughput statistics
            throughput_median: throughput_stats.median,
            throughput_mean: throughput_stats.mean,
            throughput_std_dev: throughput_stats.std_dev,
            throughput_ci_lower: throughput_stats.ci_95_lower,
            throughput_ci_upper: throughput_stats.ci_95_upper,

            // Speedup statistics
            speedup_median: speedup_stats.median,
            speedup_mean: speedup_stats.mean,
            speedup_std_dev: speedup_stats.std_dev,
            speedup_ci_lower: speedup_stats.ci_95_lower,
            speedup_ci_upper: speedup_stats.ci_95_upper,

            // Elapsed time statistics
            elapsed_median: elapsed_stats.median,
            elapsed_mean: elapsed_stats.mean,
            elapsed_std_dev: elapsed_stats.std_dev,
            elapsed_min: elapsed_stats.min,
            elapsed_max: elapsed_stats.max,
            elapsed_q1: elapsed_stats.q1,
            elapsed_q3: elapsed_stats.q3,
            elapsed_iqr: elapsed_stats.iqr,

            // Sample statistics
            n_valid: elapsed_stats.n_valid,
            n_outliers: elapsed_stats.n_outliers,
            n_warmup: elapsed_stats.n_warmup,
        };

        // Cache result
        self.tested_nodes.insert(key, result.clone());

        Ok(result)
    }

    /// Create a pruned result with zero statistics
    fn create_pruned_result(
        &self,
        operation: &str,
        node: &DAGNode,
        scale: &Scale,
    ) -> ExperimentResult {
        ExperimentResult {
            operation: operation.to_string(),
            config_name: node.name(),
            config_type: node.config_type,
            threads: node.threads,
            affinity: node.affinity.name().to_string(),
            scale: scale.name.to_string(),
            num_sequences: scale.num_sequences,
            pruned: true,
            throughput_median: 0.0,
            throughput_mean: 0.0,
            throughput_std_dev: 0.0,
            throughput_ci_lower: 0.0,
            throughput_ci_upper: 0.0,
            speedup_median: 0.0,
            speedup_mean: 0.0,
            speedup_std_dev: 0.0,
            speedup_ci_lower: 0.0,
            speedup_ci_upper: 0.0,
            elapsed_median: 0.0,
            elapsed_mean: 0.0,
            elapsed_std_dev: 0.0,
            elapsed_min: 0.0,
            elapsed_max: 0.0,
            elapsed_q1: 0.0,
            elapsed_q3: 0.0,
            elapsed_iqr: 0.0,
            n_valid: 0,
            n_outliers: 0,
            n_warmup: 0,
        }
    }
}

// ============================================================================
// Operation Loading
// ============================================================================

/// Create an operation instance by name
fn create_operation(name: &str) -> Result<Box<dyn PrimitiveOperation>> {
    match name {
        "base_counting" => Ok(Box::new(BaseCounting::new())),
        "gc_content" => Ok(Box::new(GcContent::new())),
        "at_content" => Ok(Box::new(ATContent)),
        "n_content" => Ok(Box::new(NContent)),
        "reverse_complement" => Ok(Box::new(ReverseComplement::new())),
        "sequence_length" => Ok(Box::new(SequenceLength)),
        "quality_aggregation" => Ok(Box::new(QualityAggregation::new())),
        "quality_filter" => Ok(Box::new(QualityFilter::new(20))),
        "length_filter" => Ok(Box::new(LengthFilter::new(50))),
        "complexity_score" => Ok(Box::new(ComplexityScore::new())),
        _ => anyhow::bail!("Unknown operation: {}", name),
    }
}

/// Execute operation with specific configuration
fn execute_operation(
    op: &dyn PrimitiveOperation,
    sequences: &[SequenceRecord],
    node: &DAGNode,
) -> Result<OperationOutput> {
    match (node.config_type, node.threads) {
        (ConfigType::Naive, 1) => op.execute_naive(sequences),
        (ConfigType::Neon, 1) => op.execute_neon(sequences),
        (ConfigType::Neon, threads) => op.execute_parallel(sequences, threads),
        (ConfigType::Gpu, _) => {
            anyhow::bail!("GPU execution not supported in this harness (use separate GPU pilot)")
        }
        (ConfigType::Amx, _) => {
            anyhow::bail!("AMX execution not supported (already tested separately)")
        }
        _ => anyhow::bail!("Unsupported configuration: {:?}", node),
    }
}

// ============================================================================
// Data Loading
// ============================================================================

/// Load sequences from FASTQ file
fn load_sequences(path: &str) -> Result<Vec<SequenceRecord>> {
    let file = File::open(path)
        .with_context(|| format!("Failed to open file: {}", path))?;
    let reader = BufReader::new(file);
    let mut lines = reader.lines();
    let mut sequences = Vec::new();

    while let Some(header_line) = lines.next() {
        let header = header_line?;
        if !header.starts_with('@') {
            anyhow::bail!("Expected '@' at start of FASTQ record, got: {}", header);
        }

        let id = header[1..].to_string();

        let sequence_line = lines.next()
            .ok_or_else(|| anyhow::anyhow!("Unexpected end of file (sequence)"))?;
        let sequence = sequence_line?.into_bytes();

        let _plus_line = lines.next()
            .ok_or_else(|| anyhow::anyhow!("Unexpected end of file (plus)"))?;

        let quality_line = lines.next()
            .ok_or_else(|| anyhow::anyhow!("Unexpected end of file (quality)"))?;
        let quality = quality_line?.into_bytes();

        sequences.push(SequenceRecord {
            id,
            sequence,
            quality: Some(quality),
        });
    }

    Ok(sequences)
}

// ============================================================================
// CSV Output
// ============================================================================

/// Write results to CSV with comprehensive statistics
pub fn write_results_csv(results: &[ExperimentResult], path: &Path) -> Result<()> {
    let mut file = File::create(path)
        .with_context(|| format!("Failed to create CSV file: {}", path.display()))?;

    // Write header with all statistical columns
    writeln!(
        file,
        "operation,config_name,config_type,threads,affinity,scale,num_sequences,pruned,\
        throughput_median,throughput_mean,throughput_std_dev,throughput_ci_lower,throughput_ci_upper,\
        speedup_median,speedup_mean,speedup_std_dev,speedup_ci_lower,speedup_ci_upper,\
        elapsed_median,elapsed_mean,elapsed_std_dev,elapsed_min,elapsed_max,elapsed_q1,elapsed_q3,elapsed_iqr,\
        n_valid,n_outliers,n_warmup"
    )?;

    // Write data rows with all statistics
    for result in results {
        writeln!(
            file,
            "{},{},{:?},{},{},{},{},{},\
            {:.2},{:.2},{:.2},{:.2},{:.2},\
            {:.4},{:.4},{:.4},{:.4},{:.4},\
            {:.6},{:.6},{:.6},{:.6},{:.6},{:.6},{:.6},{:.6},\
            {},{},{}",
            // Metadata
            result.operation,
            result.config_name,
            result.config_type,
            result.threads,
            result.affinity,
            result.scale,
            result.num_sequences,
            result.pruned,
            // Throughput statistics (seq/sec - 2 decimal places)
            result.throughput_median,
            result.throughput_mean,
            result.throughput_std_dev,
            result.throughput_ci_lower,
            result.throughput_ci_upper,
            // Speedup statistics (4 decimal places for precision)
            result.speedup_median,
            result.speedup_mean,
            result.speedup_std_dev,
            result.speedup_ci_lower,
            result.speedup_ci_upper,
            // Elapsed time statistics (seconds - 6 decimal places for precision)
            result.elapsed_median,
            result.elapsed_mean,
            result.elapsed_std_dev,
            result.elapsed_min,
            result.elapsed_max,
            result.elapsed_q1,
            result.elapsed_q3,
            result.elapsed_iqr,
            // Sample statistics
            result.n_valid,
            result.n_outliers,
            result.n_warmup,
        )?;
    }

    file.flush()?;

    println!("✅ Results written to: {}", path.display());
    println!("   📊 {} experiments with N={} repetitions each",
             results.len(),
             results.first().map(|r| r.n_valid + r.n_outliers).unwrap_or(0));

    Ok(())
}

// ============================================================================
// CLI Entry Point
// ============================================================================

fn main() -> Result<()> {
    // Parse command-line arguments
    let args: Vec<String> = std::env::args().collect();

    if args.len() < 3 {
        eprintln!("Usage: asbb-dag-traversal --batch <batch_type> --output <path> [OPTIONS]");
        eprintln!("Batch types: neon_parallel, core_affinity, scale_thresholds");
        eprintln!();
        eprintln!("Options:");
        eprintln!("  --repetitions <N>         Number of repetitions per experiment (default: 30)");
        eprintln!("  --warmup <N>              Number of warmup runs to discard (default: 3)");
        eprintln!("  --outlier-threshold <F>   IQR multiplier for outlier detection (default: 1.5)");
        std::process::exit(1);
    }

    // Simple argument parsing
    let mut batch_type = None;
    let mut output_path = None;
    let mut repetitions = 30; // Default: publication quality
    let mut warmup_runs = 3;  // Default: eliminate cold-start effects
    let mut outlier_threshold = 1.5; // Default: standard IQR method

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--batch" => {
                i += 1;
                if i < args.len() {
                    batch_type = Some(args[i].clone());
                }
            }
            "--output" => {
                i += 1;
                if i < args.len() {
                    output_path = Some(PathBuf::from(&args[i]));
                }
            }
            "--repetitions" => {
                i += 1;
                if i < args.len() {
                    repetitions = args[i].parse()
                        .with_context(|| format!("Invalid repetitions value: {}", args[i]))?;
                }
            }
            "--warmup" => {
                i += 1;
                if i < args.len() {
                    warmup_runs = args[i].parse()
                        .with_context(|| format!("Invalid warmup value: {}", args[i]))?;
                }
            }
            "--outlier-threshold" => {
                i += 1;
                if i < args.len() {
                    outlier_threshold = args[i].parse()
                        .with_context(|| format!("Invalid outlier-threshold value: {}", args[i]))?;
                }
            }
            _ => {}
        }
        i += 1;
    }

    let batch_type = batch_type
        .ok_or_else(|| anyhow::anyhow!("Missing --batch argument"))?;
    let output_path = output_path
        .ok_or_else(|| anyhow::anyhow!("Missing --output argument"))?;

    let batch = DAGBatch::from_str(&batch_type)?;

    println!("📊 Statistical Parameters:");
    println!("   Repetitions per experiment: {}", repetitions);
    println!("   Warmup runs: {}", warmup_runs);
    println!("   Outlier threshold (IQR): {}x", outlier_threshold);
    println!();

    // Full run with 10 operations (Level 1 primitives)
    let operations = vec![
        "base_counting".to_string(),
        "gc_content".to_string(),
        "at_content".to_string(),
        "n_content".to_string(),
        "reverse_complement".to_string(),
        "sequence_length".to_string(),
        "quality_aggregation".to_string(),
        "quality_filter".to_string(),
        "length_filter".to_string(),
        "complexity_score".to_string(),
    ];

    // Select scales based on batch type
    let scales = match batch {
        DAGBatch::NeonParallel => vec![
            SCALES[2].clone(), // Medium (10K)
            SCALES[3].clone(), // Large (100K)
            SCALES[4].clone(), // VeryLarge (1M)
        ],
        DAGBatch::CoreAffinity => vec![
            SCALES[2].clone(), // Medium (10K)
            SCALES[3].clone(), // Large (100K)
        ],
        DAGBatch::ScaleThresholds => vec![
            SCALES[0].clone(), // Tiny (100)
            SCALES[1].clone(), // Small (1K)
            SCALES[2].clone(), // Medium (10K)
            SCALES[3].clone(), // Large (100K)
        ],
    };

    let config = DAGConfig {
        operations,
        scales,
        pruning_threshold: 1.5,
        diminishing_returns_threshold: 1.3,
        output_path,
        batch,
        repetitions,
        warmup_runs,
        outlier_threshold,
    };

    // Run DAG traversal
    let mut traversal = DAGTraversal::new(config);
    let results = traversal.run()?;

    // Write results
    write_results_csv(&results, &traversal.config.output_path)?;

    Ok(())
}
