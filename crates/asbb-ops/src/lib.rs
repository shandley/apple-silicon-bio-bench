//! Primitive bioinformatics operations for Apple Silicon Bio Bench
//!
//! This crate implements the fundamental sequence operations that we systematically
//! benchmark across different hardware configurations.
//!
//! # Design Philosophy: Apple Silicon First
//!
//! Each operation implements multiple backends:
//! - **Naive**: Baseline scalar implementation (no optimization)
//! - **NEON**: ARM NEON SIMD vectorization (native, not ported from SSE)
//! - **Parallel**: Multi-threaded with Rayon
//! - **GPU**: Metal compute shaders (where applicable)
//! - **Neural/AMX**: Specialized hardware (where applicable)
//!
//! We explore novel approaches rather than porting x86 patterns.

#![allow(dead_code)] // Temporary during development
#![allow(unused_variables)]

pub mod at_content;
pub mod base_counting;
pub mod complexity_score;
pub mod gc_content;
pub mod hamming_distance;
pub mod kmer_counting;
pub mod kmer_extraction;
pub mod length_filter;
pub mod minhash_sketching;
pub mod n_content;
pub mod quality_aggregation;
pub mod quality_filter;
pub mod quality_statistics;
pub mod reverse_complement;
pub mod sequence_length;
pub mod sequence_masking;
pub mod translation;

// Re-export common types
pub use asbb_core::{
    DataCharacteristics, HardwareConfig, OperationCategory, OperationOutput,
    PrimitiveOperation, SequenceRecord,
};
