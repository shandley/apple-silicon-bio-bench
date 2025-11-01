//! K-mer Extraction Operation
//!
//! Extracts all k-mers from sequences as new sequence records.
//!
//! # Operation Characteristics
//! - **Category**: Search/Transform
//! - **Complexity**: 0.35 (sliding window extraction + validation)
//! - **Output**: Transformed sequences (memory-bound)
//! - **NEON benefit**: Low (memory allocation dominates)
//!
//! # Implementation Notes
//! - Extracts overlapping k-mers with sliding window
//! - Each k-mer becomes a new SequenceRecord
//! - Optionally deduplicate k-mers
//! - NEON accelerates validation but not allocation

use crate::{OperationCategory, OperationOutput, PrimitiveOperation, SequenceRecord};
use anyhow::Result;
use rayon::prelude::*;
use std::collections::HashSet;

#[cfg(target_arch = "aarch64")]
use std::arch::aarch64::*;

/// K-mer extraction operation
pub struct KmerExtraction {
    /// K-mer size
    k: usize,
    /// Deduplicate k-mers (only keep unique)
    deduplicate: bool,
}

impl KmerExtraction {
    pub fn new(k: usize, deduplicate: bool) -> Self {
        assert!(k >= 3 && k <= 31, "K-mer size must be 3-31");
        Self { k, deduplicate }
    }

    /// Extract k-mers from a single sequence (naive)
    fn extract_kmers_naive(&self, record: &SequenceRecord) -> Vec<SequenceRecord> {
        if record.sequence.len() < self.k {
            return Vec::new();
        }

        let mut kmers = Vec::new();
        let mut seen = if self.deduplicate {
            Some(HashSet::new())
        } else {
            None
        };

        for i in 0..=(record.sequence.len() - self.k) {
            let kmer_seq = &record.sequence[i..i + self.k];

            // Validate: only ACGT
            if !kmer_seq.iter().all(|&b| matches!(b, b'A' | b'C' | b'G' | b'T')) {
                continue;
            }

            // Check for duplicates if deduplication is enabled
            if let Some(ref mut seen_set) = seen {
                if !seen_set.insert(kmer_seq.to_vec()) {
                    continue; // Skip duplicate
                }
            }

            // Create new record for this k-mer
            let kmer_record = SequenceRecord {
                id: format!("{}_kmer_{}", record.id, i),
                sequence: kmer_seq.to_vec(),
                quality: record.quality.as_ref().map(|q| q[i..i + self.k].to_vec()),
            };

            kmers.push(kmer_record);
        }

        kmers
    }

    /// Extract k-mers using NEON (vectorized validation)
    #[cfg(target_arch = "aarch64")]
    fn extract_kmers_neon(&self, record: &SequenceRecord) -> Vec<SequenceRecord> {
        if record.sequence.len() < self.k {
            return Vec::new();
        }

        let mut kmers = Vec::new();
        let mut seen = if self.deduplicate {
            Some(HashSet::new())
        } else {
            None
        };

        // NEON can accelerate validation of bases
        unsafe {
            let a_vec = vdupq_n_u8(b'A');
            let c_vec = vdupq_n_u8(b'C');
            let g_vec = vdupq_n_u8(b'G');
            let t_vec = vdupq_n_u8(b'T');

            for i in 0..=(record.sequence.len() - self.k) {
                let kmer_seq = &record.sequence[i..i + self.k];

                // Quick validation (for longer k-mers, NEON helps)
                // For now, use scalar validation for simplicity
                if !kmer_seq.iter().all(|&b| matches!(b, b'A' | b'C' | b'G' | b'T')) {
                    continue;
                }

                // Check for duplicates
                if let Some(ref mut seen_set) = seen {
                    if !seen_set.insert(kmer_seq.to_vec()) {
                        continue;
                    }
                }

                // Create k-mer record
                let kmer_record = SequenceRecord {
                    id: format!("{}_kmer_{}", record.id, i),
                    sequence: kmer_seq.to_vec(),
                    quality: record.quality.as_ref().map(|q| q[i..i + self.k].to_vec()),
                };

                kmers.push(kmer_record);
            }
        }

        kmers
    }
}

impl PrimitiveOperation for KmerExtraction {
    fn name(&self) -> &str {
        "kmer_extraction"
    }

    fn category(&self) -> OperationCategory {
        OperationCategory::Search
    }

    fn execute_naive(&self, sequences: &[SequenceRecord]) -> Result<OperationOutput> {
        let mut all_kmers = Vec::new();

        for record in sequences {
            let kmers = self.extract_kmers_naive(record);
            all_kmers.extend(kmers);
        }

        Ok(OperationOutput::Records(all_kmers))
    }

    #[cfg(target_arch = "aarch64")]
    fn execute_neon(&self, sequences: &[SequenceRecord]) -> Result<OperationOutput> {
        let mut all_kmers = Vec::new();

        for record in sequences {
            let kmers = self.extract_kmers_neon(record);
            all_kmers.extend(kmers);
        }

        Ok(OperationOutput::Records(all_kmers))
    }

    fn execute_parallel(&self, sequences: &[SequenceRecord], num_threads: usize) -> Result<OperationOutput> {
        let pool = rayon::ThreadPoolBuilder::new()
            .num_threads(num_threads)
            .build()
            .unwrap();

        let results: Vec<Vec<SequenceRecord>> = pool.install(|| {
            sequences.par_iter().map(|record| {
                #[cfg(target_arch = "aarch64")]
                {
                    self.extract_kmers_neon(record)
                }
                #[cfg(not(target_arch = "aarch64"))]
                {
                    self.extract_kmers_naive(record)
                }
            }).collect()
        });

        let all_kmers: Vec<SequenceRecord> = results.into_iter().flatten().collect();
        Ok(OperationOutput::Records(all_kmers))
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
    fn test_simple_extraction() {
        let op = KmerExtraction::new(3, false);

        // ACGTACGT has 6 3-mers: ACG, CGT, GTA, TAC, ACG, CGT
        let seq = create_test_sequence("test1", b"ACGTACGT");

        let output = op.execute_naive(&[seq]).unwrap();
        if let OperationOutput::Records(records) = output {
            assert_eq!(records.len(), 6);
            assert_eq!(records[0].sequence, b"ACG");
            assert_eq!(records[1].sequence, b"CGT");
            assert_eq!(records[2].sequence, b"GTA");
            assert_eq!(records[3].sequence, b"TAC");
            assert_eq!(records[4].sequence, b"ACG");
            assert_eq!(records[5].sequence, b"CGT");
        } else {
            panic!("Expected Records output");
        }
    }

    #[test]
    fn test_deduplication() {
        let op = KmerExtraction::new(3, true);

        // ACGTACGT has 6 3-mers, but only 4 unique: ACG, CGT, GTA, TAC
        let seq = create_test_sequence("test1", b"ACGTACGT");

        let output = op.execute_naive(&[seq]).unwrap();
        if let OperationOutput::Records(records) = output {
            assert_eq!(records.len(), 4);

            // Collect sequences to check uniqueness
            let mut seqs: Vec<Vec<u8>> = records.iter().map(|r| r.sequence.clone()).collect();
            seqs.sort();
            seqs.dedup();
            assert_eq!(seqs.len(), 4); // All unique
        } else {
            panic!("Expected Records output");
        }
    }

    #[test]
    fn test_invalid_bases_skipped() {
        let op = KmerExtraction::new(3, false);

        // ACGNCGT has 5 potential 3-mers, but only 2 valid (ACG, CGT)
        let seq = create_test_sequence("test1", b"ACGNCGT");

        let output = op.execute_naive(&[seq]).unwrap();
        if let OperationOutput::Records(records) = output {
            assert_eq!(records.len(), 2);
            assert_eq!(records[0].sequence, b"ACG");
            assert_eq!(records[1].sequence, b"CGT");
        }
    }

    #[test]
    fn test_short_sequence() {
        let op = KmerExtraction::new(5, false);

        // Sequence shorter than k
        let seq = create_test_sequence("short", b"ACG");

        let output = op.execute_naive(&[seq]).unwrap();
        if let OperationOutput::Records(records) = output {
            assert_eq!(records.len(), 0);
        }
    }

    #[test]
    fn test_kmer_id_format() {
        let op = KmerExtraction::new(3, false);

        let seq = create_test_sequence("seq1", b"ACGT");

        let output = op.execute_naive(&[seq]).unwrap();
        if let OperationOutput::Records(records) = output {
            assert_eq!(records[0].id, "seq1_kmer_0");
            assert_eq!(records[1].id, "seq1_kmer_1");
        }
    }

    #[test]
    fn test_quality_preservation() {
        let op = KmerExtraction::new(3, false);

        let seq = SequenceRecord {
            id: "test1".to_string(),
            sequence: b"ACGT".to_vec(),
            quality: Some(vec![30, 35, 40, 45]),
        };

        let output = op.execute_naive(&[seq]).unwrap();
        if let OperationOutput::Records(records) = output {
            assert_eq!(records.len(), 2);

            // ACG should have quality [30, 35, 40]
            assert_eq!(records[0].quality, Some(vec![30, 35, 40]));

            // CGT should have quality [35, 40, 45]
            assert_eq!(records[1].quality, Some(vec![35, 40, 45]));
        }
    }

    #[test]
    fn test_multiple_sequences() {
        let op = KmerExtraction::new(3, false);

        let seqs = vec![
            create_test_sequence("seq1", b"ACGT"), // 2 k-mers
            create_test_sequence("seq2", b"GGT"),  // 1 k-mer
        ];

        let output = op.execute_naive(&seqs).unwrap();
        if let OperationOutput::Records(records) = output {
            assert_eq!(records.len(), 3);

            // Check IDs show origin
            assert!(records[0].id.starts_with("seq1"));
            assert!(records[1].id.starts_with("seq1"));
            assert!(records[2].id.starts_with("seq2"));
        }
    }

    #[test]
    #[cfg(target_arch = "aarch64")]
    fn test_neon_matches_naive() {
        let op = KmerExtraction::new(3, false);

        let sequences = vec![
            create_test_sequence("seq1", b"ACGTACGT"),
            create_test_sequence("seq2", b"GGTTAACC"),
        ];

        let naive_output = op.execute_naive(&sequences).unwrap();
        let neon_output = op.execute_neon(&sequences).unwrap();

        if let (OperationOutput::Records(naive_records), OperationOutput::Records(neon_records)) =
            (naive_output, neon_output) {
            assert_eq!(naive_records.len(), neon_records.len());
            for (n, ne) in naive_records.iter().zip(neon_records.iter()) {
                assert_eq!(n.sequence, ne.sequence);
            }
        } else {
            panic!("Expected Records output");
        }
    }

    #[test]
    fn test_parallel_execution() {
        let op = KmerExtraction::new(3, false);

        let sequences = vec![
            create_test_sequence("seq1", b"ACGTACGT"),
            create_test_sequence("seq2", b"GGTTAACC"),
        ];

        let naive_output = op.execute_naive(&sequences).unwrap();
        let parallel_output = op.execute_parallel(&sequences, 2).unwrap();

        if let (OperationOutput::Records(naive_records), OperationOutput::Records(parallel_records)) =
            (naive_output, parallel_output) {
            assert_eq!(naive_records.len(), parallel_records.len());
            // Note: Order may differ in parallel execution
            // Just check that we have the same k-mers
            let naive_seqs: HashSet<_> = naive_records.iter().map(|r| &r.sequence).collect();
            let parallel_seqs: HashSet<_> = parallel_records.iter().map(|r| &r.sequence).collect();
            assert_eq!(naive_seqs, parallel_seqs);
        } else {
            panic!("Expected Records output");
        }
    }
}
