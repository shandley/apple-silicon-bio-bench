//! Benchmark definition and configuration
//!
//! Represents a single experimental test case

use asbb_core::{DataCharacteristics, HardwareConfig};

/// A single benchmark experiment
///
/// Represents one test case: an operation with specific data and hardware config
pub struct Benchmark {
    /// Name/identifier for this benchmark
    pub name: String,

    /// The operation to benchmark
    pub operation_name: String,

    /// Data characteristics for this run
    pub data_characteristics: DataCharacteristics,

    /// Hardware configuration to use
    pub hardware_config: HardwareConfig,

    /// Number of warmup runs (not measured)
    pub warmup_runs: usize,

    /// Number of measured runs
    pub measured_runs: usize,
}

impl Benchmark {
    /// Create a new benchmark configuration
    pub fn new(
        name: String,
        operation_name: String,
        data_characteristics: DataCharacteristics,
        hardware_config: HardwareConfig,
    ) -> Self {
        Self {
            name,
            operation_name,
            data_characteristics,
            hardware_config,
            warmup_runs: 3,
            measured_runs: 10,
        }
    }

    /// Set number of warmup runs
    pub fn with_warmup_runs(mut self, runs: usize) -> Self {
        self.warmup_runs = runs;
        self
    }

    /// Set number of measured runs
    pub fn with_measured_runs(mut self, runs: usize) -> Self {
        self.measured_runs = runs;
        self
    }
}
