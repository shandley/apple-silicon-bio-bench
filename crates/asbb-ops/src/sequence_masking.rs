//! Sequence masking operation
//!
//! Masks low-quality bases with 'N' based on quality score threshold.
//!
//! **Operation Category**: Element-wise
//! - Highly vectorizable (independent per-base decisions)
//! - Memory-bound (read sequence + quality, write modified sequence)
//! - Common preprocessing step in quality control pipelines
//!
//! **Quality Threshold**: Q < 20 (error rate > 1%)
//! - Phred+33 encoding: Q20 = ASCII 53 ('5')
//! - Bases with quality < 53 are masked as 'N'
//!
//! **Expected Performance** (based on Phase 1 complexity analysis):
//! - Complexity: 0.30 (simple conditional replacement)
//! - NEON: ~20-40× speedup over naive (similar to base_counting at 0.40)
//! - Parallel: ~2-4× additional speedup on 4 cores
//!
//! # Apple Silicon Considerations
//!
//! - **NEON**: Process 16 bases/qualities per instruction
//! - **Vectorized comparison**: vcltq_u8 (compare less than, unsigned 8-bit)
//! - **Conditional masking**: Use comparison result as mask via vbslq_u8
//! - **Memory pattern**: Sequential reads (good cache behavior)

use anyhow::Result;
use asbb_core::{OperationCategory, OperationOutput, PrimitiveOperation, SequenceRecord};
use rayon::prelude::*;
use serde::{Deserialize, Serialize};

/// Sequence masking operation
pub struct SequenceMasking {
    /// Quality threshold (Phred+33 encoding)
    threshold: u8,
}

impl SequenceMasking {
    /// Create new sequence masking operation with default threshold (Q20)
    pub fn new() -> Self {
        Self {
            threshold: 53, // Q20 in Phred+33 encoding (33 + 20)
        }
    }

    /// Create with custom quality threshold
    pub fn with_threshold(threshold: u8) -> Self {
        Self { threshold }
    }

    /// Mask low-quality bases in a single sequence (naive)
    fn mask_sequence_naive(&self, sequence: &[u8], quality: &[u8]) -> Vec<u8> {
        assert_eq!(
            sequence.len(),
            quality.len(),
            "Sequence and quality must have same length"
        );

        sequence
            .iter()
            .zip(quality.iter())
            .map(|(&base, &qual)| {
                if qual < self.threshold {
                    b'N' // Mask low-quality base
                } else {
                    base // Keep high-quality base
                }
            })
            .collect()
    }

    /// Mask low-quality bases in a single sequence (NEON)
    #[cfg(target_arch = "aarch64")]
    fn mask_sequence_neon(&self, sequence: &[u8], quality: &[u8]) -> Vec<u8> {
        use std::arch::aarch64::*;

        assert_eq!(sequence.len(), quality.len());

        let mut result = vec![0u8; sequence.len()];
        let threshold_vec = unsafe { vdupq_n_u8(self.threshold) };
        let n_vec = unsafe { vdupq_n_u8(b'N') };

        let mut i = 0;

        // Process 16 bases at a time with NEON
        while i + 16 <= sequence.len() {
            unsafe {
                // Load 16 bases and 16 quality scores
                let seq_vec = vld1q_u8(sequence.as_ptr().add(i));
                let qual_vec = vld1q_u8(quality.as_ptr().add(i));

                // Compare quality < threshold (returns 0xFF for true, 0x00 for false)
                let mask = vcltq_u8(qual_vec, threshold_vec);

                // Select: if mask[i] then N else sequence[i]
                // vbslq_u8(mask, a, b) = if mask[i] then a[i] else b[i]
                let masked = vbslq_u8(mask, n_vec, seq_vec);

                // Store result
                vst1q_u8(result.as_mut_ptr().add(i), masked);
            }

            i += 16;
        }

        // Process remaining bases (< 16) with scalar code
        for j in i..sequence.len() {
            result[j] = if quality[j] < self.threshold {
                b'N'
            } else {
                sequence[j]
            };
        }

        result
    }

    #[cfg(not(target_arch = "aarch64"))]
    fn mask_sequence_neon(&self, sequence: &[u8], quality: &[u8]) -> Vec<u8> {
        // Fall back to naive on non-ARM
        self.mask_sequence_naive(sequence, quality)
    }
}

impl Default for SequenceMasking {
    fn default() -> Self {
        Self::new()
    }
}

impl PrimitiveOperation for SequenceMasking {
    fn name(&self) -> &str {
        "sequence_masking"
    }

    fn category(&self) -> OperationCategory {
        OperationCategory::ElementWise
    }

    fn execute_naive(&self, data: &[SequenceRecord]) -> Result<OperationOutput> {
        let mut masked_records = Vec::with_capacity(data.len());

        for record in data {
            let masked_sequence = if let Some(quality) = &record.quality {
                self.mask_sequence_naive(&record.sequence, quality)
            } else {
                // No quality scores: return original sequence unchanged
                record.sequence.clone()
            };

            masked_records.push(SequenceRecord {
                id: record.id.clone(),
                sequence: masked_sequence,
                quality: record.quality.clone(), // Keep original quality scores
            });
        }

        Ok(OperationOutput::Records(masked_records))
    }

    fn execute_neon(&self, data: &[SequenceRecord]) -> Result<OperationOutput> {
        let mut masked_records = Vec::with_capacity(data.len());

        for record in data {
            let masked_sequence = if let Some(quality) = &record.quality {
                self.mask_sequence_neon(&record.sequence, quality)
            } else {
                // No quality scores: return original sequence unchanged
                record.sequence.clone()
            };

            masked_records.push(SequenceRecord {
                id: record.id.clone(),
                sequence: masked_sequence,
                quality: record.quality.clone(),
            });
        }

        Ok(OperationOutput::Records(masked_records))
    }

    fn execute_parallel(
        &self,
        data: &[SequenceRecord],
        num_threads: usize,
    ) -> Result<OperationOutput> {
        rayon::ThreadPoolBuilder::new()
            .num_threads(num_threads)
            .build()?
            .install(|| {
                let masked_records: Vec<SequenceRecord> = data
                    .par_iter()
                    .map(|record| {
                        let masked_sequence = if let Some(quality) = &record.quality {
                            self.mask_sequence_naive(&record.sequence, quality)
                        } else {
                            record.sequence.clone()
                        };

                        SequenceRecord {
                            id: record.id.clone(),
                            sequence: masked_sequence,
                            quality: record.quality.clone(),
                        }
                    })
                    .collect();

                Ok(OperationOutput::Records(masked_records))
            })
    }
}

// ============================================================================
// Statistics (for analysis)
// ============================================================================

/// Statistics about masked bases
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MaskingStats {
    /// Total bases processed
    pub total_bases: usize,
    /// Bases masked (replaced with N)
    pub masked_bases: usize,
    /// Percentage of bases masked
    pub masked_percentage: f64,
}

impl MaskingStats {
    /// Calculate masking statistics from before/after sequences
    pub fn from_sequences(original: &[SequenceRecord], masked: &[SequenceRecord]) -> Self {
        let mut total_bases = 0;
        let mut masked_bases = 0;

        for (orig, mask) in original.iter().zip(masked.iter()) {
            total_bases += orig.sequence.len();

            for (&orig_base, &mask_base) in orig.sequence.iter().zip(mask.sequence.iter()) {
                if orig_base != mask_base && mask_base == b'N' {
                    masked_bases += 1;
                }
            }
        }

        let masked_percentage = if total_bases > 0 {
            (masked_bases as f64 / total_bases as f64) * 100.0
        } else {
            0.0
        };

        Self {
            total_bases,
            masked_bases,
            masked_percentage,
        }
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_record(id: &str, sequence: &[u8], quality: &[u8]) -> SequenceRecord {
        SequenceRecord::fastq(id.to_string(), sequence.to_vec(), quality.to_vec())
    }

    #[test]
    fn test_masking_all_high_quality() {
        let op = SequenceMasking::new();
        let data = vec![create_test_record(
            "seq1",
            b"ACGTACGT",
            &[70, 70, 70, 70, 70, 70, 70, 70], // All Q37 (70 - 33)
        )];

        let result = op.execute_naive(&data).unwrap();

        if let OperationOutput::Records(masked) = result {
            assert_eq!(masked.len(), 1);
            assert_eq!(masked[0].sequence, b"ACGTACGT"); // No masking
        } else {
            panic!("Expected Records output");
        }
    }

    #[test]
    fn test_masking_all_low_quality() {
        let op = SequenceMasking::new();
        let data = vec![create_test_record(
            "seq1",
            b"ACGTACGT",
            &[40, 40, 40, 40, 40, 40, 40, 40], // All Q7 (40 - 33), below Q20 threshold
        )];

        let result = op.execute_naive(&data).unwrap();

        if let OperationOutput::Records(masked) = result {
            assert_eq!(masked.len(), 1);
            assert_eq!(masked[0].sequence, b"NNNNNNNN"); // All masked
        } else {
            panic!("Expected Records output");
        }
    }

    #[test]
    fn test_masking_mixed_quality() {
        let op = SequenceMasking::new();
        let data = vec![create_test_record(
            "seq1",
            b"ACGTACGT",
            &[70, 40, 70, 40, 70, 40, 70, 40], // Alternating high/low quality
        )];

        let result = op.execute_naive(&data).unwrap();

        if let OperationOutput::Records(masked) = result {
            assert_eq!(masked.len(), 1);
            // Positions: A(70→A), C(40→N), G(70→G), T(40→N), A(70→A), C(40→N), G(70→G), T(40→N)
            assert_eq!(masked[0].sequence, b"ANGNANGN"); // Keep indices 0,2,4,6; mask indices 1,3,5,7
        } else {
            panic!("Expected Records output");
        }
    }

    #[test]
    fn test_masking_threshold_boundary() {
        let op = SequenceMasking::new(); // Q20 threshold = 53 in Phred+33

        // Test at boundary
        let data = vec![create_test_record(
            "seq1",
            b"ACGT",
            &[52, 53, 54, 55], // Q19, Q20, Q21, Q22
        )];

        let result = op.execute_naive(&data).unwrap();

        if let OperationOutput::Records(masked) = result {
            assert_eq!(masked[0].sequence, b"NCGT"); // Only Q19 masked
        } else {
            panic!("Expected Records output");
        }
    }

    #[test]
    fn test_masking_no_quality_scores() {
        let op = SequenceMasking::new();
        let data = vec![SequenceRecord::fasta("seq1".to_string(), b"ACGT".to_vec())];

        let result = op.execute_naive(&data).unwrap();

        if let OperationOutput::Records(masked) = result {
            assert_eq!(masked[0].sequence, b"ACGT"); // No masking without quality
        } else {
            panic!("Expected Records output");
        }
    }

    #[test]
    fn test_masking_custom_threshold() {
        let op = SequenceMasking::with_threshold(60); // Q27 in Phred+33

        let data = vec![create_test_record(
            "seq1",
            b"ACGT",
            &[55, 58, 60, 65], // Q22, Q25, Q27, Q32
        )];

        let result = op.execute_naive(&data).unwrap();

        if let OperationOutput::Records(masked) = result {
            assert_eq!(masked[0].sequence, b"NNGT"); // Q22, Q25 masked
        } else {
            panic!("Expected Records output");
        }
    }

    #[test]
    #[cfg(target_arch = "aarch64")]
    fn test_neon_matches_naive() {
        let op = SequenceMasking::new();

        // Test with various quality patterns
        let test_cases = vec![
            // All high quality
            (b"ACGTACGTACGTACGTACGT".to_vec(), vec![70u8; 20]),
            // All low quality
            (b"ACGTACGTACGTACGTACGT".to_vec(), vec![40u8; 20]),
            // Mixed quality
            (
                b"ACGTACGTACGTACGTACGT".to_vec(),
                vec![70, 40, 70, 40, 70, 40, 70, 40, 70, 40, 70, 40, 70, 40, 70, 40, 70, 40, 70, 40],
            ),
            // Longer sequence (32 bases, tests remainder handling)
            (
                b"ACGTACGTACGTACGTACGTACGTACGTACGT".to_vec(),
                (0..32).map(|i| if i % 2 == 0 { 70 } else { 40 }).collect(),
            ),
        ];

        for (sequence, quality) in test_cases {
            let data = vec![create_test_record("seq1", &sequence, &quality)];

            let naive_result = op.execute_naive(&data).unwrap();
            let neon_result = op.execute_neon(&data).unwrap();

            match (naive_result, neon_result) {
                (OperationOutput::Records(naive), OperationOutput::Records(neon)) => {
                    assert_eq!(
                        naive[0].sequence, neon[0].sequence,
                        "NEON result must match naive for sequence: {:?}",
                        String::from_utf8_lossy(&sequence)
                    );
                }
                _ => panic!("Expected Records output"),
            }
        }
    }

    #[test]
    fn test_masking_stats() {
        let original = vec![create_test_record(
            "seq1",
            b"ACGTACGT",
            &[70, 40, 70, 40, 70, 40, 70, 40],
        )];

        let op = SequenceMasking::new();
        let result = op.execute_naive(&original).unwrap();

        if let OperationOutput::Records(masked) = result {
            let stats = MaskingStats::from_sequences(&original, &masked);
            assert_eq!(stats.total_bases, 8);
            assert_eq!(stats.masked_bases, 4);
            assert_eq!(stats.masked_percentage, 50.0);
        } else {
            panic!("Expected Records output");
        }
    }

    #[test]
    fn test_parallel_execution() {
        let op = SequenceMasking::new();

        let data: Vec<SequenceRecord> = (0..100)
            .map(|i| {
                create_test_record(
                    &format!("seq{}", i),
                    b"ACGTACGTACGTACGT",
                    &[70, 40, 70, 40, 70, 40, 70, 40, 70, 40, 70, 40, 70, 40, 70, 40],
                )
            })
            .collect();

        let naive_result = op.execute_naive(&data).unwrap();
        let parallel_result = op.execute_parallel(&data, 4).unwrap();

        match (naive_result, parallel_result) {
            (OperationOutput::Records(naive), OperationOutput::Records(parallel)) => {
                assert_eq!(naive.len(), parallel.len());
                for (n, p) in naive.iter().zip(parallel.iter()) {
                    assert_eq!(n.sequence, p.sequence);
                }
            }
            _ => panic!("Expected Records output"),
        }
    }
}
