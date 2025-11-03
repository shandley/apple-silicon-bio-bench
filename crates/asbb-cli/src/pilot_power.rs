//! Power Consumption Pilot - Environmental Pillar Validation
//!
//! Measures energy consumption for bioinformatics operations on Apple Silicon.
//!
//! **Research Questions**:
//! 1. Does NEON increase power draw per unit time?
//! 2. Does energy savings scale proportionally with runtime reduction?
//! 3. What's the energy efficiency of NEON vs parallel optimizations?
//! 4. Can we validate the "300× less energy" claim?
//!
//! **Method**:
//! - Loop each operation for 60 seconds (stable power readings)
//! - Correlate with powermetrics CPU package power
//! - Calculate energy per sequence processed
//! - Compare energy efficiency across configurations
//!
//! Run in release mode (pair with powermetrics):
//! ```bash
//! # In Terminal 1: Start powermetrics
//! sudo powermetrics --samplers cpu_power --sample-rate 100 > powermetrics_log.txt
//!
//! # In Terminal 2: Run power pilot
//! cargo run --release -p asbb-cli --bin asbb-pilot-power
//! ```

use anyhow::Result;
use asbb_core::{PrimitiveOperation, SequenceRecord};
use asbb_ops::{
    base_counting::BaseCounting,
    gc_content::GcContent,
    quality_aggregation::QualityAggregation,
};
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;
use std::time::{Duration, Instant};
use chrono::Local;

/// Dataset scale definition
#[derive(Debug, Clone)]
struct Scale {
    name: &'static str,
    path: &'static str,
    num_sequences: usize,
}

const SCALES: &[Scale] = &[
    Scale { name: "Medium", path: "datasets/medium_10k_150bp.fq", num_sequences: 10_000 },
    Scale { name: "Large", path: "datasets/large_100k_150bp.fq", num_sequences: 100_000 },
];

/// Configuration for experiment
#[derive(Debug, Clone, Copy)]
enum Config {
    Naive,
    Neon,
    Neon4t,
    Neon8t,
}

impl Config {
    fn name(&self) -> &'static str {
        match self {
            Config::Naive => "naive",
            Config::Neon => "neon",
            Config::Neon4t => "neon_4t",
            Config::Neon8t => "neon_8t",
        }
    }

    fn description(&self) -> &'static str {
        match self {
            Config::Naive => "Scalar, 1 thread (baseline)",
            Config::Neon => "NEON SIMD, 1 thread",
            Config::Neon4t => "NEON SIMD, 4 threads",
            Config::Neon8t => "NEON SIMD, 8 threads",
        }
    }
}

/// Result of one experiment
#[derive(Debug)]
struct ExperimentResult {
    operation: String,
    config: String,
    scale: String,
    num_sequences: usize,
    loop_duration_s: f64,
    iterations: usize,
    sequences_processed: usize,
    throughput_seqs_per_sec: f64,
    timestamp: String,
}

impl ExperimentResult {
    fn csv_header() -> String {
        "operation,config,scale,num_sequences,loop_duration_s,iterations,sequences_processed,throughput_seqs_per_sec,timestamp".to_string()
    }

    fn to_csv(&self) -> String {
        format!(
            "{},{},{},{},{:.3},{},{},{:.2},{}",
            self.operation,
            self.config,
            self.scale,
            self.num_sequences,
            self.loop_duration_s,
            self.iterations,
            self.sequences_processed,
            self.throughput_seqs_per_sec,
            self.timestamp
        )
    }
}

/// Load FASTQ dataset
fn load_dataset(path: &Path) -> Result<Vec<SequenceRecord>> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);
    let mut lines = reader.lines();
    let mut records = Vec::new();

    while let Some(Ok(header)) = lines.next() {
        if !header.starts_with('@') {
            continue;
        }

        let sequence = lines.next().ok_or_else(|| anyhow::anyhow!("Missing sequence"))??;
        let _plus = lines.next().ok_or_else(|| anyhow::anyhow!("Missing +"))??;
        let quality = lines.next().ok_or_else(|| anyhow::anyhow!("Missing quality"))??;

        records.push(SequenceRecord {
            id: header[1..].to_string(),
            sequence: sequence.into_bytes(),
            quality: Some(quality.into_bytes()),
        });
    }

    Ok(records)
}

/// Run experiment: loop operation for 60 seconds
fn run_experiment<T: PrimitiveOperation>(
    operation_name: &str,
    config: Config,
    scale: &Scale,
    data: &[SequenceRecord],
    op: &T,
) -> Result<ExperimentResult> {
    // Warmup: 3 iterations (discard)
    for _ in 0..3 {
        match config {
            Config::Naive => { let _ = op.execute_naive(data)?; }
            Config::Neon => { let _ = op.execute_neon(data)?; }
            Config::Neon4t => { let _ = op.execute_parallel(data, 4)?; }
            Config::Neon8t => { let _ = op.execute_parallel(data, 8)?; }
        }
    }

    // Main loop: Run for 60 seconds
    let start = Instant::now();
    let target_duration = Duration::from_secs(60);
    let mut iterations = 0;

    while start.elapsed() < target_duration {
        match config {
            Config::Naive => { let _ = op.execute_naive(data)?; }
            Config::Neon => { let _ = op.execute_neon(data)?; }
            Config::Neon4t => { let _ = op.execute_parallel(data, 4)?; }
            Config::Neon8t => { let _ = op.execute_parallel(data, 8)?; }
        }
        iterations += 1;
    }

    let duration = start.elapsed();
    let sequences_processed = data.len() * iterations;
    let throughput = sequences_processed as f64 / duration.as_secs_f64();
    let timestamp = Local::now().format("%Y-%m-%dT%H:%M:%S").to_string();

    Ok(ExperimentResult {
        operation: operation_name.to_string(),
        config: config.name().to_string(),
        scale: scale.name.to_string(),
        num_sequences: scale.num_sequences,
        loop_duration_s: duration.as_secs_f64(),
        iterations,
        sequences_processed,
        throughput_seqs_per_sec: throughput,
        timestamp,
    })
}

fn main() -> Result<()> {
    eprintln!("🔋 Power Consumption Pilot - Environmental Pillar Validation");
    eprintln!();
    eprintln!("📊 Experiments: 3 operations × 4 configs × 2 scales = 24 total");
    eprintln!("⏱️  Duration: Each experiment loops for 60 seconds (stable power measurement)");
    eprintln!();
    eprintln!("Starting in 3 seconds...");
    eprintln!();

    // Brief delay instead of waiting for input (non-interactive friendly)
    std::thread::sleep(Duration::from_secs(3));

    // Print CSV header
    println!("{}", ExperimentResult::csv_header());

    let operations: Vec<(&str, Box<dyn Fn(&[SequenceRecord], Config) -> Result<ExperimentResult>>)> = vec![
        ("base_counting", Box::new(|data, config| {
            let op = BaseCounting::new();
            run_experiment("base_counting", config, &Scale { name: "temp", path: "temp", num_sequences: data.len() }, data, &op)
        })),
        ("gc_content", Box::new(|data, config| {
            let op = GcContent::new();
            run_experiment("gc_content", config, &Scale { name: "temp", path: "temp", num_sequences: data.len() }, data, &op)
        })),
        ("quality_aggregation", Box::new(|data, config| {
            let op = QualityAggregation::new();
            run_experiment("quality_aggregation", config, &Scale { name: "temp", path: "temp", num_sequences: data.len() }, data, &op)
        })),
    ];

    let configs = [Config::Naive, Config::Neon, Config::Neon4t, Config::Neon8t];

    let mut experiment_num = 0;
    let total_experiments = operations.len() * configs.len() * SCALES.len();

    for (operation_name, _) in &operations {
        for scale in SCALES {
            eprintln!();
            eprintln!("Loading dataset: {} ({} sequences)...", scale.name, scale.num_sequences);
            let data = load_dataset(Path::new(scale.path))?;
            eprintln!("✓ Loaded {} sequences", data.len());

            for config in &configs {
                experiment_num += 1;
                eprintln!();
                eprintln!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
                eprintln!("Experiment {}/{}", experiment_num, total_experiments);
                eprintln!("Operation: {}", operation_name);
                eprintln!("Config: {} ({})", config.name(), config.description());
                eprintln!("Scale: {} ({} sequences)", scale.name, scale.num_sequences);
                eprintln!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
                eprintln!();
                eprintln!("⏳ Running for 60 seconds...");

                let start = Instant::now();

                // Execute experiment based on operation
                let result = match operation_name.as_ref() {
                    "base_counting" => {
                        let op = BaseCounting::new();
                        run_experiment(operation_name, *config, scale, &data, &op)?
                    }
                    "gc_content" => {
                        let op = GcContent::new();
                        run_experiment(operation_name, *config, scale, &data, &op)?
                    }
                    "quality_aggregation" => {
                        let op = QualityAggregation::new();
                        run_experiment(operation_name, *config, scale, &data, &op)?
                    }
                    _ => anyhow::bail!("Unknown operation: {}", operation_name),
                };

                let elapsed = start.elapsed();

                eprintln!("✓ Complete in {:.1}s", elapsed.as_secs_f64());
                eprintln!("  Iterations: {}", result.iterations);
                eprintln!("  Sequences processed: {}", result.sequences_processed);
                eprintln!("  Throughput: {:.0} seqs/sec", result.throughput_seqs_per_sec);

                println!("{}", result.to_csv());

                // Cooldown: 5 seconds between experiments
                eprintln!();
                eprintln!("💤 Cooldown: 5 seconds...");
                std::thread::sleep(Duration::from_secs(5));
            }
        }
    }

    eprintln!();
    eprintln!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    eprintln!("✅ All {} experiments complete!", total_experiments);
    eprintln!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    eprintln!();
    eprintln!("Next steps:");
    eprintln!("1. Stop powermetrics (Ctrl+C in that terminal)");
    eprintln!("2. Run analysis script: python analysis/parse_powermetrics.py");
    eprintln!("3. Generate findings: python analysis/generate_power_findings.py");

    Ok(())
}
