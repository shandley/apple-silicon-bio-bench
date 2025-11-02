//! Composition Validation Pilot
//!
//! **Critical Experiment**: Tests if optimization rules from individual dimension pilots
//! compose correctly when combined.
//!
//! **Research Questions**:
//! 1. Do NEON and Parallel speedups multiply (compositional)?
//! 2. Does GPU add benefit on top of NEON+Parallel?
//! 3. Can we predict combined performance from individual pilot data?
//! 4. Do optimizations interfere or synergize?
//!
//! **Design**:
//! - 10 operations (complexity 0.20-0.70)
//! - 4 backends: Naive, NEON, NEON+Parallel, NEON+Parallel+GPU
//! - 5 scales: Tiny (100), Small (1K), Medium (10K), Large (100K), VeryLarge (1M)
//! - Total: 165 experiments (7 ops × 3 backends + 3 ops × 4 backends) × 5 scales
//!
//! **Run**:
//! ```bash
//! cargo run --release -p asbb-cli --bin asbb-pilot-composition > results/composition_validation/composition_raw.csv
//! ```

use anyhow::Result;
use asbb_core::{PrimitiveOperation, SequenceRecord};
use asbb_ops::{
    at_content::ATContent,
    base_counting::BaseCounting,
    complexity_score::ComplexityScore,
    gc_content::GcContent,
    n_content::NContent,
    quality_aggregation::QualityAggregation,
    quality_filter::QualityFilter,
    quality_statistics::QualityStatistics,
    reverse_complement::ReverseComplement,
    sequence_length::SequenceLength,
    sequence_masking::SequenceMasking,
};
use rayon::prelude::*;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;
use std::time::Instant;

/// Backend configuration for composition testing
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Backend {
    /// Naive baseline (no optimizations)
    Naive,
    /// NEON SIMD only
    Neon,
    /// NEON + Parallel (4 threads)
    NeonParallel,
    /// NEON + Parallel + GPU (where applicable)
    NeonParallelGpu,
}

impl Backend {
    fn name(&self) -> &'static str {
        match self {
            Backend::Naive => "naive",
            Backend::Neon => "neon",
            Backend::NeonParallel => "neon_parallel",
            Backend::NeonParallelGpu => "neon_parallel_gpu",
        }
    }

    fn description(&self) -> &'static str {
        match self {
            Backend::Naive => "Baseline scalar",
            Backend::Neon => "NEON SIMD only",
            Backend::NeonParallel => "NEON + 4 threads",
            Backend::NeonParallelGpu => "NEON + 4 threads + GPU",
        }
    }
}

/// Dataset scale definition
#[derive(Debug, Clone)]
struct Scale {
    name: &'static str,
    path: &'static str,
    num_sequences: usize,
}

const SCALES: &[Scale] = &[
    Scale { name: "Tiny", path: "datasets/tiny_100_150bp.fq", num_sequences: 100 },
    Scale { name: "Small", path: "datasets/small_1k_150bp.fq", num_sequences: 1_000 },
    Scale { name: "Medium", path: "datasets/medium_10k_150bp.fq", num_sequences: 10_000 },
    Scale { name: "Large", path: "datasets/large_100k_150bp.fq", num_sequences: 100_000 },
    Scale { name: "VeryLarge", path: "datasets/vlarge_1m_150bp.fq", num_sequences: 1_000_000 },
    // Skip "Huge" (10M) due to memory constraints (identified in Level 1/2 attempt)
];

/// Operation metadata
#[derive(Debug, Clone)]
struct OperationInfo {
    name: &'static str,
    complexity: f64,
    supports_gpu: bool,
}

const OPERATIONS: &[OperationInfo] = &[
    OperationInfo { name: "base_counting", complexity: 0.20, supports_gpu: false },
    OperationInfo { name: "sequence_length", complexity: 0.25, supports_gpu: false },
    OperationInfo { name: "at_content", complexity: 0.30, supports_gpu: false },
    OperationInfo { name: "n_content", complexity: 0.35, supports_gpu: false },
    OperationInfo { name: "gc_content", complexity: 0.40, supports_gpu: false },
    OperationInfo { name: "quality_filter", complexity: 0.45, supports_gpu: false },
    OperationInfo { name: "reverse_complement", complexity: 0.50, supports_gpu: false },
    // GPU-capable operations (complexity ≥0.55)
    OperationInfo { name: "sequence_masking", complexity: 0.55, supports_gpu: true },
    OperationInfo { name: "quality_statistics", complexity: 0.65, supports_gpu: true },
    OperationInfo { name: "complexity_score", complexity: 0.70, supports_gpu: true },
];

/// Result of one experiment
#[derive(Debug)]
struct ExperimentResult {
    operation: String,
    complexity: f64,
    scale_name: String,
    num_sequences: usize,
    backend: Backend,
    time_ms: f64,
    throughput: f64,  // sequences/sec
}

impl ExperimentResult {
    fn to_csv_header() -> String {
        "operation,complexity,scale,num_sequences,backend,time_ms,throughput_seqs_per_sec".to_string()
    }

    fn to_csv(&self) -> String {
        format!(
            "{},{:.2},{},{},{},{:.6},{:.2}",
            self.operation,
            self.complexity,
            self.scale_name,
            self.num_sequences,
            self.backend.name(),
            self.time_ms,
            self.throughput
        )
    }
}

fn main() -> Result<()> {
    eprintln!("╔════════════════════════════════════════════════════════════════════╗");
    eprintln!("║         Composition Validation Pilot - CRITICAL EXPERIMENT         ║");
    eprintln!("║       Testing if Optimization Rules Compose Correctly             ║");
    eprintln!("╚════════════════════════════════════════════════════════════════════╝");
    eprintln!();

    // Calculate total experiments
    let gpu_ops = OPERATIONS.iter().filter(|op| op.supports_gpu).count();
    let non_gpu_ops = OPERATIONS.len() - gpu_ops;
    let total_experiments = (non_gpu_ops * 3 + gpu_ops * 4) * SCALES.len();

    eprintln!("🔬 Testing: {} experiments", total_experiments);
    eprintln!("   - {} operations (complexity 0.20-0.70)", OPERATIONS.len());
    eprintln!("   - 3-4 backends per operation (Naive, NEON, NEON+Parallel, +GPU if applicable)");
    eprintln!("   - {} scales (100 → 1M sequences)", SCALES.len());
    eprintln!();

    eprintln!("🎯 Goal: Validate that NEON × Parallel ≈ (NEON speedup) × (Parallel speedup)");
    eprintln!("📊 Output: Composition ratios (actual / predicted speedup)");
    eprintln!();

    eprintln!("⏰ Estimated runtime: 2-3 hours (automated)");
    eprintln!();

    // Print CSV header
    println!("{}", ExperimentResult::to_csv_header());

    let mut experiment_count = 0;

    // Run experiments
    for op_info in OPERATIONS {
        for scale in SCALES {
            // Load dataset (shared across all backends for this operation/scale)
            let data = load_sequences(scale.path)?;

            // Determine which backends to test for this operation
            let backends = if op_info.supports_gpu {
                vec![Backend::Naive, Backend::Neon, Backend::NeonParallel, Backend::NeonParallelGpu]
            } else {
                vec![Backend::Naive, Backend::Neon, Backend::NeonParallel]
            };

            for backend in backends {
                experiment_count += 1;
                eprintln!("[{}/{}] {} @ {} with {:?}...",
                    experiment_count, total_experiments, op_info.name, scale.name, backend);

                let result = run_experiment(op_info, scale, backend, &data)?;
                println!("{}", result.to_csv());
            }
        }
    }

    eprintln!();
    eprintln!("✅ Composition Validation complete: {} experiments", experiment_count);
    eprintln!("📁 Results written to stdout (redirect to CSV file)");
    eprintln!();
    eprintln!("Next: Analyze composition ratios (actual vs predicted speedup)");

    Ok(())
}

/// Run a single experiment with specified backend
fn run_experiment(
    op_info: &OperationInfo,
    scale: &Scale,
    backend: Backend,
    data: &[SequenceRecord],
) -> Result<ExperimentResult> {
    // Warmup (1 run)
    let _ = execute_operation(op_info.name, backend, data)?;

    // Measure (3 runs, take median)
    let mut times = Vec::new();
    for _ in 0..3 {
        let start = Instant::now();
        let _ = execute_operation(op_info.name, backend, data)?;
        let duration = start.elapsed();
        times.push(duration.as_secs_f64() * 1000.0);  // Convert to ms
    }

    times.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let median_time_ms = times[1];  // Median of 3 runs
    let throughput = (scale.num_sequences as f64) / (median_time_ms / 1000.0);

    Ok(ExperimentResult {
        operation: op_info.name.to_string(),
        complexity: op_info.complexity,
        scale_name: scale.name.to_string(),
        num_sequences: scale.num_sequences,
        backend,
        time_ms: median_time_ms,
        throughput,
    })
}

/// Execute operation with specified backend
fn execute_operation(
    op_name: &str,
    backend: Backend,
    data: &[SequenceRecord],
) -> Result<()> {
    match op_name {
        "base_counting" => {
            let op = BaseCounting::new();
            match backend {
                Backend::Naive => { op.execute_naive(data)?; },
                Backend::Neon => { op.execute_neon(data)?; },
                Backend::NeonParallel => { op.execute_neon_parallel(data)?; },
                Backend::NeonParallelGpu => panic!("GPU not supported for base_counting"),
            }
        },
        "sequence_length" => {
            let op = SequenceLength::new();
            match backend {
                Backend::Naive => { op.execute_naive(data)?; },
                Backend::Neon => { op.execute_neon(data)?; },
                Backend::NeonParallel => { op.execute_neon_parallel(data)?; },
                Backend::NeonParallelGpu => panic!("GPU not supported for sequence_length"),
            }
        },
        "at_content" => {
            let op = ATContent::new();
            match backend {
                Backend::Naive => { op.execute_naive(data)?; },
                Backend::Neon => { op.execute_neon(data)?; },
                Backend::NeonParallel => { op.execute_neon_parallel(data)?; },
                Backend::NeonParallelGpu => panic!("GPU not supported for at_content"),
            }
        },
        "n_content" => {
            let op = NContent::new();
            match backend {
                Backend::Naive => { op.execute_naive(data)?; },
                Backend::Neon => { op.execute_neon(data)?; },
                Backend::NeonParallel => { op.execute_neon_parallel(data)?; },
                Backend::NeonParallelGpu => panic!("GPU not supported for n_content"),
            }
        },
        "gc_content" => {
            let op = GcContent::new();
            match backend {
                Backend::Naive => { op.execute_naive(data)?; },
                Backend::Neon => { op.execute_neon(data)?; },
                Backend::NeonParallel => { op.execute_neon_parallel(data)?; },
                Backend::NeonParallelGpu => panic!("GPU not supported for gc_content"),
            }
        },
        "quality_filter" => {
            let op = QualityFilter::new(20);
            match backend {
                Backend::Naive => { op.execute_naive(data)?; },
                Backend::Neon => { op.execute_neon(data)?; },
                Backend::NeonParallel => { op.execute_neon_parallel(data)?; },
                Backend::NeonParallelGpu => panic!("GPU not supported for quality_filter"),
            }
        },
        "reverse_complement" => {
            let op = ReverseComplement::new();
            match backend {
                Backend::Naive => { op.execute_naive(data)?; },
                Backend::Neon => { op.execute_neon(data)?; },
                Backend::NeonParallel => { op.execute_neon_parallel(data)?; },
                Backend::NeonParallelGpu => panic!("GPU not supported for reverse_complement"),
            }
        },
        "sequence_masking" => {
            let op = SequenceMasking::new();
            match backend {
                Backend::Naive => { op.execute_naive(data)?; },
                Backend::Neon => { op.execute_neon(data)?; },
                Backend::NeonParallel => { op.execute_neon_parallel(data)?; },
                Backend::NeonParallelGpu => { op.execute_gpu(data, 10000)?; },  // GPU batch size from pilot
            }
        },
        "quality_statistics" => {
            let op = QualityStatistics::new();
            match backend {
                Backend::Naive => { op.execute_naive(data)?; },
                Backend::Neon => { op.execute_neon(data)?; },
                Backend::NeonParallel => { op.execute_neon_parallel(data)?; },
                Backend::NeonParallelGpu => { op.execute_gpu(data, 10000)?; },
            }
        },
        "complexity_score" => {
            let op = ComplexityScore::new();
            match backend {
                Backend::Naive => { op.execute_naive(data)?; },
                Backend::Neon => { op.execute_neon(data)?; },
                Backend::NeonParallel => { op.execute_neon_parallel(data)?; },
                Backend::NeonParallelGpu => { op.execute_gpu(data, 10000)?; },
            }
        },
        _ => panic!("Unknown operation: {}", op_name),
    }

    Ok(())
}

/// Load sequences from FASTQ file
fn load_sequences(path: &str) -> Result<Vec<SequenceRecord>> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);
    let mut lines = reader.lines();
    let mut records = Vec::new();

    while let Some(header) = lines.next() {
        let header = header?;
        if !header.starts_with('@') {
            continue;
        }

        let sequence = lines.next().ok_or_else(|| anyhow::anyhow!("Missing sequence"))??;
        let _plus = lines.next(); // Skip '+' line
        let quality = lines.next().ok_or_else(|| anyhow::anyhow!("Missing quality"))??;

        records.push(SequenceRecord {
            id: header[1..].to_string(),  // Remove '@'
            sequence: sequence.into_bytes(),
            quality: Some(quality.into_bytes()),
        });
    }

    Ok(records)
}

// ============================================================================
// NEON+Parallel Backend Implementations
// ============================================================================
//
// These traits extend operations with combined NEON+Parallel execution.
// We combine NEON vectorization within chunks and Rayon parallelism across chunks.

trait NeonParallelExecution: PrimitiveOperation {
    /// Execute with NEON SIMD + Rayon parallel (4 threads)
    fn execute_neon_parallel(&self, data: &[SequenceRecord]) -> Result<asbb_core::OperationOutput> {
        // Set thread count to 4 (consistent with parallel dimension pilot)
        rayon::ThreadPoolBuilder::new()
            .num_threads(4)
            .build_global()
            .ok();  // Ignore error if already initialized

        // For most operations: parallel over sequences, NEON within each
        // This is the default composition pattern
        self.execute_parallel(data, 4)
    }
}

// Implement for all operations
impl NeonParallelExecution for BaseCounting {}
impl NeonParallelExecution for SequenceLength {}
impl NeonParallelExecution for ATContent {}
impl NeonParallelExecution for NContent {}
impl NeonParallelExecution for GcContent {}
impl NeonParallelExecution for QualityFilter {}
impl NeonParallelExecution for ReverseComplement {}
impl NeonParallelExecution for SequenceMasking {}
impl NeonParallelExecution for QualityStatistics {}
impl NeonParallelExecution for ComplexityScore {}
impl NeonParallelExecution for QualityAggregation {}
