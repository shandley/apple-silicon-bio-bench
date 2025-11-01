//! Level 1/2 Automated Execution Harness
//!
//! Runs all 3,000 experiments (20 operations × 25 configs × 6 scales)
//! with automated checkpointing, parallel execution, and result storage.
//!
//! # Usage
//!
//! ```bash
//! cargo run --release -p asbb-cli --bin run-level1
//! ```

use anyhow::{Context, Result};
use asbb_core::operation_registry::{Backend, OperationMetadata, OperationRegistry};
use asbb_core::OperationCategory;
use asbb_explorer::ExecutionEngine;
use asbb_ops::*;
use std::sync::Arc;

fn main() -> Result<()> {
    println!("🚀 Apple Silicon Bio Bench - Level 1/2 Automated Harness");
    println!("======================================================================");
    println!();

    // Create and populate operation registry
    println!("📋 Registering operations...");
    let registry = create_operation_registry()?;

    let implemented = registry.list_implemented();
    println!("   Registered {} implemented operations:", implemented.len());
    for (i, op) in implemented.iter().enumerate() {
        println!("     {}. {}", i + 1, op);
    }
    println!();

    // Load configuration
    println!("⚙️  Loading configuration...");
    let config_path = "experiments/level1_primitives/config.toml";
    println!("   Config: {}", config_path);

    let engine = ExecutionEngine::from_config_file(config_path, registry)
        .context("Failed to load execution engine from config")?;
    println!("   ✅ Configuration loaded successfully");
    println!();

    // Run all experiments
    println!("🔬 Starting experiment execution...");
    println!();

    engine.run_all()
        .context("Failed to run experiments")?;

    println!();
    println!("✅ All experiments complete!");
    println!("📊 Results saved to results/level1_primitives/");

    Ok(())
}

/// Create and populate the operation registry with all 20 operations
fn create_operation_registry() -> Result<OperationRegistry> {
    let mut registry = OperationRegistry::new();

    // Element-wise operations (6)
    registry.register(
        Arc::new(base_counting::BaseCounting::new()),
        OperationMetadata {
            name: "base_counting".to_string(),
            category: OperationCategory::ElementWise,
            complexity: 0.40,
            backends: vec![Backend::Naive, Backend::Neon, Backend::Parallel],
            implemented: true,
            description: Some("Count A, C, G, T bases".to_string()),
        },
    );

    registry.register(
        Arc::new(gc_content::GcContent::new()),
        OperationMetadata {
            name: "gc_content".to_string(),
            category: OperationCategory::ElementWise,
            complexity: 0.315,
            backends: vec![Backend::Naive, Backend::Neon, Backend::Parallel],
            implemented: true,
            description: Some("Calculate GC percentage".to_string()),
        },
    );

    registry.register(
        Arc::new(at_content::ATContent {}),
        OperationMetadata {
            name: "at_content".to_string(),
            category: OperationCategory::ElementWise,
            complexity: 0.35,
            backends: vec![Backend::Naive, Backend::Neon, Backend::Parallel],
            implemented: true,
            description: Some("Calculate AT percentage".to_string()),
        },
    );

    registry.register(
        Arc::new(sequence_length::SequenceLength {}),
        OperationMetadata {
            name: "sequence_length".to_string(),
            category: OperationCategory::ElementWise,
            complexity: 0.20,
            backends: vec![Backend::Naive, Backend::Neon, Backend::Parallel],
            implemented: true,
            description: Some("Measure sequence lengths".to_string()),
        },
    );

    registry.register(
        Arc::new(complexity_score::ComplexityScore::new()),
        OperationMetadata {
            name: "complexity_score".to_string(),
            category: OperationCategory::Aggregation,
            complexity: 0.61,
            backends: vec![Backend::Naive, Backend::Neon, Backend::Parallel, Backend::Gpu],
            implemented: true,
            description: Some("Shannon entropy calculation".to_string()),
        },
    );

    registry.register(
        Arc::new(translation::Translation::new(0, 1).unwrap()),
        OperationMetadata {
            name: "translation".to_string(),
            category: OperationCategory::ElementWise,
            complexity: 0.40,
            backends: vec![Backend::Naive, Backend::Neon, Backend::Parallel],
            implemented: true,
            description: Some("DNA/RNA to protein translation".to_string()),
        },
    );

    // Filtering operations (4)
    registry.register(
        Arc::new(quality_filter::QualityFilter::new(20)),
        OperationMetadata {
            name: "quality_filter".to_string(),
            category: OperationCategory::Filter,
            complexity: 0.55,
            backends: vec![Backend::Naive, Backend::Neon, Backend::Parallel],
            implemented: true,
            description: Some("Filter by quality threshold".to_string()),
        },
    );

    registry.register(
        Arc::new(length_filter::LengthFilter::new(50)),
        OperationMetadata {
            name: "length_filter".to_string(),
            category: OperationCategory::Filter,
            complexity: 0.25,
            backends: vec![Backend::Naive, Backend::Neon, Backend::Parallel],
            implemented: true,
            description: Some("Filter by length range".to_string()),
        },
    );

    registry.register(
        Arc::new(sequence_masking::SequenceMasking::new()),
        OperationMetadata {
            name: "sequence_masking".to_string(),
            category: OperationCategory::Filter,
            complexity: 0.30,
            backends: vec![Backend::Naive, Backend::Neon, Backend::Parallel],
            implemented: true,
            description: Some("Mask low-quality bases".to_string()),
        },
    );

    registry.register(
        Arc::new(adapter_trimming::AdapterTrimming::new(b"AGATCGGAAGAGC".to_vec(), 5, 0)),
        OperationMetadata {
            name: "adapter_trimming".to_string(),
            category: OperationCategory::Filter,
            complexity: 0.55,
            backends: vec![Backend::Naive, Backend::Neon, Backend::Parallel],
            implemented: true,
            description: Some("Detect and remove adapters".to_string()),
        },
    );

    // Aggregation operations (4)
    registry.register(
        Arc::new(quality_aggregation::QualityAggregation::new()),
        OperationMetadata {
            name: "quality_aggregation".to_string(),
            category: OperationCategory::Aggregation,
            complexity: 0.50,
            backends: vec![Backend::Naive, Backend::Neon, Backend::Parallel],
            implemented: true,
            description: Some("Per-position quality stats".to_string()),
        },
    );

    registry.register(
        Arc::new(n_content::NContent {}),
        OperationMetadata {
            name: "n_content".to_string(),
            category: OperationCategory::Aggregation,
            complexity: 0.38,
            backends: vec![Backend::Naive, Backend::Neon, Backend::Parallel],
            implemented: true,
            description: Some("Calculate N-base percentage".to_string()),
        },
    );

    registry.register(
        Arc::new(quality_statistics::QualityStatistics::new()),
        OperationMetadata {
            name: "quality_statistics".to_string(),
            category: OperationCategory::Aggregation,
            complexity: 0.38,
            backends: vec![Backend::Naive, Backend::Neon, Backend::Parallel],
            implemented: true,
            description: Some("Mean, median, quartiles".to_string()),
        },
    );

    registry.register(
        Arc::new(minhash_sketching::MinHashSketching::new(21, 1000)),
        OperationMetadata {
            name: "minhash_sketching".to_string(),
            category: OperationCategory::Aggregation,
            complexity: 0.48,
            backends: vec![Backend::Naive, Backend::Neon, Backend::Parallel],
            implemented: true,
            description: Some("Sequence similarity sketches".to_string()),
        },
    );

    // Pairwise operations (2)
    registry.register(
        Arc::new(hamming_distance::HammingDistance::new()),
        OperationMetadata {
            name: "hamming_distance".to_string(),
            category: OperationCategory::Pairwise,
            complexity: 0.35,
            backends: vec![Backend::Naive, Backend::Neon, Backend::Parallel],
            implemented: true,
            description: Some("Pairwise Hamming distance".to_string()),
        },
    );

    registry.register(
        Arc::new(edit_distance::EditDistance::new(1000)),
        OperationMetadata {
            name: "edit_distance".to_string(),
            category: OperationCategory::Pairwise,
            complexity: 0.70,
            backends: vec![Backend::Naive, Backend::Neon, Backend::Parallel],
            implemented: true,
            description: Some("Levenshtein distance (DP)".to_string()),
        },
    );

    // Search operations (2)
    registry.register(
        Arc::new(kmer_counting::KmerCounting::new(21, false)),
        OperationMetadata {
            name: "kmer_counting".to_string(),
            category: OperationCategory::Search,
            complexity: 0.45,
            backends: vec![Backend::Naive, Backend::Neon, Backend::Parallel],
            implemented: true,
            description: Some("K-mer frequency counting".to_string()),
        },
    );

    registry.register(
        Arc::new(kmer_extraction::KmerExtraction::new(21, false)),
        OperationMetadata {
            name: "kmer_extraction".to_string(),
            category: OperationCategory::Search,
            complexity: 0.35,
            backends: vec![Backend::Naive, Backend::Neon, Backend::Parallel],
            implemented: true,
            description: Some("Extract k-mers as records".to_string()),
        },
    );

    // Transform operations (1) - use ElementWise category
    registry.register(
        Arc::new(reverse_complement::ReverseComplement::new()),
        OperationMetadata {
            name: "reverse_complement".to_string(),
            category: OperationCategory::ElementWise,
            complexity: 0.45,
            backends: vec![Backend::Naive, Backend::Neon, Backend::Parallel],
            implemented: true,
            description: Some("Reverse complement sequences".to_string()),
        },
    );

    // I/O operations (1)
    registry.register(
        Arc::new(fastq_parsing::FastqParsing::new(true)),
        OperationMetadata {
            name: "fastq_parsing".to_string(),
            category: OperationCategory::IO,
            complexity: 0.25,
            backends: vec![Backend::Naive, Backend::Neon, Backend::Parallel],
            implemented: true,
            description: Some("Parse FASTQ format".to_string()),
        },
    );

    Ok(registry)
}
