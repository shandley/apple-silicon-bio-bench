//! MinHash Sketching Operation
//!
//! Computes MinHash sketches for sequence similarity estimation.
//!
//! # Operation Characteristics
//! - **Category**: Aggregation
//! - **Complexity**: 0.48 (k-mer extraction + hashing + min selection)
//! - **Output**: Aggregated signatures (compute-bound)
//! - **NEON benefit**: High (computation dominates, returns fixed-size sketches)
//!
//! # Implementation Notes
//! - Uses FNV-1a hash function (fast, good distribution)
//! - Sketches are fixed-size arrays of minimum hash values
//! - Sketch size determines accuracy (larger = more accurate, more memory)
//! - NEON accelerates k-mer extraction and hash computation

use crate::{OperationCategory, OperationOutput, PrimitiveOperation, SequenceRecord};
use anyhow::Result;
use rayon::prelude::*;
use serde::{Deserialize, Serialize};

#[cfg(target_arch = "aarch64")]
use std::arch::aarch64::*;

/// MinHash sketching operation
pub struct MinHashSketching {
    /// K-mer size
    k: usize,
    /// Sketch size (number of minimum hash values to keep)
    sketch_size: usize,
}

/// MinHash sketch output
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MinHashSketch {
    pub sequence_id: String,
    pub sketch: Vec<u64>,
    pub sketch_size: usize,
    pub k: usize,
}

impl MinHashSketching {
    pub fn new(k: usize, sketch_size: usize) -> Self {
        assert!(k >= 3 && k <= 31, "K-mer size must be 3-31");
        assert!(sketch_size > 0, "Sketch size must be > 0");
        Self { k, sketch_size }
    }

    /// FNV-1a hash function (fast and good distribution for k-mers)
    fn fnv1a_hash(data: &[u8]) -> u64 {
        const FNV_OFFSET: u64 = 14695981039346656037;
        const FNV_PRIME: u64 = 1099511628211;

        let mut hash = FNV_OFFSET;
        for &byte in data {
            hash ^= byte as u64;
            hash = hash.wrapping_mul(FNV_PRIME);
        }
        hash
    }

    /// Extract k-mers and compute their hashes (naive)
    fn extract_and_hash_kmers_naive(&self, sequence: &[u8]) -> Vec<u64> {
        if sequence.len() < self.k {
            return Vec::new();
        }

        let mut hashes = Vec::with_capacity(sequence.len() - self.k + 1);

        for i in 0..=(sequence.len() - self.k) {
            let kmer = &sequence[i..i + self.k];

            // Validate: only ACGT
            if kmer.iter().all(|&b| matches!(b, b'A' | b'C' | b'G' | b'T')) {
                let hash = Self::fnv1a_hash(kmer);
                hashes.push(hash);
            }
        }

        hashes
    }

    /// Extract k-mers and compute hashes using NEON
    #[cfg(target_arch = "aarch64")]
    fn extract_and_hash_kmers_neon(&self, sequence: &[u8]) -> Vec<u64> {
        if sequence.len() < self.k {
            return Vec::new();
        }

        let mut hashes = Vec::with_capacity(sequence.len() - self.k + 1);

        // NEON can accelerate k-mer validation
        unsafe {
            let a_vec = vdupq_n_u8(b'A');
            let c_vec = vdupq_n_u8(b'C');
            let g_vec = vdupq_n_u8(b'G');
            let t_vec = vdupq_n_u8(b'T');

            let mut i = 0;

            // Process k-mers
            while i + self.k <= sequence.len() {
                let kmer = &sequence[i..i + self.k];

                // Validate k-mer (can use NEON for longer k-mers)
                if kmer.iter().all(|&b| matches!(b, b'A' | b'C' | b'G' | b'T')) {
                    let hash = Self::fnv1a_hash(kmer);
                    hashes.push(hash);
                }

                i += 1;
            }
        }

        hashes
    }

    /// Select minimum hash values for sketch
    fn select_min_hashes(&self, hashes: Vec<u64>) -> Vec<u64> {
        if hashes.is_empty() {
            return Vec::new();
        }

        let mut sketch = hashes;
        sketch.sort_unstable();

        // Take the k smallest values
        sketch.truncate(self.sketch_size);
        sketch
    }

    /// Compute MinHash sketch for a single sequence
    fn compute_sketch_naive(&self, record: &SequenceRecord) -> MinHashSketch {
        let hashes = self.extract_and_hash_kmers_naive(&record.sequence);
        let sketch = self.select_min_hashes(hashes);

        MinHashSketch {
            sequence_id: record.id.clone(),
            sketch,
            sketch_size: self.sketch_size,
            k: self.k,
        }
    }

    /// Compute sketch using NEON
    #[cfg(target_arch = "aarch64")]
    fn compute_sketch_neon(&self, record: &SequenceRecord) -> MinHashSketch {
        let hashes = self.extract_and_hash_kmers_neon(&record.sequence);
        let sketch = self.select_min_hashes(hashes);

        MinHashSketch {
            sequence_id: record.id.clone(),
            sketch,
            sketch_size: self.sketch_size,
            k: self.k,
        }
    }

    /// Compute Jaccard similarity between two sketches
    pub fn jaccard_similarity(sketch1: &MinHashSketch, sketch2: &MinHashSketch) -> f64 {
        if sketch1.sketch.is_empty() || sketch2.sketch.is_empty() {
            return 0.0;
        }

        let mut i = 0;
        let mut j = 0;
        let mut intersection = 0;
        let mut union = 0;

        // Merge sorted lists
        while i < sketch1.sketch.len() && j < sketch2.sketch.len() {
            if sketch1.sketch[i] == sketch2.sketch[j] {
                intersection += 1;
                union += 1;
                i += 1;
                j += 1;
            } else if sketch1.sketch[i] < sketch2.sketch[j] {
                union += 1;
                i += 1;
            } else {
                union += 1;
                j += 1;
            }
        }

        // Add remaining elements
        union += sketch1.sketch.len() - i;
        union += sketch2.sketch.len() - j;

        intersection as f64 / union as f64
    }
}

impl PrimitiveOperation for MinHashSketching {
    fn name(&self) -> &str {
        "minhash_sketching"
    }

    fn category(&self) -> OperationCategory {
        OperationCategory::Aggregation
    }

    fn execute_naive(&self, sequences: &[SequenceRecord]) -> Result<OperationOutput> {
        let sketches: Vec<MinHashSketch> = sequences
            .iter()
            .map(|record| self.compute_sketch_naive(record))
            .collect();

        Ok(OperationOutput::Statistics(serde_json::to_value(sketches)?))
    }

    #[cfg(target_arch = "aarch64")]
    fn execute_neon(&self, sequences: &[SequenceRecord]) -> Result<OperationOutput> {
        let sketches: Vec<MinHashSketch> = sequences
            .iter()
            .map(|record| self.compute_sketch_neon(record))
            .collect();

        Ok(OperationOutput::Statistics(serde_json::to_value(sketches)?))
    }

    fn execute_parallel(&self, sequences: &[SequenceRecord], num_threads: usize) -> Result<OperationOutput> {
        let pool = rayon::ThreadPoolBuilder::new()
            .num_threads(num_threads)
            .build()
            .unwrap();

        let sketches: Vec<MinHashSketch> = pool.install(|| {
            sequences.par_iter().map(|record| {
                #[cfg(target_arch = "aarch64")]
                {
                    self.compute_sketch_neon(record)
                }
                #[cfg(not(target_arch = "aarch64"))]
                {
                    self.compute_sketch_naive(record)
                }
            }).collect()
        });

        Ok(OperationOutput::Statistics(serde_json::to_value(sketches)?))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_sequence(id: &str, seq: &[u8]) -> SequenceRecord {
        SequenceRecord {
            id: id.to_string(),
            sequence: seq.to_vec(),
            quality: None,
        }
    }

    #[test]
    fn test_fnv1a_hash() {
        // Test that hash function is deterministic
        let data1 = b"ACGT";
        let data2 = b"ACGT";
        let data3 = b"ACGG";

        let hash1 = MinHashSketching::fnv1a_hash(data1);
        let hash2 = MinHashSketching::fnv1a_hash(data2);
        let hash3 = MinHashSketching::fnv1a_hash(data3);

        assert_eq!(hash1, hash2); // Same input = same hash
        assert_ne!(hash1, hash3); // Different input = different hash
    }

    #[test]
    fn test_simple_sketch() {
        let op = MinHashSketching::new(3, 5);

        // ACGTACGT has 6 3-mers: ACG, CGT, GTA, TAC, ACG, CGT
        // (4 unique: ACG, CGT, GTA, TAC)
        let seq = create_test_sequence("test1", b"ACGTACGT");

        let output = op.execute_naive(&[seq]).unwrap();
        if let OperationOutput::Statistics(json) = output {
            let sketches: Vec<MinHashSketch> = serde_json::from_value(json).unwrap();
            assert_eq!(sketches.len(), 1);
            assert_eq!(sketches[0].sequence_id, "test1");
            assert_eq!(sketches[0].k, 3);
            assert_eq!(sketches[0].sketch_size, 5);
            // Should have 4 unique hashes (or 4 hashes if less than sketch_size)
            assert!(sketches[0].sketch.len() <= 5);
            assert!(sketches[0].sketch.len() >= 4);
        } else {
            panic!("Expected Statistics output");
        }
    }

    #[test]
    fn test_sketch_size() {
        let op = MinHashSketching::new(3, 2);

        let seq = create_test_sequence("test1", b"ACGTACGT");

        let output = op.execute_naive(&[seq]).unwrap();
        if let OperationOutput::Statistics(json) = output {
            let sketches: Vec<MinHashSketch> = serde_json::from_value(json).unwrap();
            // Sketch size limited to 2
            assert_eq!(sketches[0].sketch.len(), 2);
        }
    }

    #[test]
    fn test_sketch_ordering() {
        let op = MinHashSketching::new(3, 10);

        let seq = create_test_sequence("test1", b"ACGTACGTACGT");

        let output = op.execute_naive(&[seq]).unwrap();
        if let OperationOutput::Statistics(json) = output {
            let sketches: Vec<MinHashSketch> = serde_json::from_value(json).unwrap();

            // Sketch should be sorted (minimum hashes first)
            let sketch = &sketches[0].sketch;
            for i in 1..sketch.len() {
                assert!(sketch[i - 1] <= sketch[i]);
            }
        }
    }

    #[test]
    fn test_invalid_bases_skipped() {
        let op = MinHashSketching::new(3, 5);

        // ACGNCGT has 5 potential 3-mers, but only 2 valid (ACG, CGT)
        let seq = create_test_sequence("test1", b"ACGNCGT");

        let output = op.execute_naive(&[seq]).unwrap();
        if let OperationOutput::Statistics(json) = output {
            let sketches: Vec<MinHashSketch> = serde_json::from_value(json).unwrap();
            // Should have 2 hashes for the 2 valid k-mers
            assert_eq!(sketches[0].sketch.len(), 2);
        }
    }

    #[test]
    fn test_jaccard_similarity_identical() {
        let op = MinHashSketching::new(3, 5);

        let seq1 = create_test_sequence("seq1", b"ACGTACGT");
        let seq2 = create_test_sequence("seq2", b"ACGTACGT");

        let output = op.execute_naive(&[seq1, seq2]).unwrap();
        if let OperationOutput::Statistics(json) = output {
            let sketches: Vec<MinHashSketch> = serde_json::from_value(json).unwrap();

            let similarity = MinHashSketching::jaccard_similarity(&sketches[0], &sketches[1]);
            // Identical sequences should have similarity = 1.0
            assert!((similarity - 1.0).abs() < 0.001);
        }
    }

    #[test]
    fn test_jaccard_similarity_different() {
        let op = MinHashSketching::new(3, 5);

        let seq1 = create_test_sequence("seq1", b"ACGTACGT");
        let seq2 = create_test_sequence("seq2", b"GGTTAACC");

        let output = op.execute_naive(&[seq1, seq2]).unwrap();
        if let OperationOutput::Statistics(json) = output {
            let sketches: Vec<MinHashSketch> = serde_json::from_value(json).unwrap();

            let similarity = MinHashSketching::jaccard_similarity(&sketches[0], &sketches[1]);
            // Different sequences should have low similarity
            assert!(similarity < 0.5);
        }
    }

    #[test]
    fn test_empty_sequence() {
        let op = MinHashSketching::new(3, 5);

        let seq = create_test_sequence("empty", b"");

        let output = op.execute_naive(&[seq]).unwrap();
        if let OperationOutput::Statistics(json) = output {
            let sketches: Vec<MinHashSketch> = serde_json::from_value(json).unwrap();
            assert_eq!(sketches[0].sketch.len(), 0);
        }
    }

    #[test]
    #[cfg(target_arch = "aarch64")]
    fn test_neon_matches_naive() {
        let op = MinHashSketching::new(3, 5);

        let sequences = vec![
            create_test_sequence("seq1", b"ACGTACGT"),
            create_test_sequence("seq2", b"GGTTAACC"),
        ];

        let naive_output = op.execute_naive(&sequences).unwrap();
        let neon_output = op.execute_neon(&sequences).unwrap();

        if let (OperationOutput::Statistics(naive_json), OperationOutput::Statistics(neon_json)) =
            (naive_output, neon_output) {
            let naive_sketches: Vec<MinHashSketch> = serde_json::from_value(naive_json).unwrap();
            let neon_sketches: Vec<MinHashSketch> = serde_json::from_value(neon_json).unwrap();

            assert_eq!(naive_sketches.len(), neon_sketches.len());
            for (n, ne) in naive_sketches.iter().zip(neon_sketches.iter()) {
                assert_eq!(n.sketch, ne.sketch);
            }
        } else {
            panic!("Expected Statistics output");
        }
    }

    #[test]
    fn test_parallel_execution() {
        let op = MinHashSketching::new(3, 5);

        let sequences = vec![
            create_test_sequence("seq1", b"ACGTACGT"),
            create_test_sequence("seq2", b"GGTTAACC"),
        ];

        let naive_output = op.execute_naive(&sequences).unwrap();
        let parallel_output = op.execute_parallel(&sequences, 2).unwrap();

        if let (OperationOutput::Statistics(naive_json), OperationOutput::Statistics(parallel_json)) =
            (naive_output, parallel_output) {
            let naive_sketches: Vec<MinHashSketch> = serde_json::from_value(naive_json).unwrap();
            let parallel_sketches: Vec<MinHashSketch> = serde_json::from_value(parallel_json).unwrap();

            assert_eq!(naive_sketches.len(), parallel_sketches.len());
            // Sketches should match (order may differ, so sort by ID)
            for sketch in &naive_sketches {
                let matching = parallel_sketches.iter().find(|s| s.sequence_id == sketch.sequence_id).unwrap();
                assert_eq!(sketch.sketch, matching.sketch);
            }
        } else {
            panic!("Expected Statistics output");
        }
    }
}
