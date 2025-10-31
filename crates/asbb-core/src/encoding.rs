// 2-bit DNA encoding for efficient sequence representation
//
// Encoding scheme: A=00, C=01, G=10, T=11 (2 bits per base)
// Storage: 4 bases per byte, packed from MSB to LSB
//
// Example: "ACGT" → 0b00011011 → 0x1B
//          A=00, C=01, G=10, T=11
//
// Benefits:
// - 4× memory density (4 bases/byte vs 1 base/byte ASCII)
// - SIMD-friendly bitwise operations
// - Cache-friendly compact representation
//
// Based on BioMetal's BitSeq implementation which achieved:
// - Reverse complement: 98× speedup vs ASCII
// - Base counting: 1.3× speedup (modest, cache benefit)

use serde::{Deserialize, Serialize};
use std::fmt;

/// 2-bit encoded DNA sequence (4 bases per byte)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BitSeq {
    /// Packed 2-bit representation (4 bases per byte)
    data: Vec<u8>,

    /// Number of bases (not bytes!)
    /// Length may not be multiple of 4 (last byte padded with zeros)
    length: usize,
}

impl BitSeq {
    /// Create a new BitSeq from 2-bit encoded data
    ///
    /// # Safety
    /// Data must be valid 2-bit encoding (each 2-bit pair in 00-11 range)
    pub fn new(data: Vec<u8>, length: usize) -> Self {
        assert!(length <= data.len() * 4, "Length exceeds data capacity");
        Self { data, length }
    }

    /// Get the length in bases
    pub fn len(&self) -> usize {
        self.length
    }

    /// Check if sequence is empty
    pub fn is_empty(&self) -> bool {
        self.length == 0
    }

    /// Get underlying 2-bit data
    pub fn data(&self) -> &[u8] {
        &self.data
    }

    /// Encode ASCII DNA sequence to 2-bit
    ///
    /// Encoding: A/a=00, C/c=01, G/g=10, T/t=11
    /// Invalid bases (N, etc.) encoded as 00 (A)
    pub fn from_ascii(seq: &[u8]) -> Self {
        let num_bytes = (seq.len() + 3) / 4; // Round up to next byte
        let mut data = vec![0u8; num_bytes];

        for (i, &base) in seq.iter().enumerate() {
            let encoded = encode_base(base);
            let byte_idx = i / 4;
            let bit_offset = 6 - (i % 4) * 2; // MSB first: 6, 4, 2, 0
            data[byte_idx] |= encoded << bit_offset;
        }

        Self {
            data,
            length: seq.len(),
        }
    }

    /// Decode 2-bit to ASCII DNA sequence
    pub fn to_ascii(&self) -> Vec<u8> {
        let mut result = Vec::with_capacity(self.length);

        for i in 0..self.length {
            let byte_idx = i / 4;
            let bit_offset = 6 - (i % 4) * 2; // MSB first
            let encoded = (self.data[byte_idx] >> bit_offset) & 0b11;
            result.push(decode_base(encoded));
        }

        result
    }

    /// Get base at position (returns ASCII character)
    pub fn get(&self, index: usize) -> Option<u8> {
        if index >= self.length {
            return None;
        }

        let byte_idx = index / 4;
        let bit_offset = 6 - (index % 4) * 2;
        let encoded = (self.data[byte_idx] >> bit_offset) & 0b11;
        Some(decode_base(encoded))
    }

    /// Reverse complement using 2-bit operations
    ///
    /// TODO: Optimize with pure 2-bit NEON operations (expected 98× speedup)
    /// For now, using ASCII roundtrip for correctness
    pub fn reverse_complement(&self) -> Self {
        // Decode to ASCII
        let mut ascii = self.to_ascii();

        // Complement each base
        for base in ascii.iter_mut() {
            *base = match *base {
                b'A' => b'T',
                b'C' => b'G',
                b'G' => b'C',
                b'T' => b'A',
                _ => *base, // Shouldn't happen with valid 2-bit data
            };
        }

        // Reverse
        ascii.reverse();

        // Re-encode
        Self::from_ascii(&ascii)
    }

    /// Complement only (no reversal)
    pub fn complement(&self) -> Self {
        let mut result_data = self.data.clone();

        // Complement: XOR with 0xFF (0b11111111)
        // A=00→T=11, C=01→G=10, G=10→C=01, T=11→A=00
        for byte in result_data.iter_mut() {
            *byte ^= 0xFF;
        }

        Self {
            data: result_data,
            length: self.length,
        }
    }

    /// Count occurrences of a specific base (2-bit comparison)
    ///
    /// Base encoding: A=00, C=01, G=10, T=11
    pub fn count_base(&self, base: u8) -> usize {
        let target = encode_base(base);
        let mut count = 0;

        for i in 0..self.length {
            let byte_idx = i / 4;
            let bit_offset = 6 - (i % 4) * 2;
            let encoded = (self.data[byte_idx] >> bit_offset) & 0b11;
            if encoded == target {
                count += 1;
            }
        }

        count
    }

    /// Count GC bases (G or C)
    pub fn count_gc(&self) -> usize {
        let mut count = 0;

        for i in 0..self.length {
            let byte_idx = i / 4;
            let bit_offset = 6 - (i % 4) * 2;
            let encoded = (self.data[byte_idx] >> bit_offset) & 0b11;
            // C=01, G=10 both have at least one bit set
            // A=00, T=11 can be distinguished by checking individual bits
            // GC detection: encoded == 01 or encoded == 10
            if encoded == 0b01 || encoded == 0b10 {
                count += 1;
            }
        }

        count
    }

    /// Count AT bases (A or T)
    pub fn count_at(&self) -> usize {
        let mut count = 0;

        for i in 0..self.length {
            let byte_idx = i / 4;
            let bit_offset = 6 - (i % 4) * 2;
            let encoded = (self.data[byte_idx] >> bit_offset) & 0b11;
            // A=00, T=11
            if encoded == 0b00 || encoded == 0b11 {
                count += 1;
            }
        }

        count
    }
}

/// Encode ASCII base to 2-bit representation
///
/// A/a → 00
/// C/c → 01
/// G/g → 10
/// T/t → 11
/// Other (N, etc.) → 00 (default to A)
#[inline]
fn encode_base(base: u8) -> u8 {
    match base {
        b'A' | b'a' => 0b00,
        b'C' | b'c' => 0b01,
        b'G' | b'g' => 0b10,
        b'T' | b't' => 0b11,
        _ => 0b00, // Default invalid bases to A
    }
}

/// Decode 2-bit representation to ASCII base
///
/// 00 → A
/// 01 → C
/// 10 → G
/// 11 → T
#[inline]
fn decode_base(encoded: u8) -> u8 {
    match encoded & 0b11 {
        0b00 => b'A',
        0b01 => b'C',
        0b10 => b'G',
        0b11 => b'T',
        _ => unreachable!(), // Only 2 bits, max value 0b11
    }
}

/// Reverse 2-bit pairs within a byte
///
/// Example: 0b00011011 (ACGT) → 0b11100100 (TGCA)
///          Pair 0 (00) → Position 3
///          Pair 1 (01) → Position 2
///          Pair 2 (10) → Position 1
///          Pair 3 (11) → Position 0
#[inline]
fn reverse_2bit_pairs(byte: u8) -> u8 {
    let p0 = (byte >> 6) & 0b11; // Extract pair 0 (bits 6-7)
    let p1 = (byte >> 4) & 0b11; // Extract pair 1 (bits 4-5)
    let p2 = (byte >> 2) & 0b11; // Extract pair 2 (bits 2-3)
    let p3 = byte & 0b11; // Extract pair 3 (bits 0-1)

    // Reassemble in reverse order
    (p3 << 6) | (p2 << 4) | (p1 << 2) | p0
}

impl fmt::Display for BitSeq {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let ascii = self.to_ascii();
        write!(f, "{}", String::from_utf8_lossy(&ascii))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encode_decode_roundtrip() {
        let sequences = vec![
            b"ACGT".to_vec(),
            b"AAAA".to_vec(),
            b"TTTT".to_vec(),
            b"ACGTACGT".to_vec(),
            b"ACG".to_vec(), // Not multiple of 4
            b"ACGTACGTACGTACGT".to_vec(),
        ];

        for seq in sequences {
            let bitseq = BitSeq::from_ascii(&seq);
            let decoded = bitseq.to_ascii();
            assert_eq!(
                seq, decoded,
                "Roundtrip failed for {}",
                String::from_utf8_lossy(&seq)
            );
        }
    }

    #[test]
    fn test_encoding_correctness() {
        // "ACGT" → 0b00011011 → 0x1B
        let bitseq = BitSeq::from_ascii(b"ACGT");
        assert_eq!(bitseq.len(), 4);
        assert_eq!(bitseq.data().len(), 1);
        assert_eq!(bitseq.data()[0], 0x1B); // 00=A, 01=C, 10=G, 11=T
    }

    #[test]
    fn test_get() {
        let bitseq = BitSeq::from_ascii(b"ACGT");
        assert_eq!(bitseq.get(0), Some(b'A'));
        assert_eq!(bitseq.get(1), Some(b'C'));
        assert_eq!(bitseq.get(2), Some(b'G'));
        assert_eq!(bitseq.get(3), Some(b'T'));
        assert_eq!(bitseq.get(4), None);
    }

    #[test]
    fn test_reverse_complement() {
        let test_cases: Vec<(&[u8], &[u8])> = vec![
            (b"ACGT", b"ACGT"), // Palindrome
            (b"AAAA", b"TTTT"),
            (b"ACGTACGT", b"ACGTACGT"), // Palindrome
            (b"ATCG", b"CGAT"),
            (b"ACG", b"CGT"),  // Partial byte
            (b"ACGTA", b"TACGT"), // Partial byte
        ];

        for (input, expected) in test_cases {
            let bitseq = BitSeq::from_ascii(input);
            let result = bitseq.reverse_complement();
            let result_ascii = result.to_ascii();
            assert_eq!(
                &result_ascii, &expected.to_vec(),
                "Reverse complement failed for {}",
                String::from_utf8_lossy(input)
            );
        }
    }

    #[test]
    fn test_complement() {
        let test_cases: Vec<(&[u8], &[u8])> = vec![
            (b"ACGT", b"TGCA"),
            (b"AAAA", b"TTTT"),
            (b"CCCC", b"GGGG"),
            (b"ACG", b"TGC"),
        ];

        for (input, expected) in test_cases {
            let bitseq = BitSeq::from_ascii(input);
            let result = bitseq.complement();
            let result_ascii = result.to_ascii();
            assert_eq!(
                &result_ascii, &expected.to_vec(),
                "Complement failed for {}",
                String::from_utf8_lossy(input)
            );
        }
    }

    #[test]
    fn test_count_base() {
        let bitseq = BitSeq::from_ascii(b"ACGTACGT");
        assert_eq!(bitseq.count_base(b'A'), 2);
        assert_eq!(bitseq.count_base(b'C'), 2);
        assert_eq!(bitseq.count_base(b'G'), 2);
        assert_eq!(bitseq.count_base(b'T'), 2);
    }

    #[test]
    fn test_count_gc() {
        let bitseq = BitSeq::from_ascii(b"ACGTACGT");
        assert_eq!(bitseq.count_gc(), 4); // 2 G + 2 C

        let bitseq = BitSeq::from_ascii(b"AAATTT");
        assert_eq!(bitseq.count_gc(), 0);

        let bitseq = BitSeq::from_ascii(b"GCGCGC");
        assert_eq!(bitseq.count_gc(), 6);
    }

    #[test]
    fn test_count_at() {
        let bitseq = BitSeq::from_ascii(b"ACGTACGT");
        assert_eq!(bitseq.count_at(), 4); // 2 A + 2 T

        let bitseq = BitSeq::from_ascii(b"GCGCGC");
        assert_eq!(bitseq.count_at(), 0);

        let bitseq = BitSeq::from_ascii(b"ATATAT");
        assert_eq!(bitseq.count_at(), 6);
    }

    #[test]
    fn test_reverse_2bit_pairs() {
        assert_eq!(reverse_2bit_pairs(0b00011011), 0b11100100); // ACGT → TGCA
        assert_eq!(reverse_2bit_pairs(0b00000000), 0b00000000); // AAAA → AAAA
        assert_eq!(reverse_2bit_pairs(0b11111111), 0b11111111); // TTTT → TTTT
    }

    #[test]
    fn test_case_insensitive() {
        let upper = BitSeq::from_ascii(b"ACGT");
        let lower = BitSeq::from_ascii(b"acgt");
        let mixed = BitSeq::from_ascii(b"AcGt");

        assert_eq!(upper.to_ascii(), b"ACGT");
        assert_eq!(lower.to_ascii(), b"ACGT"); // Decoded as uppercase
        assert_eq!(mixed.to_ascii(), b"ACGT");
    }

    #[test]
    fn test_invalid_bases() {
        // N and other invalid bases default to A (00)
        let bitseq = BitSeq::from_ascii(b"ACNGT");
        let decoded = bitseq.to_ascii();
        assert_eq!(decoded, b"ACAGT"); // N → A
    }
}
