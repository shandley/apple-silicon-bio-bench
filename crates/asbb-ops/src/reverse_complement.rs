//! Reverse complement operation
//!
//! Reverses DNA sequences and computes complement (A↔T, C↔G).
//!
//! **Operation Category**: Element-wise
//! - Highly vectorizable (independent bases)
//! - More complex than base counting (requires lookup/translation)
//! - Embarrassingly parallel (no shared state)
//!
//! **Expected Results** (based on BioMetal experience):
//! - NEON: 98× speedup (higher than base counting due to complexity)
//! - Parallel (4 cores): Similar threshold pattern (1,000 sequences)
//! - This tests if higher-complexity operations show same patterns
//!
//! # Apple Silicon Considerations
//!
//! Reverse complement is more complex than base counting:
//! - Requires lookup table or bit manipulation for complement
//! - Requires reversing the sequence
//! - NEON can handle both efficiently with table lookups

use anyhow::Result;
use asbb_core::{OperationCategory, OperationOutput, PrimitiveOperation, SequenceRecord};
use rayon::prelude::*;

/// Reverse complement operation
pub struct ReverseComplement;

impl ReverseComplement {
    pub fn new() -> Self {
        Self
    }
}

impl PrimitiveOperation for ReverseComplement {
    fn name(&self) -> &str {
        "reverse_complement"
    }

    fn category(&self) -> OperationCategory {
        OperationCategory::ElementWise
    }

    fn execute_naive(&self, data: &[SequenceRecord]) -> Result<OperationOutput> {
        let mut results = Vec::with_capacity(data.len());

        for record in data {
            let revcomp = naive_reverse_complement(&record.sequence);
            results.push(SequenceRecord::fasta(
                format!("{}_revcomp", record.id),
                revcomp,
            ));
        }

        Ok(OperationOutput::Records(results))
    }

    fn execute_neon(&self, data: &[SequenceRecord]) -> Result<OperationOutput> {
        #[cfg(target_arch = "aarch64")]
        {
            let mut results = Vec::with_capacity(data.len());

            for record in data {
                let revcomp = neon_reverse_complement(&record.sequence);
                results.push(SequenceRecord::fasta(
                    format!("{}_revcomp", record.id),
                    revcomp,
                ));
            }

            Ok(OperationOutput::Records(results))
        }

        #[cfg(not(target_arch = "aarch64"))]
        {
            // Fall back to naive on non-ARM
            self.execute_naive(data)
        }
    }

    fn execute_parallel(
        &self,
        data: &[SequenceRecord],
        num_threads: usize,
    ) -> Result<OperationOutput> {
        // Configure Rayon thread pool
        let pool = rayon::ThreadPoolBuilder::new()
            .num_threads(num_threads)
            .build()?;

        let results = pool.install(|| {
            data.par_iter()
                .map(|record| {
                    #[cfg(target_arch = "aarch64")]
                    {
                        // Use NEON per-thread for true combined optimization
                        let revcomp = neon_reverse_complement(&record.sequence);
                        SequenceRecord::fasta(
                            format!("{}_revcomp", record.id),
                            revcomp,
                        )
                    }

                    #[cfg(not(target_arch = "aarch64"))]
                    {
                        // Fall back to naive on non-ARM
                        let revcomp = naive_reverse_complement(&record.sequence);
                        SequenceRecord::fasta(
                            format!("{}_revcomp", record.id),
                            revcomp,
                        )
                    }
                })
                .collect()
        });

        Ok(OperationOutput::Records(results))
    }
}

impl Default for ReverseComplement {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Naive Implementation
// ============================================================================

/// Complement lookup table
const COMPLEMENT_TABLE: [u8; 256] = {
    let mut table = [b'N'; 256];
    table[b'A' as usize] = b'T';
    table[b'T' as usize] = b'A';
    table[b'C' as usize] = b'G';
    table[b'G' as usize] = b'C';
    table[b'a' as usize] = b't';
    table[b't' as usize] = b'a';
    table[b'c' as usize] = b'g';
    table[b'g' as usize] = b'c';
    table[b'N' as usize] = b'N';
    table[b'n' as usize] = b'n';
    table
};

fn naive_reverse_complement(seq: &[u8]) -> Vec<u8> {
    seq.iter()
        .rev()
        .map(|&base| COMPLEMENT_TABLE[base as usize])
        .collect()
}

// ============================================================================
// NEON SIMD Implementation
// ============================================================================

#[cfg(target_arch = "aarch64")]
fn neon_reverse_complement(seq: &[u8]) -> Vec<u8> {
    use std::arch::aarch64::*;

    let mut result = vec![0u8; seq.len()];

    // Process in chunks of 16 bytes with NEON
    let chunks = seq.chunks_exact(16);
    let remainder = chunks.remainder();
    let num_chunks = chunks.len();

    unsafe {
        // Create lookup tables for complement using NEON
        // We'll use a clever trick: XOR with specific masks
        // A (0x41) ↔ T (0x54): XOR with 0x15
        // C (0x43) ↔ G (0x47): XOR with 0x04
        // But this is tricky with upper/lower case, so we'll use table lookups

        // For each 16-byte chunk, we:
        // 1. Load the chunk
        // 2. Apply complement via table lookup
        // 3. Store in reversed position

        for (i, chunk) in chunks.enumerate() {
            // Load 16 bytes
            let data = vld1q_u8(chunk.as_ptr());

            // Complement using table lookup (two passes for 16 entries)
            // NEON table lookup operates on 4-bit indices (16 entries)
            // We'll use a simpler approach: conditional moves

            // Create masks for each base
            let mask_a_upper = vceqq_u8(data, vdupq_n_u8(b'A'));
            let mask_t_upper = vceqq_u8(data, vdupq_n_u8(b'T'));
            let mask_c_upper = vceqq_u8(data, vdupq_n_u8(b'C'));
            let mask_g_upper = vceqq_u8(data, vdupq_n_u8(b'G'));
            let mask_a_lower = vceqq_u8(data, vdupq_n_u8(b'a'));
            let mask_t_lower = vceqq_u8(data, vdupq_n_u8(b't'));
            let mask_c_lower = vceqq_u8(data, vdupq_n_u8(b'c'));
            let mask_g_lower = vceqq_u8(data, vdupq_n_u8(b'g'));

            // Create complement values
            let val_t_upper = vdupq_n_u8(b'T');
            let val_a_upper = vdupq_n_u8(b'A');
            let val_g_upper = vdupq_n_u8(b'G');
            let val_c_upper = vdupq_n_u8(b'C');
            let val_t_lower = vdupq_n_u8(b't');
            let val_a_lower = vdupq_n_u8(b'a');
            let val_g_lower = vdupq_n_u8(b'g');
            let val_c_lower = vdupq_n_u8(b'c');

            // Apply complement using bitwise select (mask ? complement : original)
            let mut complemented = data;
            complemented = vbslq_u8(mask_a_upper, val_t_upper, complemented);
            complemented = vbslq_u8(mask_t_upper, val_a_upper, complemented);
            complemented = vbslq_u8(mask_c_upper, val_g_upper, complemented);
            complemented = vbslq_u8(mask_g_upper, val_c_upper, complemented);
            complemented = vbslq_u8(mask_a_lower, val_t_lower, complemented);
            complemented = vbslq_u8(mask_t_lower, val_a_lower, complemented);
            complemented = vbslq_u8(mask_c_lower, val_g_lower, complemented);
            complemented = vbslq_u8(mask_g_lower, val_c_lower, complemented);

            // Store in reversed position
            let rev_pos = seq.len() - (i + 1) * 16;

            // Reverse the 16 bytes within the chunk
            // Extract bytes and reverse
            let mut temp = [0u8; 16];
            vst1q_u8(temp.as_mut_ptr(), complemented);

            for j in 0..16 {
                result[rev_pos + j] = temp[15 - j];
            }
        }
    }

    // Process remainder with scalar code
    let remainder_start = num_chunks * 16;
    for (i, &base) in remainder.iter().enumerate() {
        let rev_pos = remainder.len() - 1 - i;
        result[rev_pos] = COMPLEMENT_TABLE[base as usize];
    }

    result
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_data() -> Vec<SequenceRecord> {
        vec![
            SequenceRecord::fasta("seq1".to_string(), b"ACGT".to_vec()),
            SequenceRecord::fasta("seq2".to_string(), b"AAAA".to_vec()),
            SequenceRecord::fasta("seq3".to_string(), b"TTTT".to_vec()),
            SequenceRecord::fasta("seq4".to_string(), b"ACGTACGT".to_vec()),
        ]
    }

    #[test]
    fn test_reverse_complement_naive() {
        let op = ReverseComplement::new();
        let data = create_test_data();

        let result = op.execute_naive(&data).unwrap();

        if let OperationOutput::Records(sequences) = result {
            assert_eq!(sequences.len(), 4);

            // ACGT → ACGT (reverse) → TGCA (complement)
            assert_eq!(sequences[0].sequence, b"ACGT");

            // AAAA → AAAA (reverse) → TTTT (complement)
            assert_eq!(sequences[1].sequence, b"TTTT");

            // TTTT → TTTT (reverse) → AAAA (complement)
            assert_eq!(sequences[2].sequence, b"AAAA");

            // ACGTACGT → TGCATGCA (reverse complement)
            assert_eq!(sequences[3].sequence, b"ACGTACGT");
        } else {
            panic!("Expected Records output");
        }
    }

    #[test]
    fn test_naive_reverse_complement() {
        assert_eq!(naive_reverse_complement(b"ACGT"), b"ACGT");
        assert_eq!(naive_reverse_complement(b"AAAA"), b"TTTT");
        assert_eq!(naive_reverse_complement(b"TTTT"), b"AAAA");
        assert_eq!(naive_reverse_complement(b"CCCC"), b"GGGG");
        assert_eq!(naive_reverse_complement(b"GGGG"), b"CCCC");
        assert_eq!(naive_reverse_complement(b"ACGTACGT"), b"ACGTACGT");
        assert_eq!(naive_reverse_complement(b"ATCG"), b"CGAT");
    }

    #[test]
    #[cfg(target_arch = "aarch64")]
    fn test_reverse_complement_neon() {
        let op = ReverseComplement::new();
        let data = create_test_data();

        let result_naive = op.execute_naive(&data).unwrap();
        let result_neon = op.execute_neon(&data).unwrap();

        // NEON should produce identical results to naive
        assert_eq!(result_naive, result_neon);
    }

    #[test]
    fn test_reverse_complement_parallel() {
        let op = ReverseComplement::new();
        let data = create_test_data();

        let result_naive = op.execute_naive(&data).unwrap();
        let result_parallel = op.execute_parallel(&data, 4).unwrap();

        // Parallel should produce identical results to naive
        assert_eq!(result_naive, result_parallel);
    }

    #[test]
    fn test_reverse_complement_case_insensitive() {
        let op = ReverseComplement::new();
        let data = vec![
            SequenceRecord::fasta("test".to_string(), b"AcGt".to_vec()),
        ];

        let result = op.execute_naive(&data).unwrap();

        if let OperationOutput::Records(sequences) = result {
            assert_eq!(sequences[0].sequence, b"aCgT");
        } else {
            panic!("Expected Records output");
        }
    }

    #[test]
    fn test_reverse_complement_palindrome() {
        // ACGT is its own reverse complement
        assert_eq!(naive_reverse_complement(b"ACGT"), b"ACGT");

        // GCGC is also palindromic
        assert_eq!(naive_reverse_complement(b"GCGC"), b"GCGC");
    }

    #[test]
    fn test_reverse_complement_long_sequence() {
        // Test with a longer sequence (>16 bytes to test NEON chunking)
        let seq = b"ACGTACGTACGTACGTACGTACGT";  // 24 bases
        let expected = b"ACGTACGTACGTACGTACGTACGT";  // Palindrome

        assert_eq!(naive_reverse_complement(seq), expected);

        #[cfg(target_arch = "aarch64")]
        {
            assert_eq!(neon_reverse_complement(seq), expected);
        }
    }
}
