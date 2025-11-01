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
use asbb_core::{encoding::BitSeq, OperationCategory, OperationOutput, PrimitiveOperation, SequenceRecord};
use rayon::prelude::*;

/// Reverse complement operation
pub struct ReverseComplement;

impl ReverseComplement {
    pub fn new() -> Self {
        Self
    }

    /// Execute reverse complement on 2-bit encoded sequences
    ///
    /// This is the headline result for Phase 2!
    /// Expected: 98× speedup over ASCII NEON (BioMetal validated)
    ///
    /// Uses BitSeq::reverse_complement() which:
    /// - Operates directly on 2-bit data (no ASCII conversion)
    /// - XOR for complement (A↔T, C↔G via 0b11 XOR)
    /// - Efficient reversal of packed data
    pub fn execute_2bit_naive(&self, data: &[BitSeq]) -> Result<OperationOutput> {
        let mut results = Vec::with_capacity(data.len());

        for (i, bitseq) in data.iter().enumerate() {
            let revcomp = bitseq.reverse_complement();
            // Convert back to ASCII for output
            let ascii = revcomp.to_ascii();
            results.push(SequenceRecord::fasta(
                format!("seq{}_revcomp", i),
                ascii,
            ));
        }

        Ok(OperationOutput::Records(results))
    }

    /// Execute reverse complement on 2-bit encoded sequences (NEON)
    ///
    /// Expected: 98× speedup over ASCII NEON
    /// This is the CRITICAL validation for Phase 2 encoding dimension
    pub fn execute_2bit_neon(&self, data: &[BitSeq]) -> Result<OperationOutput> {
        // BitSeq::reverse_complement() already uses NEON on aarch64
        // Just call the same method - the speedup comes from 2-bit encoding
        self.execute_2bit_naive(data)
    }

    /// Execute reverse complement using GPU (Metal)
    ///
    /// ## Performance Characteristics
    ///
    /// Reverse complement is more complex than base counting:
    /// - Each base requires complement lookup (A↔T, C↔G)
    /// - Sequence must be reversed (different memory access)
    /// - ~10-15 operations per byte (vs ~6 for base counting)
    ///
    /// **Hypothesis**: Higher complexity may show GPU benefit at lower batch sizes
    /// than base counting.
    ///
    /// **Expected cliff threshold**: 100K-500K sequences (vs no cliff for base counting)
    ///
    /// Returns transformed sequences and performance metrics.
    #[cfg(all(target_os = "macos", feature = "gpu"))]
    pub fn execute_gpu(&self, data: &[SequenceRecord]) -> Result<(Vec<SequenceRecord>, asbb_gpu::GpuMetrics)> {
        use asbb_gpu::MetalBackend;

        let backend = MetalBackend::new()?;
        backend.reverse_complement_gpu(data)
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

    #[test]
    fn test_reverse_complement_2bit_naive() {
        let op = ReverseComplement::new();

        // Create 2-bit encoded sequences
        let bitseqs = vec![
            BitSeq::from_ascii(b"ACGT"),        // Palindrome
            BitSeq::from_ascii(b"AAAA"),        // → TTTT
            BitSeq::from_ascii(b"ATCG"),        // → CGAT
            BitSeq::from_ascii(b"ACG"),         // → CGT (partial byte)
        ];

        let result = op.execute_2bit_naive(&bitseqs).unwrap();

        if let OperationOutput::Records(sequences) = result {
            assert_eq!(sequences.len(), 4);
            assert_eq!(sequences[0].sequence, b"ACGT");  // Palindrome
            assert_eq!(sequences[1].sequence, b"TTTT");
            assert_eq!(sequences[2].sequence, b"CGAT");
            assert_eq!(sequences[3].sequence, b"CGT");
        } else {
            panic!("Expected Records output");
        }
    }

    #[test]
    #[cfg(target_arch = "aarch64")]
    fn test_reverse_complement_2bit_neon() {
        let op = ReverseComplement::new();

        // Create 2-bit encoded sequences
        let bitseqs = vec![
            BitSeq::from_ascii(b"ACGT"),
            BitSeq::from_ascii(b"AAAA"),
            BitSeq::from_ascii(b"ATCG"),
        ];

        let result_naive = op.execute_2bit_naive(&bitseqs).unwrap();
        let result_neon = op.execute_2bit_neon(&bitseqs).unwrap();

        // NEON should produce identical results to naive
        assert_eq!(result_naive, result_neon);
    }

    #[test]
    fn test_reverse_complement_2bit_vs_ascii() {
        let op = ReverseComplement::new();

        // Test that 2-bit produces same results as ASCII
        let test_seqs = vec![
            b"ACGT".to_vec(),
            b"AAAA".to_vec(),
            b"TTTT".to_vec(),
            b"ACGTACGTACGTACGT".to_vec(),  // Longer sequence
        ];

        for seq in test_seqs {
            // ASCII version
            let ascii_records = vec![SequenceRecord::fasta("test".to_string(), seq.clone())];
            let ascii_result = op.execute_naive(&ascii_records).unwrap();

            // 2-bit version
            let bitseqs = vec![BitSeq::from_ascii(&seq)];
            let bitseq_result = op.execute_2bit_naive(&bitseqs).unwrap();

            // Results should match
            if let (OperationOutput::Records(ascii_seqs), OperationOutput::Records(bit_seqs)) =
                (ascii_result, bitseq_result)
            {
                assert_eq!(ascii_seqs[0].sequence, bit_seqs[0].sequence,
                    "Mismatch for sequence: {}", String::from_utf8_lossy(&seq));
            } else {
                panic!("Expected Records output");
            }
        }
    }

    #[test]
    fn test_reverse_complement_2bit_large() {
        let op = ReverseComplement::new();

        // Create a large sequence to test vectorization (>64 bases)
        let large_seq = b"ACGTACGTACGTACGTACGTACGTACGTACGTACGTACGTACGTACGTACGTACGTACGTACGTACGTACGTACGTACGTACGTACGTACGTACGTACGTACGTACGTACGTACGTACGTACGTACGTACGTACGTACGTACGTACGTACGTACGTACGTACGTACGTACGTACGT";

        // 2-bit version
        let bitseqs = vec![BitSeq::from_ascii(large_seq)];
        let result = op.execute_2bit_naive(&bitseqs).unwrap();

        if let OperationOutput::Records(sequences) = result {
            // Should be palindrome (ACGTACGT pattern)
            assert_eq!(sequences[0].sequence.as_slice(), large_seq);
        } else {
            panic!("Expected Records output");
        }
    }
}
