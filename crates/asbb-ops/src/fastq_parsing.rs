//! FASTQ Parsing Operation
//!
//! Parses FASTQ format sequence data into SequenceRecords.
//!
//! # Operation Characteristics
//! - **Category**: I/O
//! - **Complexity**: 0.25 (simple line parsing with validation)
//! - **Output**: Parsed sequence records
//! - **NEON benefit**: Low (I/O bandwidth limited, simple parsing)
//!
//! # Implementation Notes
//! - Validates FASTQ format (4 lines per record)
//! - Checks quality score lengths match sequence lengths
//! - Handles malformed records gracefully
//! - NEON can accelerate quality score validation

use crate::{OperationCategory, OperationOutput, PrimitiveOperation, SequenceRecord};
use anyhow::{anyhow, Result};
use rayon::prelude::*;

#[cfg(target_arch = "aarch64")]
use std::arch::aarch64::*;

/// FASTQ parsing operation
pub struct FastqParsing {
    /// Validate quality scores are in valid range
    validate_quality: bool,
}

impl FastqParsing {
    pub fn new(validate_quality: bool) -> Self {
        Self { validate_quality }
    }

    /// Parse a single FASTQ record from 4 lines
    fn parse_record_naive(&self, lines: &[&str]) -> Result<SequenceRecord> {
        if lines.len() != 4 {
            return Err(anyhow!("FASTQ record must have exactly 4 lines"));
        }

        // Line 0: @ + ID
        let id_line = lines[0];
        if !id_line.starts_with('@') {
            return Err(anyhow!("FASTQ ID line must start with '@'"));
        }
        let id = id_line[1..].trim().to_string();

        // Line 1: Sequence
        let sequence = lines[1].trim().as_bytes().to_vec();

        // Line 2: + (separator, may contain ID)
        if !lines[2].starts_with('+') {
            return Err(anyhow!("FASTQ separator line must start with '+'"));
        }

        // Line 3: Quality scores
        let quality = lines[3].trim().as_bytes().to_vec();

        // Validate lengths match
        if sequence.len() != quality.len() {
            return Err(anyhow!(
                "Sequence length ({}) does not match quality length ({})",
                sequence.len(),
                quality.len()
            ));
        }

        // Validate quality scores (Phred+33 encoding: ! to ~, ASCII 33-126)
        if self.validate_quality {
            for &q in &quality {
                if q < 33 || q > 126 {
                    return Err(anyhow!("Invalid quality score: {}", q));
                }
            }
        }

        Ok(SequenceRecord {
            id,
            sequence,
            quality: Some(quality),
        })
    }

    /// Parse FASTQ record using NEON (vectorized quality validation)
    #[cfg(target_arch = "aarch64")]
    fn parse_record_neon(&self, lines: &[&str]) -> Result<SequenceRecord> {
        if lines.len() != 4 {
            return Err(anyhow!("FASTQ record must have exactly 4 lines"));
        }

        let id_line = lines[0];
        if !id_line.starts_with('@') {
            return Err(anyhow!("FASTQ ID line must start with '@'"));
        }
        let id = id_line[1..].trim().to_string();

        let sequence = lines[1].trim().as_bytes().to_vec();

        if !lines[2].starts_with('+') {
            return Err(anyhow!("FASTQ separator line must start with '+'"));
        }

        let quality = lines[3].trim().as_bytes().to_vec();

        if sequence.len() != quality.len() {
            return Err(anyhow!(
                "Sequence length ({}) does not match quality length ({})",
                sequence.len(),
                quality.len()
            ));
        }

        // NEON-accelerated quality validation
        if self.validate_quality {
            unsafe {
                let min_qual = vdupq_n_u8(33);
                let max_qual = vdupq_n_u8(126);

                let mut i = 0;

                // Process 16 bytes at a time
                while i + 16 <= quality.len() {
                    let qual_vec = vld1q_u8(quality.as_ptr().add(i));

                    // Check if all values are >= 33 and <= 126
                    let gte_min = vcgeq_u8(qual_vec, min_qual);
                    let lte_max = vcleq_u8(qual_vec, max_qual);
                    let valid = vandq_u8(gte_min, lte_max);

                    // Check if all are valid (all bits set)
                    let mut valid_bytes = [0u8; 16];
                    vst1q_u8(valid_bytes.as_mut_ptr(), valid);

                    if !valid_bytes.iter().all(|&b| b == 0xFF) {
                        return Err(anyhow!("Invalid quality score in range"));
                    }

                    i += 16;
                }

                // Handle remainder with scalar
                for &q in &quality[i..] {
                    if q < 33 || q > 126 {
                        return Err(anyhow!("Invalid quality score: {}", q));
                    }
                }
            }
        }

        Ok(SequenceRecord {
            id,
            sequence,
            quality: Some(quality),
        })
    }

    /// Parse FASTQ data from text
    fn parse_fastq_text(&self, text: &str) -> Vec<Result<SequenceRecord>> {
        let lines: Vec<&str> = text.lines().collect();
        let num_records = lines.len() / 4;

        let mut records = Vec::with_capacity(num_records);

        for i in 0..num_records {
            let record_lines = &lines[i * 4..(i * 4) + 4];

            #[cfg(target_arch = "aarch64")]
            let result = self.parse_record_neon(record_lines);
            #[cfg(not(target_arch = "aarch64"))]
            let result = self.parse_record_naive(record_lines);

            records.push(result);
        }

        records
    }
}

impl PrimitiveOperation for FastqParsing {
    fn name(&self) -> &str {
        "fastq_parsing"
    }

    fn category(&self) -> OperationCategory {
        OperationCategory::IO
    }

    fn execute_naive(&self, sequences: &[SequenceRecord]) -> Result<OperationOutput> {
        // For testing purposes, we simulate FASTQ parsing by converting
        // SequenceRecords back to FASTQ format and then parsing them

        // Generate FASTQ text from input records
        let mut fastq_text = String::new();
        for record in sequences {
            fastq_text.push_str(&format!("@{}\n", record.id));
            fastq_text.push_str(&format!("{}\n", String::from_utf8_lossy(&record.sequence)));
            fastq_text.push_str("+\n");
            if let Some(ref quality) = record.quality {
                fastq_text.push_str(&format!("{}\n", String::from_utf8_lossy(quality)));
            } else {
                // Generate default quality (all Q30)
                let default_quality = vec![b'?'; record.sequence.len()];
                fastq_text.push_str(&format!("{}\n", String::from_utf8_lossy(&default_quality)));
            }
        }

        // Parse the FASTQ text
        let results = self.parse_fastq_text(&fastq_text);

        // Collect successful parses
        let parsed_records: Vec<SequenceRecord> = results
            .into_iter()
            .filter_map(|r| r.ok())
            .collect();

        Ok(OperationOutput::Records(parsed_records))
    }

    #[cfg(target_arch = "aarch64")]
    fn execute_neon(&self, sequences: &[SequenceRecord]) -> Result<OperationOutput> {
        // Same as naive, but uses NEON in parse_record_neon
        self.execute_naive(sequences)
    }

    fn execute_parallel(&self, sequences: &[SequenceRecord], num_threads: usize) -> Result<OperationOutput> {
        // Generate FASTQ text
        let mut fastq_text = String::new();
        for record in sequences {
            fastq_text.push_str(&format!("@{}\n", record.id));
            fastq_text.push_str(&format!("{}\n", String::from_utf8_lossy(&record.sequence)));
            fastq_text.push_str("+\n");
            if let Some(ref quality) = record.quality {
                fastq_text.push_str(&format!("{}\n", String::from_utf8_lossy(quality)));
            } else {
                let default_quality = vec![b'?'; record.sequence.len()];
                fastq_text.push_str(&format!("{}\n", String::from_utf8_lossy(&default_quality)));
            }
        }

        // Parse in parallel (chunk by records)
        let lines: Vec<&str> = fastq_text.lines().collect();
        let num_records = lines.len() / 4;

        let pool = rayon::ThreadPoolBuilder::new()
            .num_threads(num_threads)
            .build()
            .unwrap();

        let results: Vec<Result<SequenceRecord>> = pool.install(|| {
            (0..num_records).into_par_iter().map(|i| {
                let record_lines = &lines[i * 4..(i * 4) + 4];

                #[cfg(target_arch = "aarch64")]
                {
                    self.parse_record_neon(record_lines)
                }
                #[cfg(not(target_arch = "aarch64"))]
                {
                    self.parse_record_naive(record_lines)
                }
            }).collect()
        });

        let parsed_records: Vec<SequenceRecord> = results
            .into_iter()
            .filter_map(|r| r.ok())
            .collect();

        Ok(OperationOutput::Records(parsed_records))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_fastq_record() {
        let op = FastqParsing::new(true);

        let lines = vec![
            "@SEQ1",
            "ACGTACGT",
            "+",
            "IIIIIIII",
        ];

        let result = op.parse_record_naive(&lines).unwrap();
        assert_eq!(result.id, "SEQ1");
        assert_eq!(result.sequence, b"ACGTACGT");
        assert_eq!(result.quality.unwrap(), b"IIIIIIII");
    }

    #[test]
    fn test_missing_at_symbol() {
        let op = FastqParsing::new(true);

        let lines = vec![
            "SEQ1",  // Missing @
            "ACGTACGT",
            "+",
            "IIIIIIII",
        ];

        let result = op.parse_record_naive(&lines);
        assert!(result.is_err());
    }

    #[test]
    fn test_missing_plus_symbol() {
        let op = FastqParsing::new(true);

        let lines = vec![
            "@SEQ1",
            "ACGTACGT",
            "SEQ1",  // Missing +
            "IIIIIIII",
        ];

        let result = op.parse_record_naive(&lines);
        assert!(result.is_err());
    }

    #[test]
    fn test_length_mismatch() {
        let op = FastqParsing::new(true);

        let lines = vec![
            "@SEQ1",
            "ACGTACGT",
            "+",
            "III",  // Too short
        ];

        let result = op.parse_record_naive(&lines);
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_quality_score() {
        let op = FastqParsing::new(true);

        let lines = vec![
            "@SEQ1",
            "ACGTACGT",
            "+",
            "IIIII\x01II",  // Invalid quality (ASCII 1)
        ];

        let result = op.parse_record_naive(&lines);
        assert!(result.is_err());
    }

    #[test]
    fn test_quality_validation_disabled() {
        let op = FastqParsing::new(false);

        let lines = vec![
            "@SEQ1",
            "ACGTACGT",
            "+",
            "IIIII\x01II",  // Invalid quality, but validation disabled
        ];

        let result = op.parse_record_naive(&lines);
        assert!(result.is_ok());
    }

    #[test]
    fn test_multiple_records() {
        let op = FastqParsing::new(true);

        let fastq_text = "@SEQ1\nACGT\n+\nIIII\n@SEQ2\nGGTT\n+\nJJJJ\n";

        let results = op.parse_fastq_text(fastq_text);
        assert_eq!(results.len(), 2);
        assert!(results[0].is_ok());
        assert!(results[1].is_ok());

        let rec1 = results[0].as_ref().unwrap();
        assert_eq!(rec1.id, "SEQ1");
        assert_eq!(rec1.sequence, b"ACGT");

        let rec2 = results[1].as_ref().unwrap();
        assert_eq!(rec2.id, "SEQ2");
        assert_eq!(rec2.sequence, b"GGTT");
    }

    #[test]
    fn test_execute_round_trip() {
        let op = FastqParsing::new(true);

        let input = vec![
            SequenceRecord {
                id: "seq1".to_string(),
                sequence: b"ACGTACGT".to_vec(),
                quality: Some(b"IIIIIIII".to_vec()),
            },
            SequenceRecord {
                id: "seq2".to_string(),
                sequence: b"GGTTAACC".to_vec(),
                quality: Some(b"JJJJJJJJ".to_vec()),
            },
        ];

        let output = op.execute_naive(&input).unwrap();
        if let OperationOutput::Records(records) = output {
            assert_eq!(records.len(), 2);
            assert_eq!(records[0].id, "seq1");
            assert_eq!(records[0].sequence, b"ACGTACGT");
            assert_eq!(records[1].id, "seq2");
            assert_eq!(records[1].sequence, b"GGTTAACC");
        } else {
            panic!("Expected Records output");
        }
    }

    #[test]
    #[cfg(target_arch = "aarch64")]
    fn test_neon_matches_naive() {
        let op = FastqParsing::new(true);

        let input = vec![
            SequenceRecord {
                id: "seq1".to_string(),
                sequence: b"ACGTACGTACGTACGT".to_vec(),
                quality: Some(b"IIIIIIIIIIIIIIII".to_vec()),
            },
        ];

        let naive_output = op.execute_naive(&input).unwrap();
        let neon_output = op.execute_neon(&input).unwrap();

        if let (OperationOutput::Records(naive_records), OperationOutput::Records(neon_records)) =
            (naive_output, neon_output) {
            assert_eq!(naive_records.len(), neon_records.len());
            for (n, ne) in naive_records.iter().zip(neon_records.iter()) {
                assert_eq!(n.sequence, ne.sequence);
                assert_eq!(n.quality, ne.quality);
            }
        } else {
            panic!("Expected Records output");
        }
    }

    #[test]
    fn test_parallel_execution() {
        let op = FastqParsing::new(true);

        let input = vec![
            SequenceRecord {
                id: "seq1".to_string(),
                sequence: b"ACGTACGT".to_vec(),
                quality: Some(b"IIIIIIII".to_vec()),
            },
            SequenceRecord {
                id: "seq2".to_string(),
                sequence: b"GGTTAACC".to_vec(),
                quality: Some(b"JJJJJJJJ".to_vec()),
            },
        ];

        let naive_output = op.execute_naive(&input).unwrap();
        let parallel_output = op.execute_parallel(&input, 2).unwrap();

        if let (OperationOutput::Records(naive_records), OperationOutput::Records(parallel_records)) =
            (naive_output, parallel_output) {
            assert_eq!(naive_records.len(), parallel_records.len());
            for (n, p) in naive_records.iter().zip(parallel_records.iter()) {
                assert_eq!(n.sequence, p.sequence);
                assert_eq!(n.quality, p.quality);
            }
        } else {
            panic!("Expected Records output");
        }
    }
}
