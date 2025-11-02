//! Compression/decompression utilities for Hardware Compression pilot
//!
//! Provides functions to decompress FASTQ data using various algorithms:
//! - gzip (flate2): Software baseline
//! - zstd: Fast compression with good ratio

use anyhow::Result;
use flate2::read::GzDecoder;
use std::fs::File;
use std::io::Read;

/// Compression algorithm
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompressionAlgorithm {
    /// No compression (uncompressed file)
    None,
    /// gzip compression (software, baseline)
    Gzip,
    /// zstd compression (fast, good ratio)
    Zstd,
}

impl CompressionAlgorithm {
    pub fn name(&self) -> &'static str {
        match self {
            CompressionAlgorithm::None => "uncompressed",
            CompressionAlgorithm::Gzip => "gzip",
            CompressionAlgorithm::Zstd => "zstd",
        }
    }
}

/// Decompress a file using the specified algorithm
///
/// Returns the decompressed bytes
pub fn decompress_file(path: &str, algorithm: CompressionAlgorithm) -> Result<Vec<u8>> {
    use anyhow::Context;

    match algorithm {
        CompressionAlgorithm::None => {
            // Just read the file as-is
            let mut file = File::open(path)
                .with_context(|| format!("Failed to open file: {}", path))?;
            let mut contents = Vec::new();
            file.read_to_end(&mut contents)?;
            Ok(contents)
        }
        CompressionAlgorithm::Gzip => {
            // Decompress gzip
            let file = File::open(path)
                .with_context(|| format!("Failed to open gzip file: {}", path))?;
            let mut decoder = GzDecoder::new(file);
            let mut contents = Vec::new();
            decoder.read_to_end(&mut contents)?;
            Ok(contents)
        }
        CompressionAlgorithm::Zstd => {
            // Decompress zstd
            let file = File::open(path)
                .with_context(|| format!("Failed to open zstd file: {}", path))?;
            let mut decoder = zstd::Decoder::new(file)?;
            let mut contents = Vec::new();
            decoder.read_to_end(&mut contents)?;
            Ok(contents)
        }
    }
}

/// Parse FASTQ from bytes (in-memory)
///
/// Simple FASTQ parser for compression pilot.
/// Returns vector of sequence records.
pub fn parse_fastq_from_bytes(bytes: &[u8]) -> Result<Vec<crate::SequenceRecord>> {
    use asbb_core::SequenceRecord;

    let mut records = Vec::new();
    let mut lines = bytes.split(|&b| b == b'\n');

    loop {
        // Read 4 lines for each FASTQ record
        let id_line = match lines.next() {
            Some(line) if !line.is_empty() => line,
            _ => break, // End of file or empty line
        };

        let seq_line = match lines.next() {
            Some(line) => line,
            None => break,
        };

        let _plus_line = match lines.next() {
            Some(_) => (),
            None => break,
        };

        let qual_line = match lines.next() {
            Some(line) => line,
            None => break,
        };

        // Parse ID (skip '@' prefix)
        let id = if id_line.starts_with(b"@") {
            String::from_utf8_lossy(&id_line[1..]).to_string()
        } else {
            String::from_utf8_lossy(id_line).to_string()
        };

        let sequence = seq_line.to_vec();
        let quality = qual_line.to_vec();

        records.push(SequenceRecord::fastq(id, sequence, quality));
    }

    Ok(records)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compression_algorithm_name() {
        assert_eq!(CompressionAlgorithm::None.name(), "uncompressed");
        assert_eq!(CompressionAlgorithm::Gzip.name(), "gzip");
        assert_eq!(CompressionAlgorithm::Zstd.name(), "zstd");
    }
}
