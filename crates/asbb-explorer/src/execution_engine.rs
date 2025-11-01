//! Execution Engine - Automated experiment runner for Level 1/2
//!
//! The ExecutionEngine orchestrates running thousands of experiments in parallel,
//! with checkpointing, progress tracking, and result storage.
//!
//! # Architecture
//!
//! 1. **Configuration Loading**: Parse config.toml (TOML → ExperimentConfig)
//! 2. **Experiment Generation**: Cartesian product of operations × configs × scales
//! 3. **Parallel Execution**: Rayon thread pool (8 concurrent experiments)
//! 4. **Checkpointing**: Save progress every 100 experiments (resume capability)
//! 5. **Result Storage**: Store in Parquet format (efficient columnar storage)
//! 6. **Progress Tracking**: indicatif progress bars
//!
//! # Usage
//!
//! ```ignore
//! let engine = ExecutionEngine::from_config_file("config.toml")?;
//! engine.run_all()?;
//! ```

use anyhow::{Context, Result};
use asbb_core::operation_registry::OperationRegistry;
use asbb_core::{
    HardwareConfig, QualityOfService, SequenceRecord, ThreadAssignment,
};
use indicatif::{ProgressBar, ProgressStyle};
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

// ============================================================================
// Configuration Types (matches config.toml structure)
// ============================================================================

/// Top-level experiment configuration
#[derive(Debug, Clone, Deserialize)]
pub struct ExperimentConfig {
    pub metadata: ExperimentMetadata,
    pub datasets: DatasetsConfig,
    pub operations: OperationsConfig,
    pub hardware: HardwareConfigList,
    pub execution: ExecutionSettings,
    pub output: OutputSettings,
    pub analysis: AnalysisSettings,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ExperimentMetadata {
    pub name: String,
    pub description: String,
    pub version: String,
    pub total_experiments: usize,
    pub target_completion: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DatasetsConfig {
    pub sequence_length: usize,
    pub quality_encoding: String,
    pub seed: u64,
    pub scales: Vec<DatasetScale>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DatasetScale {
    pub name: String,
    pub sequences: usize,
    pub description: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct OperationsConfig {
    pub list: Vec<OperationConfigEntry>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct OperationConfigEntry {
    pub name: String,
    pub category: String,
    pub complexity: f64,
    pub implemented: bool,
    pub backends: Vec<String>,
    #[serde(default)]
    pub description: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct HardwareConfigList {
    pub configs: Vec<HardwareConfigEntry>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct HardwareConfigEntry {
    pub id: String,
    pub description: String,
    pub use_neon: bool,
    pub num_threads: usize,
    pub thread_assignment: String,
    pub encoding: String,
    pub use_gpu: bool,
    #[serde(default)]
    pub gpu_batch_size: Option<usize>,
    #[serde(default)]
    pub use_2bit: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ExecutionSettings {
    pub parallel_experiments: usize,
    pub checkpoint_interval: usize,
    pub timeout_seconds: u64,
    pub warmup_runs: usize,
    pub measurement_runs: usize,
    pub validate_correctness: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct OutputSettings {
    pub results_dir: String,
    pub parquet_file: String,
    pub checkpoint_file: String,
    pub log_file: String,
    pub progress_bar: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AnalysisSettings {
    pub train_test_split: f64,
    pub cross_validation_folds: usize,
    pub confidence_level: f64,
    pub target_prediction_accuracy: f64,
    pub target_r_squared: f64,
}

// ============================================================================
// Experiment Definition
// ============================================================================

/// A single experiment to run
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Experiment {
    /// Unique experiment ID
    pub id: String,

    /// Operation name
    pub operation: String,

    /// Hardware configuration ID
    pub hardware_config_id: String,

    /// Dataset scale name
    pub scale: String,

    /// Number of sequences in dataset
    pub num_sequences: usize,
}

/// Result from a single experiment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExperimentResult {
    /// Experiment ID
    pub experiment_id: String,

    /// Operation name
    pub operation: String,

    /// Operation category
    pub operation_category: String,

    /// Operation complexity score
    pub operation_complexity: f64,

    /// Hardware configuration ID
    pub hardware_config_id: String,

    /// Hardware configuration description
    pub hardware_description: String,

    /// Dataset scale name
    pub scale: String,

    /// Number of sequences
    pub num_sequences: usize,

    /// Sequence length
    pub sequence_length: usize,

    /// Mean execution time (seconds)
    pub mean_time_seconds: f64,

    /// Median execution time (seconds)
    pub median_time_seconds: f64,

    /// Standard deviation of execution time
    pub std_time_seconds: f64,

    /// Throughput (sequences/second)
    pub throughput_seqs_per_sec: f64,

    /// Throughput (MB/s)
    pub throughput_mbps: f64,

    /// Peak memory usage (bytes)
    pub memory_peak_bytes: usize,

    /// Average memory usage (bytes)
    pub memory_avg_bytes: usize,

    /// CPU utilization (0-1 per core, can exceed 1)
    pub cpu_utilization: f64,

    /// GPU utilization (0-1, if used)
    pub gpu_utilization: Option<f64>,

    /// Energy consumed (joules, if measurable)
    pub energy_joules: Option<f64>,

    /// Output matches reference (correctness)
    pub correct: bool,

    /// Timestamp
    pub timestamp: String,
}

// ============================================================================
// Checkpoint Management
// ============================================================================

/// Checkpoint state for resuming interrupted runs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Checkpoint {
    /// Experiment IDs that have completed
    pub completed: Vec<String>,

    /// Total number of experiments
    pub total: usize,

    /// Timestamp of last checkpoint
    pub last_updated: String,
}

impl Checkpoint {
    pub fn new(total: usize) -> Self {
        Self {
            completed: Vec::new(),
            total,
            last_updated: chrono::Utc::now().to_rfc3339(),
        }
    }

    pub fn is_completed(&self, id: &str) -> bool {
        self.completed.contains(&id.to_string())
    }

    pub fn mark_completed(&mut self, id: String) {
        if !self.completed.contains(&id) {
            self.completed.push(id);
            self.last_updated = chrono::Utc::now().to_rfc3339();
        }
    }

    pub fn load(path: &Path) -> Result<Self> {
        let contents = fs::read_to_string(path)?;
        let checkpoint: Self = serde_json::from_str(&contents)?;
        Ok(checkpoint)
    }

    pub fn save(&self, path: &Path) -> Result<()> {
        let contents = serde_json::to_string_pretty(self)?;
        fs::write(path, contents)?;
        Ok(())
    }
}

// ============================================================================
// Execution Engine
// ============================================================================

/// Main execution engine for automated experiments
pub struct ExecutionEngine {
    /// Experiment configuration
    config: ExperimentConfig,

    /// Operation registry
    registry: Arc<OperationRegistry>,

    /// All experiments to run
    experiments: Vec<Experiment>,

    /// Collected results (thread-safe)
    results: Arc<Mutex<Vec<ExperimentResult>>>,

    /// Checkpoint state (thread-safe)
    checkpoint: Arc<Mutex<Checkpoint>>,

    /// Output directory
    output_dir: PathBuf,
}

impl ExecutionEngine {
    /// Create engine from config file
    pub fn from_config_file<P: AsRef<Path>>(
        config_path: P,
        registry: OperationRegistry,
    ) -> Result<Self> {
        let config_str = fs::read_to_string(config_path.as_ref())
            .context("Failed to read config file")?;
        let config: ExperimentConfig = toml::from_str(&config_str)
            .context("Failed to parse config TOML")?;

        Self::from_config(config, registry)
    }

    /// Create engine from config struct
    pub fn from_config(config: ExperimentConfig, registry: OperationRegistry) -> Result<Self> {
        // Generate all experiments
        let experiments = Self::generate_experiments(&config)?;
        let total = experiments.len();

        // Create output directory
        let output_dir = PathBuf::from(&config.output.results_dir);
        fs::create_dir_all(&output_dir)?;

        // Load or create checkpoint
        let checkpoint_path = output_dir.join(&config.output.checkpoint_file);
        let checkpoint = if checkpoint_path.exists() {
            Checkpoint::load(&checkpoint_path)?
        } else {
            Checkpoint::new(total)
        };

        Ok(Self {
            config,
            registry: Arc::new(registry),
            experiments,
            results: Arc::new(Mutex::new(Vec::new())),
            checkpoint: Arc::new(Mutex::new(checkpoint)),
            output_dir,
        })
    }

    /// Generate all experiment combinations
    fn generate_experiments(config: &ExperimentConfig) -> Result<Vec<Experiment>> {
        let mut experiments = Vec::new();
        let mut experiment_id = 0;

        // Filter to only implemented operations
        let implemented_ops: Vec<_> = config
            .operations
            .list
            .iter()
            .filter(|op| op.implemented)
            .collect();

        for operation in &implemented_ops {
            for hardware in &config.hardware.configs {
                for scale in &config.datasets.scales {
                    experiment_id += 1;
                    experiments.push(Experiment {
                        id: format!("exp_{:06}", experiment_id),
                        operation: operation.name.clone(),
                        hardware_config_id: hardware.id.clone(),
                        scale: scale.name.clone(),
                        num_sequences: scale.sequences,
                    });
                }
            }
        }

        Ok(experiments)
    }

    /// Run all experiments
    pub fn run_all(&self) -> Result<()> {
        let total = self.experiments.len();
        let checkpoint = self.checkpoint.lock().unwrap();
        let completed_count = checkpoint.completed.len();
        drop(checkpoint);

        println!("Starting experiment execution:");
        println!("  Total experiments: {}", total);
        println!("  Already completed: {}", completed_count);
        println!("  Remaining: {}", total - completed_count);
        println!("  Parallel workers: {}", self.config.execution.parallel_experiments);

        // Filter to only incomplete experiments
        eprintln!("DEBUG: Filtering incomplete experiments...");
        let incomplete: Vec<_> = self
            .experiments
            .iter()
            .filter(|exp| {
                let checkpoint = self.checkpoint.lock().unwrap();
                !checkpoint.is_completed(&exp.id)
            })
            .cloned()
            .collect();
        eprintln!("DEBUG: Filtered {} incomplete experiments", incomplete.len());

        if incomplete.is_empty() {
            println!("All experiments already completed!");
            return Ok(());
        }

        // Create progress bar
        eprintln!("DEBUG: Creating progress bar...");
        let progress = if self.config.output.progress_bar {
            let pb = ProgressBar::new(incomplete.len() as u64);
            pb.set_style(
                ProgressStyle::default_bar()
                    .template(
                        "[{elapsed_precise}] {bar:40.cyan/blue} {pos}/{len} {msg}",
                    )?
                    .progress_chars("=>-"),
            );
            Some(pb)
        } else {
            None
        };
        eprintln!("DEBUG: Progress bar created");

        // Set up Rayon thread pool
        eprintln!("DEBUG: Creating Rayon thread pool with {} threads...", self.config.execution.parallel_experiments);
        let pool = rayon::ThreadPoolBuilder::new()
            .num_threads(self.config.execution.parallel_experiments)
            .build()?;
        eprintln!("DEBUG: Thread pool created successfully");

        // Execute experiments in parallel
        eprintln!("DEBUG: Starting parallel execution...");
        let checkpoint_interval = self.config.execution.checkpoint_interval;
        let results_ref = Arc::clone(&self.results);
        let checkpoint_ref = Arc::clone(&self.checkpoint);
        let registry_ref = Arc::clone(&self.registry);
        let config_clone = self.config.clone();

        pool.install(|| {
            eprintln!("DEBUG: Inside pool.install, about to start par_iter...");
            incomplete
                .par_iter()
                .enumerate()
                .try_for_each(|(i, experiment)| -> Result<()> {
                    if i == 0 {
                        eprintln!("DEBUG: Starting first experiment: {} with {}",
                            experiment.operation, experiment.hardware_config_id);
                    }
                    // Run experiment
                    let result = self.run_experiment(
                        experiment,
                        &registry_ref,
                        &config_clone,
                    )?;
                    if i == 0 {
                        eprintln!("DEBUG: First experiment completed successfully");
                    }

                    // Store result
                    {
                        let mut results = results_ref.lock().unwrap();
                        results.push(result);
                    }

                    // Mark completed
                    {
                        let mut checkpoint = checkpoint_ref.lock().unwrap();
                        checkpoint.mark_completed(experiment.id.clone());
                    }

                    // Update progress
                    if let Some(ref pb) = progress {
                        pb.inc(1);
                        pb.set_message(format!(
                            "Running: {} with {}",
                            experiment.operation, experiment.hardware_config_id
                        ));
                    }

                    // Checkpoint periodically
                    if (i + 1) % checkpoint_interval == 0 {
                        let checkpoint = checkpoint_ref.lock().unwrap();
                        let checkpoint_path = self.output_dir.join(&self.config.output.checkpoint_file);
                        checkpoint.save(&checkpoint_path)?;
                    }

                    Ok(())
                })
        })?;

        // Final checkpoint save
        let checkpoint = self.checkpoint.lock().unwrap();
        let checkpoint_path = self.output_dir.join(&self.config.output.checkpoint_file);
        checkpoint.save(&checkpoint_path)?;

        if let Some(ref pb) = progress {
            pb.finish_with_message("All experiments complete!");
        }

        // Save results
        self.save_results()?;

        println!("\nExecution complete!");
        println!("  Results saved to: {}", self.output_dir.display());

        Ok(())
    }

    /// Run a single experiment
    fn run_experiment(
        &self,
        experiment: &Experiment,
        registry: &Arc<OperationRegistry>,
        config: &ExperimentConfig,
    ) -> Result<ExperimentResult> {
        eprintln!("DEBUG: run_experiment called for {} with {}",
            experiment.operation, experiment.hardware_config_id);

        // Get operation from registry
        eprintln!("DEBUG: Getting operation from registry...");
        let operation = registry.get(&experiment.operation)?;
        let metadata = registry.get_metadata(&experiment.operation)?;
        eprintln!("DEBUG: Operation retrieved successfully");

        // Generate test data
        eprintln!("DEBUG: Generating test data ({} sequences)...", experiment.num_sequences);
        let data = self.generate_test_data(experiment, config)?;
        eprintln!("DEBUG: Test data generated");

        // Convert hardware config
        let hw_config = self.create_hardware_config(experiment, config)?;

        // Get hardware description
        let hw_entry = config
            .hardware
            .configs
            .iter()
            .find(|c| c.id == experiment.hardware_config_id)
            .context("Hardware config not found")?;

        // Run benchmark
        let perf_result = crate::benchmark_operation(
            operation.as_ref(),
            &data,
            &hw_config,
            config.execution.warmup_runs,
            config.execution.measurement_runs,
        )?;

        // Convert to ExperimentResult
        Ok(ExperimentResult {
            experiment_id: experiment.id.clone(),
            operation: experiment.operation.clone(),
            operation_category: format!("{:?}", metadata.category),
            operation_complexity: metadata.complexity,
            hardware_config_id: experiment.hardware_config_id.clone(),
            hardware_description: hw_entry.description.clone(),
            scale: experiment.scale.clone(),
            num_sequences: experiment.num_sequences,
            sequence_length: config.datasets.sequence_length,
            mean_time_seconds: perf_result.latency_p50.as_secs_f64(),
            median_time_seconds: perf_result.latency_p50.as_secs_f64(),
            std_time_seconds: 0.0, // TODO: Calculate from multiple runs
            throughput_seqs_per_sec: perf_result.throughput_seqs_per_sec,
            throughput_mbps: perf_result.throughput_mbps,
            memory_peak_bytes: perf_result.memory_peak,
            memory_avg_bytes: perf_result.memory_avg,
            cpu_utilization: perf_result.cpu_utilization,
            gpu_utilization: perf_result.gpu_utilization,
            energy_joules: perf_result.energy_joules,
            correct: perf_result.output_matches_reference,
            timestamp: chrono::Utc::now().to_rfc3339(),
        })
    }

    /// Generate test data for an experiment
    fn generate_test_data(
        &self,
        experiment: &Experiment,
        config: &ExperimentConfig,
    ) -> Result<Vec<SequenceRecord>> {
        use rand::{Rng, SeedableRng};
        use rand_chacha::ChaCha8Rng;

        let mut rng = ChaCha8Rng::seed_from_u64(config.datasets.seed);
        let mut data = Vec::with_capacity(experiment.num_sequences);

        for i in 0..experiment.num_sequences {
            let id = format!("seq_{}", i);
            let sequence: Vec<u8> = (0..config.datasets.sequence_length)
                .map(|_| match rng.gen_range(0..4) {
                    0 => b'A',
                    1 => b'C',
                    2 => b'G',
                    _ => b'T',
                })
                .collect();

            // Generate quality scores (FASTQ)
            let quality: Vec<u8> = (0..config.datasets.sequence_length)
                .map(|_| rng.gen_range(33..74)) // Phred+33 encoding (Q0-Q40)
                .collect();

            data.push(SequenceRecord::fastq(id, sequence, quality));
        }

        Ok(data)
    }

    /// Create HardwareConfig from experiment settings
    fn create_hardware_config(
        &self,
        experiment: &Experiment,
        config: &ExperimentConfig,
    ) -> Result<HardwareConfig> {
        let hw_entry = config
            .hardware
            .configs
            .iter()
            .find(|c| c.id == experiment.hardware_config_id)
            .context("Hardware config not found")?;

        let thread_assignment = match hw_entry.thread_assignment.as_str() {
            "default" => ThreadAssignment::Mixed,
            "p_cores" => ThreadAssignment::PCoresOnly,
            "e_cores" => ThreadAssignment::ECoresOnly,
            "mixed" | "mixed_2p2e" | "mixed_4p6e" => ThreadAssignment::Mixed,
            _ => ThreadAssignment::Mixed,
        };

        Ok(HardwareConfig {
            use_neon: hw_entry.use_neon,
            num_threads: hw_entry.num_threads,
            thread_assignment,
            encoding: asbb_core::Encoding::Ascii, // TODO: Support from config
            use_unified_memory: hw_entry.use_gpu, // If GPU, use unified memory
            use_gpu: hw_entry.use_gpu,
            gpu_batch_size: hw_entry.gpu_batch_size,
            use_amx: false, // Not yet implemented
            use_neural_engine: false, // Not yet implemented
            use_m5_gpu_neural_accel: false, // Not yet implemented
            use_hw_compression: false, // Not yet implemented
            use_gcd: false, // Use Rayon instead
            qos: QualityOfService::UserInitiated,
            chip_generation: None,
        })
    }

    /// Save results to Parquet file
    fn save_results(&self) -> Result<()> {
        let results = self.results.lock().unwrap();
        let json_path = self.output_dir.join("results.json");
        let json_str = serde_json::to_string_pretty(&*results)?;
        fs::write(&json_path, json_str)?;

        println!("  Saved {} results to {}", results.len(), json_path.display());

        // TODO: Implement Parquet storage
        // This requires converting Vec<ExperimentResult> to Arrow RecordBatch
        // and writing with parquet::arrow::ArrowWriter

        Ok(())
    }
}

// ============================================================================
// Helper Modules (to be added)
// ============================================================================

// TODO: Add Parquet conversion module
// TODO: Add statistical analysis module
