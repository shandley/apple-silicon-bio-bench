//! Adapter Trimming Operation
//!
//! Detects and removes adapter sequences from reads.
//!
//! # Operation Characteristics
//! - **Category**: Filtering/Transform
//! - **Complexity**: 0.55 (pattern matching + trimming)
//! - **Output**: Transformed sequences (memory-bound)
//! - **NEON benefit**: Moderate (string matching vectorizable, but memory allocation present)
//!
//! # Implementation Notes
//! - Uses suffix-prefix matching to detect adapters
//! - Minimum overlap required for detection
//! - Handles 3' adapter trimming (most common)
//! - NEON accelerates pattern matching

use crate::{OperationCategory, OperationOutput, PrimitiveOperation, SequenceRecord};
use anyhow::Result;
use rayon::prelude::*;

#[cfg(target_arch = "aarch64")]
use std::arch::aarch64::*;

/// Adapter trimming operation
pub struct AdapterTrimming {
    /// Adapter sequence to detect and remove
    adapter: Vec<u8>,
    /// Minimum overlap length to detect adapter
    min_overlap: usize,
    /// Minimum sequence length after trimming
    min_length: usize,
}

impl AdapterTrimming {
    pub fn new(adapter: Vec<u8>, min_overlap: usize, min_length: usize) -> Self {
        assert!(min_overlap >= 3, "Minimum overlap must be >= 3");
        assert!(min_overlap <= adapter.len(), "Minimum overlap cannot exceed adapter length");
        Self {
            adapter,
            min_overlap,
            min_length,
        }
    }

    /// Find adapter position in sequence (naive implementation)
    fn find_adapter_naive(&self, sequence: &[u8]) -> Option<usize> {
        // Check for adapter at each position
        for pos in 0..sequence.len() {
            let remaining = sequence.len() - pos;

            // Check overlaps from longest to shortest
            for overlap in (self.min_overlap..=std::cmp::min(remaining, self.adapter.len())).rev() {
                let seq_suffix = &sequence[pos..pos + overlap];
                let adapter_prefix = &self.adapter[..overlap];

                if seq_suffix == adapter_prefix {
                    return Some(pos);
                }
            }
        }

        None
    }

    /// Find adapter using NEON (vectorized matching)
    #[cfg(target_arch = "aarch64")]
    fn find_adapter_neon(&self, sequence: &[u8]) -> Option<usize> {
        if sequence.len() < self.min_overlap || self.adapter.is_empty() {
            return None;
        }

        unsafe {
            // For each position, try to match adapter prefix
            for pos in 0..sequence.len() {
                let remaining = sequence.len() - pos;

                // Try different overlap lengths
                for overlap in (self.min_overlap..=std::cmp::min(remaining, self.adapter.len())).rev() {
                    let seq_suffix = &sequence[pos..pos + overlap];
                    let adapter_prefix = &self.adapter[..overlap];

                    // Use NEON for comparison if overlap is >= 16
                    if overlap >= 16 {
                        let mut match_count = 0;
                        let mut i = 0;

                        // Process 16 bytes at a time
                        while i + 16 <= overlap {
                            let seq_vec = vld1q_u8(seq_suffix.as_ptr().add(i));
                            let adapter_vec = vld1q_u8(adapter_prefix.as_ptr().add(i));

                            let eq_mask = vceqq_u8(seq_vec, adapter_vec);

                            // Count matches (all must match)
                            let mut mask_bytes = [0u8; 16];
                            vst1q_u8(mask_bytes.as_mut_ptr(), eq_mask);

                            if mask_bytes.iter().all(|&b| b == 0xFF) {
                                match_count += 16;
                            } else {
                                break;
                            }

                            i += 16;
                        }

                        // Check remainder
                        let remainder_matches = seq_suffix[i..].iter()
                            .zip(adapter_prefix[i..].iter())
                            .all(|(a, b)| a == b);

                        if match_count == i && remainder_matches {
                            return Some(pos);
                        }
                    } else {
                        // Short overlap, use scalar comparison
                        if seq_suffix == adapter_prefix {
                            return Some(pos);
                        }
                    }
                }
            }
        }

        None
    }

    /// Trim sequence at adapter position
    fn trim_sequence(&self, record: &SequenceRecord, adapter_pos: usize) -> SequenceRecord {
        let trimmed_seq = record.sequence[..adapter_pos].to_vec();
        let trimmed_qual = record.quality.as_ref().map(|q| q[..adapter_pos].to_vec());

        SequenceRecord {
            id: format!("{}_trimmed", record.id),
            sequence: trimmed_seq,
            quality: trimmed_qual,
        }
    }
}

impl PrimitiveOperation for AdapterTrimming {
    fn name(&self) -> &str {
        "adapter_trimming"
    }

    fn category(&self) -> OperationCategory {
        OperationCategory::Filter
    }

    fn execute_naive(&self, sequences: &[SequenceRecord]) -> Result<OperationOutput> {
        let mut trimmed = Vec::new();

        for record in sequences {
            let result = if let Some(adapter_pos) = self.find_adapter_naive(&record.sequence) {
                // Adapter found, trim it
                let trimmed_record = self.trim_sequence(record, adapter_pos);

                // Only keep if meets minimum length
                if trimmed_record.sequence.len() >= self.min_length {
                    Some(trimmed_record)
                } else {
                    None
                }
            } else {
                // No adapter found, keep original
                Some(record.clone())
            };

            if let Some(rec) = result {
                trimmed.push(rec);
            }
        }

        Ok(OperationOutput::Records(trimmed))
    }

    #[cfg(target_arch = "aarch64")]
    fn execute_neon(&self, sequences: &[SequenceRecord]) -> Result<OperationOutput> {
        let mut trimmed = Vec::new();

        for record in sequences {
            let result = if let Some(adapter_pos) = self.find_adapter_neon(&record.sequence) {
                let trimmed_record = self.trim_sequence(record, adapter_pos);

                if trimmed_record.sequence.len() >= self.min_length {
                    Some(trimmed_record)
                } else {
                    None
                }
            } else {
                Some(record.clone())
            };

            if let Some(rec) = result {
                trimmed.push(rec);
            }
        }

        Ok(OperationOutput::Records(trimmed))
    }

    fn execute_parallel(&self, sequences: &[SequenceRecord], num_threads: usize) -> Result<OperationOutput> {
        let pool = rayon::ThreadPoolBuilder::new()
            .num_threads(num_threads)
            .build()
            .unwrap();

        let results: Vec<Option<SequenceRecord>> = pool.install(|| {
            sequences.par_iter().map(|record| {
                #[cfg(target_arch = "aarch64")]
                let adapter_pos = self.find_adapter_neon(&record.sequence);
                #[cfg(not(target_arch = "aarch64"))]
                let adapter_pos = self.find_adapter_naive(&record.sequence);

                if let Some(pos) = adapter_pos {
                    let trimmed_record = self.trim_sequence(record, pos);
                    if trimmed_record.sequence.len() >= self.min_length {
                        Some(trimmed_record)
                    } else {
                        None
                    }
                } else {
                    Some(record.clone())
                }
            }).collect()
        });

        let trimmed: Vec<SequenceRecord> = results.into_iter().flatten().collect();
        Ok(OperationOutput::Records(trimmed))
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
    fn test_no_adapter() {
        let adapter = b"AGATCGGAAGAG".to_vec();
        let op = AdapterTrimming::new(adapter, 5, 10);

        let seq = create_test_sequence("seq1", b"ACGTACGTACGTACGTACGT");

        let output = op.execute_naive(&[seq]).unwrap();
        if let OperationOutput::Records(records) = output {
            assert_eq!(records.len(), 1);
            assert_eq!(records[0].sequence, b"ACGTACGTACGTACGTACGT");
        } else {
            panic!("Expected Records output");
        }
    }

    #[test]
    fn test_full_adapter_at_end() {
        let adapter = b"AGATCGGAAGAG".to_vec();
        let op = AdapterTrimming::new(adapter, 5, 10);

        // Sequence with full adapter at end
        let seq = create_test_sequence("seq1", b"ACGTACGTACGTAGATCGGAAGAG");

        let output = op.execute_naive(&[seq]).unwrap();
        if let OperationOutput::Records(records) = output {
            assert_eq!(records.len(), 1);
            assert_eq!(records[0].sequence, b"ACGTACGTACGT");
            assert!(records[0].id.contains("trimmed"));
        }
    }

    #[test]
    fn test_partial_adapter() {
        let adapter = b"AGATCGGAAGAG".to_vec();
        let op = AdapterTrimming::new(adapter, 5, 10);

        // Sequence with partial adapter (only 6 bases)
        let seq = create_test_sequence("seq1", b"ACGTACGTACGTAGATCG");

        let output = op.execute_naive(&[seq]).unwrap();
        if let OperationOutput::Records(records) = output {
            assert_eq!(records.len(), 1);
            // Should be trimmed at position 12 (where AGATCG starts)
            assert_eq!(records[0].sequence, b"ACGTACGTACGT");
        }
    }

    #[test]
    fn test_min_overlap_threshold() {
        let adapter = b"AGATCGGAAGAG".to_vec();
        let op = AdapterTrimming::new(adapter, 8, 10);

        // Partial adapter with only 6 bases (below threshold)
        let seq = create_test_sequence("seq1", b"ACGTACGTACGTAGATCG");

        let output = op.execute_naive(&[seq]).unwrap();
        if let OperationOutput::Records(records) = output {
            // Should NOT be trimmed (overlap < 8)
            assert_eq!(records[0].sequence, b"ACGTACGTACGTAGATCG");
        }
    }

    #[test]
    fn test_min_length_filter() {
        let adapter = b"AGATCGGAAGAG".to_vec();
        let op = AdapterTrimming::new(adapter, 5, 20);

        // Short sequence with adapter (result < min_length)
        let seq = create_test_sequence("seq1", b"ACGTACGTAGATCGGAAGAG");

        let output = op.execute_naive(&[seq]).unwrap();
        if let OperationOutput::Records(records) = output {
            // Should be filtered out (trimmed length = 8 < 20)
            assert_eq!(records.len(), 0);
        }
    }

    #[test]
    fn test_quality_preservation() {
        let adapter = b"AGATCGGAAGAG".to_vec();
        let op = AdapterTrimming::new(adapter, 5, 5);

        let seq = SequenceRecord {
            id: "seq1".to_string(),
            sequence: b"ACGTACGTACGTAGATCGGAAGAG".to_vec(),
            quality: Some(vec![30; 24]),
        };

        let output = op.execute_naive(&[seq]).unwrap();
        if let OperationOutput::Records(records) = output {
            assert_eq!(records[0].sequence.len(), 12);
            assert_eq!(records[0].quality.as_ref().unwrap().len(), 12);
        }
    }

    #[test]
    fn test_multiple_sequences() {
        let adapter = b"AGATCGGAAGAG".to_vec();
        let op = AdapterTrimming::new(adapter, 5, 3); // min_length = 3 to keep seq3

        let sequences = vec![
            create_test_sequence("seq1", b"ACGTACGTAGATCGGAAGAG"), // Has adapter
            create_test_sequence("seq2", b"GGTTAACCGGTTAACC"),     // No adapter
            create_test_sequence("seq3", b"TTTTAGATCGGAA"),        // Has partial adapter
        ];

        let output = op.execute_naive(&sequences).unwrap();
        if let OperationOutput::Records(records) = output {
            assert_eq!(records.len(), 3);

            // seq1 should be trimmed
            assert_eq!(records[0].sequence, b"ACGTACGT");

            // seq2 should be unchanged
            assert_eq!(records[1].sequence, b"GGTTAACCGGTTAACC");

            // seq3 should be trimmed
            assert_eq!(records[2].sequence, b"TTTT");
        }
    }

    #[test]
    fn test_adapter_at_start() {
        let adapter = b"AGATCGGAAGAG".to_vec();
        let op = AdapterTrimming::new(adapter, 5, 0); // min_length = 0 to allow empty

        // Adapter at start (unusual but valid)
        let seq = create_test_sequence("seq1", b"AGATCGGAAGAGACGTACGT");

        let output = op.execute_naive(&[seq]).unwrap();
        if let OperationOutput::Records(records) = output {
            // Should be trimmed to empty (start of adapter at position 0)
            assert_eq!(records.len(), 1);
            assert_eq!(records[0].sequence, b"");
        }
    }

    #[test]
    #[cfg(target_arch = "aarch64")]
    fn test_neon_matches_naive() {
        let adapter = b"AGATCGGAAGAG".to_vec();
        let op = AdapterTrimming::new(adapter, 5, 5);

        let sequences = vec![
            create_test_sequence("seq1", b"ACGTACGTACGTAGATCGGAAGAG"),
            create_test_sequence("seq2", b"GGTTAACCGGTTAACC"),
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
        let adapter = b"AGATCGGAAGAG".to_vec();
        let op = AdapterTrimming::new(adapter, 5, 5);

        let sequences = vec![
            create_test_sequence("seq1", b"ACGTACGTACGTAGATCGGAAGAG"),
            create_test_sequence("seq2", b"GGTTAACCGGTTAACC"),
        ];

        let naive_output = op.execute_naive(&sequences).unwrap();
        let parallel_output = op.execute_parallel(&sequences, 2).unwrap();

        if let (OperationOutput::Records(naive_records), OperationOutput::Records(parallel_records)) =
            (naive_output, parallel_output) {
            assert_eq!(naive_records.len(), parallel_records.len());
            // Results should be identical
            for (n, p) in naive_records.iter().zip(parallel_records.iter()) {
                assert_eq!(n.sequence, p.sequence);
            }
        } else {
            panic!("Expected Records output");
        }
    }
}
