//! High-level kernel interfaces for bioinformatics operations
//!
//! This module provides Rust-friendly wrappers around Metal compute kernels.

use crate::{GpuMetrics, MetalBackend};
use anyhow::Result;
use asbb_core::SequenceRecord;

/// Result of base counting operation
#[derive(Debug, Clone)]
pub struct BaseCountsGpu {
    pub count_a: usize,
    pub count_c: usize,
    pub count_g: usize,
    pub count_t: usize,
    pub total_bases: usize,
    pub metrics: GpuMetrics,
}

/// Result of quality aggregation operation
#[derive(Debug, Clone)]
pub struct QualityStatsGpu {
    pub min_quality: u8,
    pub max_quality: u8,
    pub total_quality: u64,
    pub num_bases: usize,
}

impl MetalBackend {
    /// Count bases using GPU
    ///
    /// This dispatches the `count_bases` Metal kernel, which counts A, C, G, T
    /// in parallel across all sequences.
    ///
    /// ## Performance
    ///
    /// - Dispatch overhead: ~50-100ms (fixed cost)
    /// - Break-even: ~50,000 sequences
    /// - Maximum speedup: 4-6× vs CPU NEON (for large batches)
    ///
    /// ## Returns
    ///
    /// Aggregate counts across all sequences plus performance metrics.
    pub fn count_bases_gpu(&self, data: &[SequenceRecord]) -> Result<BaseCountsGpu> {
        if data.is_empty() {
            return Ok(BaseCountsGpu {
                count_a: 0,
                count_c: 0,
                count_g: 0,
                count_t: 0,
                total_bases: 0,
                metrics: GpuMetrics {
                    total_time_ms: 0.0,
                    kernel_time_ms: 0.0,
                    overhead_ms: 0.0,
                    num_sequences: 0,
                    throughput: 0.0,
                },
            });
        }

        // Flatten sequences into contiguous buffer
        let mut flat_sequences = Vec::new();
        let mut seq_offsets = Vec::new();
        let mut seq_lengths = Vec::new();

        for record in data {
            seq_offsets.push(flat_sequences.len() as u32);
            seq_lengths.push(record.sequence.len() as u32);
            flat_sequences.extend_from_slice(&record.sequence);
        }

        // Create GPU buffers (unified memory - no copy)
        let sequences_buffer = self.create_buffer(&flat_sequences);
        let offsets_buffer = self.create_buffer(&seq_offsets);
        let lengths_buffer = self.create_buffer(&seq_lengths);

        // Create output buffer (4 counts per sequence: A, C, G, T)
        let output_size = (data.len() * 4 * std::mem::size_of::<u32>()) as u64;
        let counts_buffer = self.create_empty_buffer(output_size);

        // Dispatch kernel
        let metrics = self.dispatch_kernel(
            "count_bases",
            &[&sequences_buffer, &offsets_buffer, &lengths_buffer, &counts_buffer],
            data.len(),
        )?;

        // Read results from GPU buffer (unified memory - direct access)
        let counts_ptr = counts_buffer.contents() as *const u32;
        let counts = unsafe {
            std::slice::from_raw_parts(counts_ptr, data.len() * 4)
        };

        // Aggregate results
        let mut total_a = 0;
        let mut total_c = 0;
        let mut total_g = 0;
        let mut total_t = 0;

        for i in 0..data.len() {
            let base_idx = i * 4;
            total_a += counts[base_idx] as usize;
            total_c += counts[base_idx + 1] as usize;
            total_g += counts[base_idx + 2] as usize;
            total_t += counts[base_idx + 3] as usize;
        }

        let total_bases = total_a + total_c + total_g + total_t;

        Ok(BaseCountsGpu {
            count_a: total_a,
            count_c: total_c,
            count_g: total_g,
            count_t: total_t,
            total_bases,
            metrics,
        })
    }

    /// Count GC bases using GPU
    pub fn count_gc_gpu(&self, data: &[SequenceRecord]) -> Result<(usize, usize, GpuMetrics)> {
        if data.is_empty() {
            return Ok((0, 0, GpuMetrics {
                total_time_ms: 0.0,
                kernel_time_ms: 0.0,
                overhead_ms: 0.0,
                num_sequences: 0,
                throughput: 0.0,
            }));
        }

        // Flatten sequences
        let mut flat_sequences = Vec::new();
        let mut seq_offsets = Vec::new();
        let mut seq_lengths = Vec::new();
        let mut total_bases = 0;

        for record in data {
            seq_offsets.push(flat_sequences.len() as u32);
            seq_lengths.push(record.sequence.len() as u32);
            flat_sequences.extend_from_slice(&record.sequence);
            total_bases += record.sequence.len();
        }

        // Create buffers
        let sequences_buffer = self.create_buffer(&flat_sequences);
        let offsets_buffer = self.create_buffer(&seq_offsets);
        let lengths_buffer = self.create_buffer(&seq_lengths);

        let output_size = (data.len() * std::mem::size_of::<u32>()) as u64;
        let gc_counts_buffer = self.create_empty_buffer(output_size);

        // Dispatch
        let metrics = self.dispatch_kernel(
            "count_gc",
            &[&sequences_buffer, &offsets_buffer, &lengths_buffer, &gc_counts_buffer],
            data.len(),
        )?;

        // Read results
        let counts_ptr = gc_counts_buffer.contents() as *const u32;
        let counts = unsafe {
            std::slice::from_raw_parts(counts_ptr, data.len())
        };

        let total_gc: usize = counts.iter().map(|&c| c as usize).sum();

        Ok((total_gc, total_bases, metrics))
    }

    /// Count AT bases using GPU
    pub fn count_at_gpu(&self, data: &[SequenceRecord]) -> Result<(usize, usize, GpuMetrics)> {
        if data.is_empty() {
            return Ok((0, 0, GpuMetrics {
                total_time_ms: 0.0,
                kernel_time_ms: 0.0,
                overhead_ms: 0.0,
                num_sequences: 0,
                throughput: 0.0,
            }));
        }

        // Flatten sequences
        let mut flat_sequences = Vec::new();
        let mut seq_offsets = Vec::new();
        let mut seq_lengths = Vec::new();
        let mut total_bases = 0;

        for record in data {
            seq_offsets.push(flat_sequences.len() as u32);
            seq_lengths.push(record.sequence.len() as u32);
            flat_sequences.extend_from_slice(&record.sequence);
            total_bases += record.sequence.len();
        }

        // Create buffers
        let sequences_buffer = self.create_buffer(&flat_sequences);
        let offsets_buffer = self.create_buffer(&seq_offsets);
        let lengths_buffer = self.create_buffer(&seq_lengths);

        let output_size = (data.len() * std::mem::size_of::<u32>()) as u64;
        let at_counts_buffer = self.create_empty_buffer(output_size);

        // Dispatch
        let metrics = self.dispatch_kernel(
            "count_at",
            &[&sequences_buffer, &offsets_buffer, &lengths_buffer, &at_counts_buffer],
            data.len(),
        )?;

        // Read results
        let counts_ptr = at_counts_buffer.contents() as *const u32;
        let counts = unsafe {
            std::slice::from_raw_parts(counts_ptr, data.len())
        };

        let total_at: usize = counts.iter().map(|&c| c as usize).sum();

        Ok((total_at, total_bases, metrics))
    }

    /// Aggregate quality scores using GPU
    ///
    /// Computes min/max/sum quality scores across all sequences.
    /// Returns aggregate statistics and performance metrics.
    pub fn aggregate_quality_gpu(&self, data: &[SequenceRecord]) -> Result<(QualityStatsGpu, GpuMetrics)> {
        if data.is_empty() {
            return Ok((QualityStatsGpu {
                min_quality: 255,
                max_quality: 0,
                total_quality: 0,
                num_bases: 0,
            }, GpuMetrics {
                total_time_ms: 0.0,
                kernel_time_ms: 0.0,
                overhead_ms: 0.0,
                num_sequences: 0,
                throughput: 0.0,
            }));
        }

        // Flatten quality scores into contiguous buffer
        let mut flat_quality = Vec::new();
        let mut seq_offsets = Vec::new();
        let mut seq_lengths = Vec::new();
        let mut total_bases = 0;

        for record in data {
            if let Some(qual) = &record.quality {
                seq_offsets.push(flat_quality.len() as u32);
                seq_lengths.push(qual.len() as u32);
                flat_quality.extend_from_slice(qual);
                total_bases += qual.len();
            } else {
                // No quality scores - skip this sequence
                seq_offsets.push(flat_quality.len() as u32);
                seq_lengths.push(0);
            }
        }

        // Create GPU buffers
        let quality_buffer = self.create_buffer(&flat_quality);
        let offsets_buffer = self.create_buffer(&seq_offsets);
        let lengths_buffer = self.create_buffer(&seq_lengths);

        // Create output buffer (4 values per sequence: min, max, sum_low, sum_high)
        let output_size = (data.len() * 4 * std::mem::size_of::<u32>()) as u64;
        let stats_buffer = self.create_empty_buffer(output_size);

        // Dispatch kernel
        let metrics = self.dispatch_kernel(
            "aggregate_quality",
            &[&quality_buffer, &offsets_buffer, &lengths_buffer, &stats_buffer],
            data.len(),
        )?;

        // Read results from GPU buffer
        let stats_ptr = stats_buffer.contents() as *const u32;
        let stats_data = unsafe {
            std::slice::from_raw_parts(stats_ptr, data.len() * 4)
        };

        // Aggregate results across all sequences
        let mut global_min = 255u8;
        let mut global_max = 0u8;
        let mut global_sum = 0u64;

        for i in 0..data.len() {
            let base_idx = i * 4;
            let min_q = stats_data[base_idx] as u8;
            let max_q = stats_data[base_idx + 1] as u8;
            let sum_low = stats_data[base_idx + 2] as u64;
            let sum_high = stats_data[base_idx + 3] as u64;
            let sum_q = (sum_high << 32) | sum_low;

            // Skip sequences with no quality scores (length was 0)
            if seq_lengths[i] == 0 {
                continue;
            }

            global_min = global_min.min(min_q);
            global_max = global_max.max(max_q);
            global_sum += sum_q;
        }

        Ok((QualityStatsGpu {
            min_quality: global_min,
            max_quality: global_max,
            total_quality: global_sum,
            num_bases: total_bases,
        }, metrics))
    }

    /// Reverse complement sequences using GPU
    ///
    /// Returns the reverse complemented sequences and performance metrics.
    pub fn reverse_complement_gpu(&self, data: &[SequenceRecord]) -> Result<(Vec<SequenceRecord>, GpuMetrics)> {
        if data.is_empty() {
            return Ok((Vec::new(), GpuMetrics {
                total_time_ms: 0.0,
                kernel_time_ms: 0.0,
                overhead_ms: 0.0,
                num_sequences: 0,
                throughput: 0.0,
            }));
        }

        // Flatten sequences
        let mut flat_sequences = Vec::new();
        let mut seq_offsets = Vec::new();
        let mut seq_lengths = Vec::new();

        for record in data {
            seq_offsets.push(flat_sequences.len() as u32);
            seq_lengths.push(record.sequence.len() as u32);
            flat_sequences.extend_from_slice(&record.sequence);
        }

        // Create input buffers
        let sequences_buffer = self.create_buffer(&flat_sequences);
        let offsets_buffer = self.create_buffer(&seq_offsets);
        let lengths_buffer = self.create_buffer(&seq_lengths);

        // Create output buffer (same size as input)
        let output_size = flat_sequences.len() as u64;
        let output_buffer = self.create_empty_buffer(output_size);

        // Output offsets (same as input offsets)
        let out_offsets_buffer = self.create_buffer(&seq_offsets);

        // Dispatch
        let metrics = self.dispatch_kernel(
            "reverse_complement",
            &[&sequences_buffer, &offsets_buffer, &lengths_buffer, &output_buffer, &out_offsets_buffer],
            data.len(),
        )?;

        // Read results
        let output_ptr = output_buffer.contents() as *const u8;
        let output_data = unsafe {
            std::slice::from_raw_parts(output_ptr, flat_sequences.len())
        };

        // Reconstruct sequences
        let mut results = Vec::with_capacity(data.len());
        for (i, record) in data.iter().enumerate() {
            let offset = seq_offsets[i] as usize;
            let length = seq_lengths[i] as usize;
            let seq_data = output_data[offset..offset + length].to_vec();

            results.push(SequenceRecord::fasta(
                format!("{}_revcomp", record.id),
                seq_data,
            ));
        }

        Ok((results, metrics))
    }

    /// Calculate complexity scores using GPU
    pub fn calculate_complexity_gpu(&self, data: &[SequenceRecord]) -> Result<(Vec<f64>, GpuMetrics)> {
        if data.is_empty() {
            return Ok((Vec::new(), GpuMetrics {
                total_time_ms: 0.0,
                kernel_time_ms: 0.0,
                overhead_ms: 0.0,
                num_sequences: 0,
                throughput: 0.0,
            }));
        }

        // Flatten sequences
        let mut flat_sequences = Vec::new();
        let mut seq_offsets = Vec::new();
        let mut seq_lengths = Vec::new();

        for record in data {
            seq_offsets.push(flat_sequences.len() as u32);
            seq_lengths.push(record.sequence.len() as u32);
            flat_sequences.extend_from_slice(&record.sequence);
        }

        // Create buffers
        let sequences_buffer = self.create_buffer(&flat_sequences);
        let offsets_buffer = self.create_buffer(&seq_offsets);
        let lengths_buffer = self.create_buffer(&seq_lengths);

        let output_size = (data.len() * std::mem::size_of::<u32>()) as u64;
        let complexity_buffer = self.create_empty_buffer(output_size);

        // Dispatch
        let metrics = self.dispatch_kernel(
            "calculate_complexity",
            &[&sequences_buffer, &offsets_buffer, &lengths_buffer, &complexity_buffer],
            data.len(),
        )?;

        // Read results (scaled integers, divide by 1000 to get f64)
        let complexity_ptr = complexity_buffer.contents() as *const u32;
        let complexity_data = unsafe {
            std::slice::from_raw_parts(complexity_ptr, data.len())
        };

        let results: Vec<f64> = complexity_data
            .iter()
            .map(|&scaled| scaled as f64 / 1000.0)
            .collect();

        Ok((results, metrics))
    }
}
