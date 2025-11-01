//! Operation Registry - Centralized management of bioinformatics operations
//!
//! The OperationRegistry provides a centralized place to:
//! - Register all available operations
//! - Query operation metadata
//! - Select appropriate backend based on hardware config
//! - Validate operation capabilities
//!
//! # Design
//!
//! The registry acts as a factory for operations, storing metadata about each
//! operation's category, complexity, and available backends. This enables the
//! automated harness to dynamically select and execute operations.

use crate::{HardwareConfig, OperationCategory, PrimitiveOperation};
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

// ============================================================================
// Operation Metadata
// ============================================================================

/// Metadata about an operation (independent of execution)
///
/// This describes an operation's characteristics for experimental design and analysis.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OperationMetadata {
    /// Operation name (e.g., "base_counting")
    pub name: String,

    /// Operation category (element_wise, filtering, search, etc.)
    pub category: OperationCategory,

    /// Complexity score (0.0-1.0, from Phase 1 analysis)
    ///
    /// This is the "NEON effectiveness" metric that predicts optimization benefit.
    /// - **Low (0.20-0.30)**: Simple operations, high NEON speedup (10-50×)
    /// - **Medium (0.40-0.50)**: Moderate complexity, variable NEON benefit (1-8×)
    /// - **High (0.55-0.70)**: Complex operations, limited NEON benefit (<2×)
    pub complexity: f64,

    /// Available backends for this operation
    pub backends: Vec<Backend>,

    /// Whether operation is implemented (vs planned)
    pub implemented: bool,

    /// Human-readable description
    pub description: Option<String>,
}

impl OperationMetadata {
    /// Predict if NEON will be effective for this operation
    ///
    /// Based on Phase 1 findings: complexity <0.45 → good NEON benefit
    pub fn neon_effective(&self) -> bool {
        self.complexity < 0.45
    }

    /// Predict if GPU might be beneficial for this operation
    ///
    /// Based on Phase 1 findings: NEON ineffective AND complex → GPU candidate
    pub fn gpu_candidate(&self) -> bool {
        !self.neon_effective() && self.complexity > 0.55
    }

    /// Check if operation supports a specific backend
    pub fn has_backend(&self, backend: Backend) -> bool {
        self.backends.contains(&backend)
    }
}

/// Available execution backends
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Backend {
    /// Naive scalar implementation (baseline)
    Naive,
    /// NEON SIMD vectorization
    Neon,
    /// Multi-threaded parallelism
    Parallel,
    /// Metal GPU compute
    Gpu,
    /// Neural Engine (ML-based)
    Neural,
    /// AMX matrix engine
    Amx,
    /// 2-bit encoding optimized
    TwoBit,
}

impl Backend {
    /// Convert HardwareConfig to the primary backend to use
    pub fn from_config(config: &HardwareConfig) -> Self {
        if config.use_gpu {
            Backend::Gpu
        } else if config.use_neural_engine || config.use_m5_gpu_neural_accel {
            Backend::Neural
        } else if config.use_amx {
            Backend::Amx
        } else if config.use_neon {
            Backend::Neon
        } else if config.num_threads > 1 {
            Backend::Parallel
        } else {
            Backend::Naive
        }
    }
}

// ============================================================================
// Operation Registry
// ============================================================================

/// Registry of all available operations
///
/// This is the central catalog of operations for the automated harness.
/// Operations are registered at startup and can be queried by name.
pub struct OperationRegistry {
    /// Map from operation name to operation implementation
    operations: HashMap<String, Arc<dyn PrimitiveOperation>>,

    /// Map from operation name to metadata
    metadata: HashMap<String, OperationMetadata>,
}

impl OperationRegistry {
    /// Create a new empty registry
    pub fn new() -> Self {
        Self {
            operations: HashMap::new(),
            metadata: HashMap::new(),
        }
    }

    /// Register an operation with its metadata
    ///
    /// # Example
    ///
    /// ```ignore
    /// let mut registry = OperationRegistry::new();
    /// registry.register(
    ///     Arc::new(BaseCountingOp),
    ///     OperationMetadata {
    ///         name: "base_counting".to_string(),
    ///         category: OperationCategory::ElementWise,
    ///         complexity: 0.40,
    ///         backends: vec![Backend::Naive, Backend::Neon],
    ///         implemented: true,
    ///         description: Some("Count A, C, G, T bases".to_string()),
    ///     },
    /// );
    /// ```
    pub fn register(
        &mut self,
        operation: Arc<dyn PrimitiveOperation>,
        metadata: OperationMetadata,
    ) {
        let name = metadata.name.clone();
        self.operations.insert(name.clone(), operation);
        self.metadata.insert(name, metadata);
    }

    /// Get an operation by name
    pub fn get(&self, name: &str) -> Result<Arc<dyn PrimitiveOperation>> {
        self.operations
            .get(name)
            .cloned()
            .context(format!("Operation '{}' not found in registry", name))
    }

    /// Get metadata for an operation
    pub fn get_metadata(&self, name: &str) -> Result<&OperationMetadata> {
        self.metadata
            .get(name)
            .context(format!("Metadata for operation '{}' not found", name))
    }

    /// List all registered operation names
    pub fn list_operations(&self) -> Vec<String> {
        let mut names: Vec<_> = self.operations.keys().cloned().collect();
        names.sort();
        names
    }

    /// List all implemented operations
    pub fn list_implemented(&self) -> Vec<String> {
        self.metadata
            .iter()
            .filter(|(_, meta)| meta.implemented)
            .map(|(name, _)| name.clone())
            .collect()
    }

    /// List all operations by category
    pub fn list_by_category(&self, category: OperationCategory) -> Vec<String> {
        self.metadata
            .iter()
            .filter(|(_, meta)| meta.category == category)
            .map(|(name, _)| name.clone())
            .collect()
    }

    /// Count operations by category
    pub fn count_by_category(&self) -> HashMap<OperationCategory, usize> {
        let mut counts = HashMap::new();
        for meta in self.metadata.values() {
            *counts.entry(meta.category).or_insert(0) += 1;
        }
        counts
    }

    /// Validate that an operation supports a given hardware config
    ///
    /// Returns true if the operation has the required backend available.
    pub fn supports_config(&self, operation: &str, config: &HardwareConfig) -> Result<bool> {
        let meta = self.get_metadata(operation)?;
        let required_backend = Backend::from_config(config);

        Ok(meta.has_backend(required_backend))
    }

    /// Get all operations that support a specific backend
    pub fn operations_with_backend(&self, backend: Backend) -> Vec<String> {
        self.metadata
            .iter()
            .filter(|(_, meta)| meta.has_backend(backend))
            .map(|(name, _)| name.clone())
            .collect()
    }

    /// Get statistics about the registry
    pub fn stats(&self) -> RegistryStats {
        let total = self.operations.len();
        let implemented = self.list_implemented().len();
        let by_category = self.count_by_category();

        let neon_operations = self.operations_with_backend(Backend::Neon).len();
        let gpu_operations = self.operations_with_backend(Backend::Gpu).len();
        let neural_operations = self.operations_with_backend(Backend::Neural).len();

        RegistryStats {
            total_operations: total,
            implemented_operations: implemented,
            operations_by_category: by_category,
            neon_operations,
            gpu_operations,
            neural_operations,
        }
    }
}

impl Default for OperationRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Statistics about registered operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistryStats {
    /// Total number of registered operations
    pub total_operations: usize,

    /// Number of implemented operations
    pub implemented_operations: usize,

    /// Count of operations by category
    pub operations_by_category: HashMap<OperationCategory, usize>,

    /// Number of operations with NEON backend
    pub neon_operations: usize,

    /// Number of operations with GPU backend
    pub gpu_operations: usize,

    /// Number of operations with Neural Engine backend
    pub neural_operations: usize,
}

// ============================================================================
// Builder for Operation Registry
// ============================================================================

/// Builder for creating a populated OperationRegistry
///
/// This provides a convenient way to register all operations at once.
pub struct RegistryBuilder {
    registry: OperationRegistry,
}

impl RegistryBuilder {
    /// Create a new builder
    pub fn new() -> Self {
        Self {
            registry: OperationRegistry::new(),
        }
    }

    /// Add an operation with metadata
    pub fn add(
        mut self,
        operation: Arc<dyn PrimitiveOperation>,
        metadata: OperationMetadata,
    ) -> Self {
        self.registry.register(operation, metadata);
        self
    }

    /// Build the final registry
    pub fn build(self) -> OperationRegistry {
        self.registry
    }
}

impl Default for RegistryBuilder {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{OperationOutput, SequenceRecord};

    // Mock operation for testing
    struct MockOperation {
        name: String,
    }

    impl PrimitiveOperation for MockOperation {
        fn name(&self) -> &str {
            &self.name
        }

        fn category(&self) -> OperationCategory {
            OperationCategory::ElementWise
        }

        fn execute_naive(&self, _data: &[SequenceRecord]) -> Result<OperationOutput> {
            Ok(OperationOutput::Count(42))
        }
    }

    #[test]
    fn test_operation_metadata_predictions() {
        let simple = OperationMetadata {
            name: "simple".to_string(),
            category: OperationCategory::ElementWise,
            complexity: 0.30,
            backends: vec![Backend::Naive, Backend::Neon],
            implemented: true,
            description: None,
        };

        assert!(simple.neon_effective());
        assert!(!simple.gpu_candidate());

        let complex = OperationMetadata {
            name: "complex".to_string(),
            category: OperationCategory::Aggregation,
            complexity: 0.61,
            backends: vec![Backend::Naive, Backend::Gpu],
            implemented: true,
            description: None,
        };

        assert!(!complex.neon_effective());
        assert!(complex.gpu_candidate());
    }

    #[test]
    fn test_backend_from_config() {
        let naive_config = HardwareConfig::naive();
        assert_eq!(Backend::from_config(&naive_config), Backend::Naive);

        let mut neon_config = HardwareConfig::naive();
        neon_config.use_neon = true;
        assert_eq!(Backend::from_config(&neon_config), Backend::Neon);

        let mut gpu_config = HardwareConfig::naive();
        gpu_config.use_gpu = true;
        gpu_config.gpu_batch_size = Some(10000);
        assert_eq!(Backend::from_config(&gpu_config), Backend::Gpu);
    }

    #[test]
    fn test_registry_basic() {
        let mut registry = OperationRegistry::new();

        let op = Arc::new(MockOperation {
            name: "test_op".to_string(),
        });

        let metadata = OperationMetadata {
            name: "test_op".to_string(),
            category: OperationCategory::ElementWise,
            complexity: 0.40,
            backends: vec![Backend::Naive, Backend::Neon],
            implemented: true,
            description: Some("Test operation".to_string()),
        };

        registry.register(op, metadata);

        assert!(registry.get("test_op").is_ok());
        assert!(registry.get_metadata("test_op").is_ok());
        assert_eq!(registry.list_operations(), vec!["test_op"]);
        assert_eq!(registry.list_implemented(), vec!["test_op"]);
    }

    #[test]
    fn test_registry_by_category() {
        let mut registry = OperationRegistry::new();

        for i in 0..3 {
            let op = Arc::new(MockOperation {
                name: format!("op{}", i),
            });
            let metadata = OperationMetadata {
                name: format!("op{}", i),
                category: if i == 0 {
                    OperationCategory::ElementWise
                } else {
                    OperationCategory::Filter
                },
                complexity: 0.40,
                backends: vec![Backend::Naive],
                implemented: true,
                description: None,
            };
            registry.register(op, metadata);
        }

        let counts = registry.count_by_category();
        assert_eq!(counts.get(&OperationCategory::ElementWise), Some(&1));
        assert_eq!(counts.get(&OperationCategory::Filter), Some(&2));
    }

    #[test]
    fn test_registry_supports_config() {
        let mut registry = OperationRegistry::new();

        let op = Arc::new(MockOperation {
            name: "test_op".to_string(),
        });

        let metadata = OperationMetadata {
            name: "test_op".to_string(),
            category: OperationCategory::ElementWise,
            complexity: 0.40,
            backends: vec![Backend::Naive, Backend::Neon],
            implemented: true,
            description: None,
        };

        registry.register(op, metadata);

        let naive_config = HardwareConfig::naive();
        assert!(registry.supports_config("test_op", &naive_config).unwrap());

        let mut neon_config = HardwareConfig::naive();
        neon_config.use_neon = true;
        assert!(registry.supports_config("test_op", &neon_config).unwrap());

        let mut gpu_config = HardwareConfig::naive();
        gpu_config.use_gpu = true;
        gpu_config.gpu_batch_size = Some(10000);
        assert!(!registry.supports_config("test_op", &gpu_config).unwrap());
    }

    #[test]
    fn test_registry_builder() {
        let op1 = Arc::new(MockOperation {
            name: "op1".to_string(),
        });
        let meta1 = OperationMetadata {
            name: "op1".to_string(),
            category: OperationCategory::ElementWise,
            complexity: 0.40,
            backends: vec![Backend::Naive],
            implemented: true,
            description: None,
        };

        let op2 = Arc::new(MockOperation {
            name: "op2".to_string(),
        });
        let meta2 = OperationMetadata {
            name: "op2".to_string(),
            category: OperationCategory::Filter,
            complexity: 0.55,
            backends: vec![Backend::Naive, Backend::Neon],
            implemented: true,
            description: None,
        };

        let registry = RegistryBuilder::new()
            .add(op1, meta1)
            .add(op2, meta2)
            .build();

        assert_eq!(registry.list_operations().len(), 2);
        assert_eq!(registry.list_implemented().len(), 2);
    }

    #[test]
    fn test_registry_stats() {
        let mut registry = OperationRegistry::new();

        let op = Arc::new(MockOperation {
            name: "test_op".to_string(),
        });

        let metadata = OperationMetadata {
            name: "test_op".to_string(),
            category: OperationCategory::ElementWise,
            complexity: 0.40,
            backends: vec![Backend::Naive, Backend::Neon, Backend::Gpu],
            implemented: true,
            description: None,
        };

        registry.register(op, metadata);

        let stats = registry.stats();
        assert_eq!(stats.total_operations, 1);
        assert_eq!(stats.implemented_operations, 1);
        assert_eq!(stats.neon_operations, 1);
        assert_eq!(stats.gpu_operations, 1);
        assert_eq!(stats.neural_operations, 0);
    }
}
