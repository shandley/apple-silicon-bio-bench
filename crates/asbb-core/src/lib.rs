//! Core types and traits for Apple Silicon Bio Bench
//!
//! This crate provides the foundational types for systematic performance characterization
//! of bioinformatics sequence operations on Apple Silicon hardware.
//!
//! # Design Philosophy: Apple Silicon First
//!
//! These types are designed specifically for Apple Silicon's unique capabilities:
//! - Unified memory architecture (CPU/GPU share memory)
//! - NEON SIMD (128-bit vectors, native to ARM)
//! - Heterogeneous computing (P-cores + E-cores + GPU + Neural Engine)
//! - AMX matrix engine (512-bit matrix operations)
//! - Hardware compression/decompression
//! - M5 GPU Neural Accelerators (4× AI performance)
//!
//! We do NOT simply port x86 optimization strategies. We explore novel approaches
//! that leverage Apple Silicon's architecture.

#![allow(dead_code)] // Temporary during development
#![allow(unused_variables)]

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::time::Duration;

// ============================================================================
// Modules
// ============================================================================

/// 2-bit DNA encoding for efficient sequence representation
pub mod encoding;

/// Operation registry for centralized operation management
pub mod operation_registry;

// ============================================================================
// Data Characteristics
// ============================================================================

/// Characteristics of input sequence data
///
/// Describes the input data for an operation, used to predict optimal
/// hardware configuration and expected performance.
///
/// Note: Does not derive Eq/Hash because it contains floating point values.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DataCharacteristics {
    /// File format (FASTA, FASTQ)
    pub format: DataFormat,

    /// Number of sequences (100 to 1B range)
    pub num_sequences: usize,

    /// Mean sequence length in base pairs
    pub seq_length_mean: usize,

    /// Standard deviation of sequence length
    pub seq_length_std: usize,

    /// Read type (single-end, paired-end, interleaved)
    pub read_type: ReadType,

    /// Quality score distribution (FASTQ only)
    pub quality_distribution: Option<QualityDistribution>,

    /// Estimated file size in bytes
    pub estimated_size_bytes: Option<usize>,
}

impl DataCharacteristics {
    /// Calculate total bases in dataset
    pub fn total_bases(&self) -> usize {
        self.num_sequences * self.seq_length_mean
    }

    /// Calculate scale category (tiny, small, medium, large, etc.)
    pub fn scale_category(&self) -> DataScale {
        match self.num_sequences {
            0..=999 => DataScale::Tiny,
            1000..=9_999 => DataScale::Small,
            10_000..=99_999 => DataScale::Medium,
            100_000..=999_999 => DataScale::Large,
            1_000_000..=9_999_999 => DataScale::VeryLarge,
            _ => DataScale::Huge,
        }
    }
}

/// Sequence file format
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DataFormat {
    /// FASTA format (sequences only, no quality scores)
    Fasta,
    /// FASTQ format (sequences with quality scores)
    Fastq,
}

/// Read type for sequencing data
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ReadType {
    /// Single-end reads
    SingleEnd,
    /// Paired-end reads (separate R1/R2 files)
    PairedEnd,
    /// Interleaved paired reads (R1/R2 in same file)
    Interleaved,
}

/// Quality score distribution characteristics (FASTQ)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct QualityDistribution {
    /// Mean quality score (Phred scale)
    pub mean_quality: f64,
    /// Standard deviation of quality scores
    pub std_quality: f64,
    /// Quality distribution type
    pub distribution_type: QualityDistType,
}

/// Type of quality score distribution
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum QualityDistType {
    /// Uniform high quality (e.g., Q40 throughout)
    UniformHigh,
    /// Degrading quality (typical Illumina: Q40 → Q20)
    Degrading,
    /// Realistic with occasional drops
    Realistic,
}

/// Data scale categories
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum DataScale {
    Tiny,      // <1K sequences
    Small,     // 1K-10K
    Medium,    // 10K-100K
    Large,     // 100K-1M
    VeryLarge, // 1M-10M
    Huge,      // >10M
}

// ============================================================================
// Hardware Configuration
// ============================================================================

/// Hardware configuration for experimental run
///
/// Specifies which Apple Silicon features to use for an operation.
/// This is the optimization space we systematically explore.
///
/// # Apple Silicon Features
///
/// - **NEON**: 128-bit SIMD vectors (native ARM)
/// - **AMX**: 512-bit matrix operations
/// - **GPU**: Metal compute shaders
/// - **Neural Engine**: ML acceleration (16 TOPS+)
/// - **M5 GPU Neural Accelerators**: ML on GPU (4× faster than M4)
/// - **P-cores**: Performance cores
/// - **E-cores**: Efficiency cores
/// - **Unified Memory**: CPU/GPU share memory (zero-copy)
/// - **Hardware Compression**: AppleArchive, zstd acceleration
/// - **GCD**: Grand Central Dispatch (work-stealing scheduler)
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct HardwareConfig {
    /// Use NEON SIMD instructions
    pub use_neon: bool,

    /// Number of threads to use (1-8+ depending on chip)
    pub num_threads: usize,

    /// Thread assignment strategy (P-cores, E-cores, mixed)
    pub thread_assignment: ThreadAssignment,

    /// Data encoding (2-bit, ASCII, etc.)
    pub encoding: Encoding,

    /// Use unified memory optimization (CPU/GPU zero-copy)
    pub use_unified_memory: bool,

    /// Use Metal GPU for compute
    pub use_gpu: bool,

    /// GPU batch size (if using GPU)
    pub gpu_batch_size: Option<usize>,

    /// Use AMX matrix engine
    pub use_amx: bool,

    /// Use Neural Engine for ML operations
    pub use_neural_engine: bool,

    /// Use M5 GPU Neural Accelerators (M5 only, 4× AI performance)
    pub use_m5_gpu_neural_accel: bool,

    /// Use hardware compression/decompression
    pub use_hw_compression: bool,

    /// Use Grand Central Dispatch (GCD) work-stealing scheduler
    pub use_gcd: bool,

    /// Quality of Service for thread scheduling
    pub qos: QualityOfService,

    /// Target chip generation (for validation and reporting)
    pub chip_generation: Option<ChipGeneration>,
}

impl HardwareConfig {
    /// Create a naive baseline configuration (scalar, single-threaded)
    pub fn naive() -> Self {
        Self {
            use_neon: false,
            num_threads: 1,
            thread_assignment: ThreadAssignment::PCoresOnly,
            encoding: Encoding::Ascii,
            use_unified_memory: false,
            use_gpu: false,
            gpu_batch_size: None,
            use_amx: false,
            use_neural_engine: false,
            use_m5_gpu_neural_accel: false,
            use_hw_compression: false,
            use_gcd: false,
            qos: QualityOfService::Default,
            chip_generation: None,
        }
    }

    /// Create a fully-optimized configuration (all features enabled)
    pub fn fully_optimized(chip: ChipGeneration) -> Self {
        let is_m5 = matches!(chip, ChipGeneration::M5);

        Self {
            use_neon: true,
            num_threads: 8, // Adjust based on chip
            thread_assignment: ThreadAssignment::Mixed,
            encoding: Encoding::TwoBit,
            use_unified_memory: true,
            use_gpu: true,
            gpu_batch_size: Some(100_000),
            use_amx: true,
            use_neural_engine: true,
            use_m5_gpu_neural_accel: is_m5, // Only available on M5
            use_hw_compression: true,
            use_gcd: true,
            qos: QualityOfService::UserInitiated,
            chip_generation: Some(chip),
        }
    }
}

/// Thread assignment strategy
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ThreadAssignment {
    /// Use only Performance cores
    PCoresOnly,
    /// Use only Efficiency cores
    ECoresOnly,
    /// Mix of P-cores and E-cores (OS decides)
    Mixed,
    /// Custom pinning (advanced)
    Custom,
}

/// DNA sequence encoding scheme
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Encoding {
    /// ASCII representation (1 byte per base)
    Ascii,
    /// 2-bit encoding (0.25 bytes per base)
    TwoBit,
    /// 2-bit with N-mask
    TwoBitExtended,
    /// 4-bit IUPAC ambiguity codes
    FourBit,
}

impl Encoding {
    /// Bytes per base for this encoding
    pub fn bytes_per_base(&self) -> f64 {
        match self {
            Encoding::Ascii => 1.0,
            Encoding::TwoBit => 0.25,
            Encoding::TwoBitExtended => 0.25, // Plus separate mask
            Encoding::FourBit => 0.5,
        }
    }
}

/// Quality of Service for thread scheduling
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum QualityOfService {
    /// User-interactive (highest priority)
    UserInteractive,
    /// User-initiated (high priority)
    UserInitiated,
    /// Default priority
    Default,
    /// Utility (low priority)
    Utility,
    /// Background (lowest priority)
    Background,
}

/// Apple Silicon chip generation
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum ChipGeneration {
    M1,
    M2,
    M3,
    M4,
    M5, // Latest: GPU Neural Accelerators, 153 GB/s bandwidth
}

impl ChipGeneration {
    /// Memory bandwidth in GB/s (base variant)
    pub fn memory_bandwidth_gbps(&self) -> f64 {
        match self {
            ChipGeneration::M1 => 68.25,
            ChipGeneration::M2 => 102.4,
            ChipGeneration::M3 => 102.4,
            ChipGeneration::M4 => 120.0,
            ChipGeneration::M5 => 153.6,
        }
    }

    /// Neural Engine TOPS (trillion operations per second)
    pub fn neural_engine_tops(&self) -> f64 {
        match self {
            ChipGeneration::M1 => 11.0,
            ChipGeneration::M2 => 15.8,
            ChipGeneration::M3 => 18.0,
            ChipGeneration::M4 => 38.0,
            ChipGeneration::M5 => 50.0, // Estimated
        }
    }

    /// Has M5 GPU Neural Accelerators (4× AI performance)
    pub fn has_gpu_neural_accelerators(&self) -> bool {
        matches!(self, ChipGeneration::M5)
    }

    /// Has 3rd-gen ray tracing
    pub fn has_ray_tracing_gen3(&self) -> bool {
        matches!(self, ChipGeneration::M5)
    }
}

// ============================================================================
// Performance Results
// ============================================================================

/// Performance metrics from an experimental run
///
/// Comprehensive measurements of throughput, latency, resource usage, and energy.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PerformanceResult {
    /// Throughput in sequences per second
    pub throughput_seqs_per_sec: f64,

    /// Throughput in megabytes per second
    pub throughput_mbps: f64,

    /// Latency to first result (streaming operations)
    pub latency_first_result: Duration,

    /// Median (50th percentile) latency
    pub latency_p50: Duration,

    /// 99th percentile latency
    pub latency_p99: Duration,

    /// Peak memory usage in bytes
    pub memory_peak: usize,

    /// Average memory usage in bytes
    pub memory_avg: usize,

    /// CPU utilization (0.0 to 1.0 per core, can exceed 1.0 for multi-core)
    pub cpu_utilization: f64,

    /// GPU utilization (0.0 to 1.0, if GPU used)
    pub gpu_utilization: Option<f64>,

    /// Energy consumed in joules (if measurable)
    pub energy_joules: Option<f64>,

    /// Correctness: output matches reference implementation
    pub output_matches_reference: bool,
}

impl PerformanceResult {
    /// Calculate speedup relative to baseline
    pub fn speedup_vs(&self, baseline: &PerformanceResult) -> f64 {
        self.throughput_seqs_per_sec / baseline.throughput_seqs_per_sec
    }

    /// Calculate efficiency (throughput per watt)
    pub fn efficiency_seqs_per_joule(&self) -> Option<f64> {
        self.energy_joules.map(|joules| {
            let total_time_secs = self.latency_p50.as_secs_f64();
            let total_seqs = self.throughput_seqs_per_sec * total_time_secs;
            total_seqs / joules
        })
    }
}

// ============================================================================
// Operations
// ============================================================================

/// Category of bioinformatics operation
///
/// Different categories have different optimization characteristics:
/// - **Element-wise**: Highly vectorizable, memory-bound
/// - **Filter**: Sequential, data-dependent branches
/// - **Search**: Parallel, memory-intensive
/// - **Pairwise**: O(n²), compute-intensive
/// - **Aggregation**: Reducible, embarrassingly parallel
/// - **IO**: Bandwidth-limited, hardware compression helps
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum OperationCategory {
    /// Element-wise operations (base counting, GC content, reverse complement)
    ElementWise,
    /// Filtering operations (quality filter, length filter)
    Filter,
    /// Search operations (k-mer extraction, motif finding)
    Search,
    /// Pairwise operations (alignment, clustering)
    Pairwise,
    /// Aggregation operations (statistics, histograms)
    Aggregation,
    /// I/O operations (parsing, decompression, format conversion)
    IO,
}

/// A primitive bioinformatics operation
///
/// Each operation must implement multiple backends to explore the optimization space:
/// - **Naive**: Baseline scalar implementation
/// - **NEON**: ARM NEON SIMD vectorization
/// - **Parallel**: Multi-threaded with Rayon or GCD
/// - **GPU**: Metal compute shaders
/// - **Neural**: Neural Engine (ML-based, if applicable)
/// - **AMX**: Matrix operations (if applicable)
///
/// # Apple Silicon First
///
/// Implementations should explore novel approaches, not just port x86 patterns.
/// For example, use unified memory for zero-copy CPU/GPU, explore M5 GPU Neural
/// Accelerators for ML-based operations.
pub trait PrimitiveOperation: Send + Sync {
    /// Operation name (e.g., "base_counting", "quality_filter")
    fn name(&self) -> &str;

    /// Operation category
    fn category(&self) -> OperationCategory;

    /// Execute with naive (baseline) implementation
    fn execute_naive(&self, data: &[SequenceRecord]) -> Result<OperationOutput>;

    /// Execute with NEON SIMD (if applicable)
    fn execute_neon(&self, data: &[SequenceRecord]) -> Result<OperationOutput> {
        // Default: fall back to naive
        self.execute_naive(data)
    }

    /// Execute with parallel threads
    fn execute_parallel(
        &self,
        data: &[SequenceRecord],
        num_threads: usize,
    ) -> Result<OperationOutput> {
        // Default: fall back to naive
        self.execute_naive(data)
    }

    /// Execute with Metal GPU (if applicable)
    fn execute_gpu(
        &self,
        data: &[SequenceRecord],
        batch_size: usize,
    ) -> Result<OperationOutput> {
        // Default: not supported
        anyhow::bail!("GPU execution not implemented for {}", self.name())
    }

    /// Execute with Neural Engine (if applicable, ML-based)
    fn execute_neural(&self, data: &[SequenceRecord]) -> Result<OperationOutput> {
        // Default: not supported
        anyhow::bail!("Neural Engine execution not implemented for {}", self.name())
    }

    /// Execute with AMX matrix engine (if applicable)
    fn execute_amx(&self, data: &[SequenceRecord]) -> Result<OperationOutput> {
        // Default: not supported
        anyhow::bail!("AMX execution not implemented for {}", self.name())
    }

    /// Execute with full hardware configuration
    fn execute_with_config(
        &self,
        data: &[SequenceRecord],
        config: &HardwareConfig,
    ) -> Result<OperationOutput> {
        // Decision logic based on config
        if config.use_gpu {
            if let Some(batch_size) = config.gpu_batch_size {
                return self.execute_gpu(data, batch_size);
            }
        }

        if config.use_neural_engine || config.use_m5_gpu_neural_accel {
            if let Ok(result) = self.execute_neural(data) {
                return Ok(result);
            }
        }

        if config.use_amx {
            if let Ok(result) = self.execute_amx(data) {
                return Ok(result);
            }
        }

        if config.num_threads > 1 {
            return self.execute_parallel(data, config.num_threads);
        }

        if config.use_neon {
            return self.execute_neon(data);
        }

        self.execute_naive(data)
    }
}

/// Input: A sequence record (FASTA or FASTQ)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SequenceRecord {
    /// Sequence identifier (header)
    pub id: String,
    /// DNA sequence (A, C, G, T, N)
    pub sequence: Vec<u8>,
    /// Quality scores (FASTQ only, Phred+33 encoded)
    pub quality: Option<Vec<u8>>,
}

impl SequenceRecord {
    /// Create a new FASTA record (no quality)
    pub fn fasta(id: String, sequence: Vec<u8>) -> Self {
        Self {
            id,
            sequence,
            quality: None,
        }
    }

    /// Create a new FASTQ record (with quality)
    pub fn fastq(id: String, sequence: Vec<u8>, quality: Vec<u8>) -> Self {
        Self {
            id,
            sequence,
            quality: Some(quality),
        }
    }

    /// Get sequence length
    pub fn len(&self) -> usize {
        self.sequence.len()
    }

    /// Check if record is empty
    pub fn is_empty(&self) -> bool {
        self.sequence.is_empty()
    }
}

/// Output from an operation
///
/// Operations produce different output types:
/// - Filtered records
/// - Computed statistics
/// - Boolean results (pass/fail)
/// - Transformed sequences
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum OperationOutput {
    /// Filtered sequence records
    Records(Vec<SequenceRecord>),
    /// Statistics (counts, means, etc.)
    Statistics(serde_json::Value),
    /// Boolean result (pass/fail)
    Boolean(bool),
    /// Count result
    Count(usize),
    /// Generic JSON output
    Json(serde_json::Value),
}

// ============================================================================
// Hardware Profile (Runtime Detection)
// ============================================================================

/// Detected hardware capabilities of the system
///
/// Auto-detected at runtime using system APIs
///
/// Note: Does not derive Eq because it contains floating point values.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HardwareProfile {
    /// Chip generation (M1, M2, M3, M4, M5)
    pub chip: ChipGeneration,

    /// Chip variant (Base, Pro, Max, Ultra)
    pub chip_variant: ChipVariant,

    /// Number of Performance cores
    pub num_p_cores: usize,

    /// Number of Efficiency cores
    pub num_e_cores: usize,

    /// Number of GPU cores
    pub num_gpu_cores: usize,

    /// Total unified memory (GB)
    pub memory_gb: usize,

    /// Memory bandwidth (GB/s)
    pub memory_bandwidth_gbps: f64,

    /// Has NEON support (always true on Apple Silicon)
    pub has_neon: bool,

    /// Has AMX support
    pub has_amx: bool,

    /// Has Neural Engine
    pub has_neural_engine: bool,

    /// Has M5 GPU Neural Accelerators
    pub has_m5_gpu_neural_accel: bool,
}

impl HardwareProfile {
    /// Detect hardware profile from system
    pub fn detect() -> Result<Self> {
        // TODO: Implement system detection using sysctl/IOKit
        // For now, return placeholder
        anyhow::bail!("Hardware detection not yet implemented")
    }
}

/// Chip variant (Base, Pro, Max, Ultra)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ChipVariant {
    Base,
    Pro,
    Max,
    Ultra,
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_data_characteristics_scale() {
        let tiny = DataCharacteristics {
            format: DataFormat::Fastq,
            num_sequences: 100,
            seq_length_mean: 150,
            seq_length_std: 10,
            read_type: ReadType::SingleEnd,
            quality_distribution: None,
            estimated_size_bytes: None,
        };
        assert_eq!(tiny.scale_category(), DataScale::Tiny);
        assert_eq!(tiny.total_bases(), 15_000);

        let huge = DataCharacteristics {
            num_sequences: 50_000_000,
            ..tiny
        };
        assert_eq!(huge.scale_category(), DataScale::Huge);
    }

    #[test]
    fn test_hardware_config_naive() {
        let config = HardwareConfig::naive();
        assert!(!config.use_neon);
        assert!(!config.use_gpu);
        assert_eq!(config.num_threads, 1);
    }

    #[test]
    fn test_hardware_config_optimized_m5() {
        let config = HardwareConfig::fully_optimized(ChipGeneration::M5);
        assert!(config.use_neon);
        assert!(config.use_gpu);
        assert!(config.use_m5_gpu_neural_accel); // M5 only

        let config_m4 = HardwareConfig::fully_optimized(ChipGeneration::M4);
        assert!(!config_m4.use_m5_gpu_neural_accel); // Not available on M4
    }

    #[test]
    fn test_chip_generation_capabilities() {
        assert_eq!(ChipGeneration::M1.memory_bandwidth_gbps(), 68.25);
        assert_eq!(ChipGeneration::M5.memory_bandwidth_gbps(), 153.6);

        assert!(!ChipGeneration::M4.has_gpu_neural_accelerators());
        assert!(ChipGeneration::M5.has_gpu_neural_accelerators());
    }

    #[test]
    fn test_encoding_bytes_per_base() {
        assert_eq!(Encoding::Ascii.bytes_per_base(), 1.0);
        assert_eq!(Encoding::TwoBit.bytes_per_base(), 0.25);
    }

    #[test]
    fn test_sequence_record() {
        let fasta = SequenceRecord::fasta(
            "seq1".to_string(),
            b"ACGT".to_vec(),
        );
        assert_eq!(fasta.len(), 4);
        assert!(fasta.quality.is_none());

        let fastq = SequenceRecord::fastq(
            "seq2".to_string(),
            b"ACGT".to_vec(),
            b"IIII".to_vec(),
        );
        assert!(fastq.quality.is_some());
    }

    #[test]
    fn test_performance_result_speedup() {
        let baseline = PerformanceResult {
            throughput_seqs_per_sec: 1000.0,
            throughput_mbps: 1.5,
            latency_first_result: Duration::from_millis(10),
            latency_p50: Duration::from_millis(100),
            latency_p99: Duration::from_millis(200),
            memory_peak: 1_000_000,
            memory_avg: 500_000,
            cpu_utilization: 1.0,
            gpu_utilization: None,
            energy_joules: Some(10.0),
            output_matches_reference: true,
        };

        let optimized = PerformanceResult {
            throughput_seqs_per_sec: 10_000.0,
            ..baseline.clone()
        };

        assert_eq!(optimized.speedup_vs(&baseline), 10.0);
    }
}
