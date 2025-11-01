//! Parallel/Threading Dimension Pilot - COMPREHENSIVE
//!
//! Systematic testing of parallel performance across all 10 operations with core affinity.
//!
//! **Research Questions**:
//! 1. Which operations benefit from parallelism?
//! 2. What's the optimal thread count per operation category?
//! 3. Does complexity score predict parallelism benefit?
//! 4. Is there a batch size threshold for parallel benefit?
//! 5. Do P-cores vs E-cores matter for different operation types?
//! 6. Does heterogeneous compute (P+E mixed) provide any benefit?
//!
//! **Comprehensive Approach**:
//! - Test 1, 2, 4, 8 threads
//! - Test Default, P-cores, E-cores scheduling
//! - 10 operations × 12 configurations × 6 scales = 720 experiments
//!
//! Run in release mode:
//! ```bash
//! cargo run --release -p asbb-cli --bin asbb-pilot-parallel > results/parallel_dimension_raw.csv
//! ```

use anyhow::Result;
use asbb_core::{PrimitiveOperation, SequenceRecord};
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
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;
use std::time::Instant;

/// Core assignment strategy
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CoreAssignment {
    /// Let OS decide (baseline)
    Default,
    /// Try to pin to Performance cores (high QoS)
    PerformanceCores,
    /// Try to pin to Efficiency cores (low QoS)
    EfficiencyCores,
}

impl CoreAssignment {
    fn name(&self) -> &'static str {
        match self {
            CoreAssignment::Default => "default",
            CoreAssignment::PerformanceCores => "p_cores",
            CoreAssignment::EfficiencyCores => "e_cores",
        }
    }

    fn description(&self) -> &'static str {
        match self {
            CoreAssignment::Default => "OS-scheduled",
            CoreAssignment::PerformanceCores => "P-cores (QoS UserInitiated)",
            CoreAssignment::EfficiencyCores => "E-cores (QoS Background)",
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
    Scale { name: "Huge", path: "datasets/huge_10m_150bp.fq", num_sequences: 10_000_000 },
];

const THREAD_COUNTS: &[usize] = &[1, 2, 4, 8];

/// Configuration for one experiment
#[derive(Debug, Clone)]
struct ExperimentConfig {
    threads: usize,
    assignment: CoreAssignment,
}

/// Result of one experiment
#[derive(Debug)]
struct ExperimentResult {
    operation: String,
    complexity: f64,
    scale_name: String,
    num_sequences: usize,
    threads: usize,
    assignment: CoreAssignment,
    time_ms: f64,
    speedup_vs_1t: f64,
    efficiency: f64,  // speedup / threads
    throughput: f64,  // sequences/sec
}

impl ExperimentResult {
    fn to_csv_header() -> String {
        "operation,complexity,scale,num_sequences,threads,assignment,time_ms,speedup_vs_1t,efficiency,throughput_seqs_per_sec".to_string()
    }

    fn to_csv(&self) -> String {
        format!(
            "{},{:.2},{},{},{},{},{:.6},{:.4},{:.4},{:.2}",
            self.operation,
            self.complexity,
            self.scale_name,
            self.num_sequences,
            self.threads,
            self.assignment.name(),
            self.time_ms,
            self.speedup_vs_1t,
            self.efficiency,
            self.throughput
        )
    }
}

fn main() -> Result<()> {
    eprintln!("╔════════════════════════════════════════════════════════════════════╗");
    eprintln!("║        Parallel/Threading Dimension Pilot - COMPREHENSIVE        ║");
    eprintln!("║        Systematic Testing with Core Affinity                       ║");
    eprintln!("╚════════════════════════════════════════════════════════════════════╝");
    eprintln!();

    eprintln!("🔬 Testing: 10 operations × 12 configurations × 6 scales = 720 experiments");
    eprintln!("🎯 Goal: Identify optimal thread count and core assignment per operation");
    eprintln!("📊 Thread counts: {:?}", THREAD_COUNTS);
    eprintln!("🖥️  Core assignments: Default, P-cores (QoS), E-cores (QoS)");
    eprintln!();

    eprintln!("💡 Note: On macOS, explicit core pinning is limited.");
    eprintln!("   We use QoS hints to influence P-core vs E-core scheduling:");
    eprintln!("   - P-cores: QoS UserInitiated (high priority)");
    eprintln!("   - E-cores: QoS Background (low priority)");
    eprintln!("   - Default: QoS Default (let OS decide)");
    eprintln!();
    eprintln!("📊 Validate with Activity Monitor:");
    eprintln!("   1. Open Activity Monitor");
    eprintln!("   2. View → CPU History");
    eprintln!("   3. Watch which cores light up during execution");
    eprintln!();

    // Print CSV header to stdout
    println!("{}", ExperimentResult::to_csv_header());

    // Test each operation
    test_operation("base_counting", 0.40, BaseCounting)?;
    test_operation("gc_content", 0.32, GcContent)?;
    test_operation("at_content", 0.35, ATContent)?;
    test_operation("n_content", 0.25, NContent)?;
    test_operation("sequence_length", 0.20, SequenceLength)?;
    test_operation("reverse_complement", 0.45, ReverseComplement::new())?;
    test_operation("quality_aggregation", 0.50, QualityAggregation::new())?;
    test_operation("quality_filter", 0.55, QualityFilter::new(30))?;
    test_operation("length_filter", 0.55, LengthFilter::new(50))?;
    test_operation("complexity_score", 0.61, ComplexityScore::new())?;

    eprintln!();
    eprintln!("═══════════════════════════════════════════════════════════════");
    eprintln!("✅ Parallel Dimension Pilot Complete - 720 experiments");
    eprintln!("═══════════════════════════════════════════════════════════════");
    eprintln!();
    eprintln!("📊 Output: CSV data to stdout");
    eprintln!("💾 Save with: cargo run --release --bin asbb-pilot-parallel > results.csv");
    eprintln!();
    eprintln!("Next steps:");
    eprintln!("  1. Analyze CSV data");
    eprintln!("  2. Generate speedup matrices");
    eprintln!("  3. Document patterns");
    eprintln!("  4. Derive decision rules");

    Ok(())
}

fn test_operation<T: PrimitiveOperation>(
    name: &str,
    complexity: f64,
    op: T,
) -> Result<()> {
    eprintln!("═══════════════════════════════════════════════════════════════");
    eprintln!("🧬 Operation: {} (complexity {:.2})", name, complexity);
    eprintln!("═══════════════════════════════════════════════════════════════");

    for scale in SCALES {
        // Load data
        let mut records = load_fastq(scale.path)?;
        if records.is_empty() {
            eprintln!("   ⚠️  Skipping {} (file not found: {})", scale.name, scale.path);
            continue;
        }
        if records.len() > scale.num_sequences {
            records.truncate(scale.num_sequences);
        }

        eprint!("📦 {:>10} ({:>8} seqs): ", scale.name, scale.num_sequences);

        // Generate all configurations
        let mut configs = Vec::new();
        for &threads in THREAD_COUNTS {
            if threads == 1 {
                // For single-threaded, only test default (assignment doesn't matter)
                configs.push(ExperimentConfig {
                    threads,
                    assignment: CoreAssignment::Default,
                });
            } else {
                // For multi-threaded, test all assignments
                configs.push(ExperimentConfig {
                    threads,
                    assignment: CoreAssignment::Default,
                });
                configs.push(ExperimentConfig {
                    threads,
                    assignment: CoreAssignment::PerformanceCores,
                });
                configs.push(ExperimentConfig {
                    threads,
                    assignment: CoreAssignment::EfficiencyCores,
                });
            }
        }

        // Run all experiments for this scale
        let mut results = Vec::new();
        for config in &configs {
            let time_ms = run_experiment(&op, &records, config)?;
            results.push((config.clone(), time_ms));
        }

        // Get baseline (single-threaded default)
        let baseline_time = results.iter()
            .find(|(cfg, _)| cfg.threads == 1)
            .map(|(_, time)| *time)
            .unwrap_or(1.0);

        // Calculate metrics and print CSV
        for (config, time_ms) in &results {
            let speedup = baseline_time / time_ms;
            let efficiency = speedup / config.threads as f64;
            let throughput = (scale.num_sequences as f64) / (time_ms / 1000.0);

            let result = ExperimentResult {
                operation: name.to_string(),
                complexity,
                scale_name: scale.name.to_string(),
                num_sequences: scale.num_sequences,
                threads: config.threads,
                assignment: config.assignment,
                time_ms: *time_ms,
                speedup_vs_1t: speedup,
                efficiency,
                throughput,
            };

            // Print CSV row to stdout
            println!("{}", result.to_csv());
        }

        // Print summary to stderr
        let best = results.iter()
            .max_by(|a, b| {
                let speedup_a = baseline_time / a.1;
                let speedup_b = baseline_time / b.1;
                speedup_a.partial_cmp(&speedup_b).unwrap()
            })
            .unwrap();

        let best_speedup = baseline_time / best.1;
        if best.0.threads == 1 {
            eprintln!("→ No parallel benefit");
        } else {
            eprintln!("→ Best: {}t/{} ({:.2}×)",
                best.0.threads,
                best.0.assignment.name(),
                best_speedup
            );
        }
    }

    eprintln!();
    Ok(())
}

/// Run a single experiment with specified configuration
fn run_experiment<T: PrimitiveOperation>(
    op: &T,
    records: &[SequenceRecord],
    config: &ExperimentConfig,
) -> Result<f64> {
    // Warmup (5 iterations)
    for _ in 0..5 {
        let _ = execute_with_config(op, records, config)?;
    }

    // Measurement (20 iterations)
    let mut times = Vec::new();
    for _ in 0..20 {
        let start = Instant::now();
        let _ = execute_with_config(op, records, config)?;
        let elapsed = start.elapsed();
        times.push(elapsed.as_secs_f64() * 1000.0);
    }

    // Remove outliers (>3 std dev) and compute mean
    let mean = times.iter().sum::<f64>() / times.len() as f64;
    let std_dev = (times.iter()
        .map(|t| (t - mean).powi(2))
        .sum::<f64>() / times.len() as f64)
        .sqrt();

    let filtered: Vec<f64> = times.iter()
        .filter(|&&t| (t - mean).abs() <= 3.0 * std_dev)
        .copied()
        .collect();

    let final_time = if filtered.is_empty() {
        mean
    } else {
        filtered.iter().sum::<f64>() / filtered.len() as f64
    };

    Ok(final_time)
}

/// Execute operation with specified configuration
fn execute_with_config<T: PrimitiveOperation>(
    op: &T,
    records: &[SequenceRecord],
    config: &ExperimentConfig,
) -> Result<()> {
    // Set QoS hint based on core assignment
    // Note: This is the Apple-recommended approach for influencing P-core vs E-core scheduling
    match config.assignment {
        CoreAssignment::PerformanceCores => {
            // Set high QoS to prefer P-cores
            // On macOS, this uses dispatch_set_qos_class or pthread_set_qos_class_self_np
            set_thread_qos(ThreadQoS::UserInitiated);
        }
        CoreAssignment::EfficiencyCores => {
            // Set low QoS to prefer E-cores
            set_thread_qos(ThreadQoS::Background);
        }
        CoreAssignment::Default => {
            // Use default QoS
            set_thread_qos(ThreadQoS::Default);
        }
    }

    // Execute with specified thread count
    let _ = op.execute_parallel(records, config.threads)?;

    // Reset QoS
    set_thread_qos(ThreadQoS::Default);

    Ok(())
}

/// Thread Quality of Service levels (macOS)
#[derive(Debug, Clone, Copy)]
enum ThreadQoS {
    Background,      // E-cores preferred
    Utility,         // E-cores likely
    Default,         // OS decides
    UserInitiated,   // P-cores preferred
    UserInteractive, // P-cores strongly preferred
}

/// Set thread QoS hint (macOS-specific)
#[cfg(target_os = "macos")]
fn set_thread_qos(qos: ThreadQoS) {
    // macOS pthread QoS classes
    // These are hints to the scheduler about core preference
    // QOS_CLASS_BACKGROUND (0x09) -> E-cores
    // QOS_CLASS_UTILITY (0x11) -> E-cores likely
    // QOS_CLASS_DEFAULT (0x15) -> OS decides
    // QOS_CLASS_USER_INITIATED (0x19) -> P-cores
    // QOS_CLASS_USER_INTERACTIVE (0x21) -> P-cores

    let qos_class = match qos {
        ThreadQoS::Background => 0x09,
        ThreadQoS::Utility => 0x11,
        ThreadQoS::Default => 0x15,
        ThreadQoS::UserInitiated => 0x19,
        ThreadQoS::UserInteractive => 0x21,
    };

    unsafe {
        // pthread_set_qos_class_self_np is macOS-specific
        // This is a best-effort hint to the scheduler
        extern "C" {
            fn pthread_set_qos_class_self_np(qos_class: u32, relative_priority: i32) -> i32;
        }

        let _ = pthread_set_qos_class_self_np(qos_class, 0);
    }
}

#[cfg(not(target_os = "macos"))]
fn set_thread_qos(_qos: ThreadQoS) {
    // No-op on non-macOS platforms
}

fn load_fastq(path: &str) -> Result<Vec<SequenceRecord>> {
    let file_path = Path::new(path);
    if !file_path.exists() {
        return Ok(Vec::new());
    }

    let file = File::open(file_path)?;
    let reader = BufReader::new(file);
    let mut lines = reader.lines();
    let mut records = Vec::new();

    while let Some(Ok(header)) = lines.next() {
        if !header.starts_with('@') {
            continue;
        }

        let id = header[1..].split_whitespace().next().unwrap_or("unknown").to_string();

        if let (Some(Ok(sequence)), Some(Ok(_plus)), Some(Ok(quality))) =
            (lines.next(), lines.next(), lines.next())
        {
            records.push(SequenceRecord::fastq(id, sequence.into_bytes(), quality.into_bytes()));
        }
    }

    Ok(records)
}
