//! GPU (Metal) Backend for Apple Silicon Bio Bench
//!
//! This crate provides Metal compute shader implementations for bioinformatics
//! operations, leveraging Apple Silicon's unified memory architecture.
//!
//! ## Architecture
//!
//! - **Unified Memory**: Zero-copy data access (CPU and GPU share memory)
//! - **Metal Compute**: Direct GPU access via Metal API
//! - **Batch Processing**: Optimal for large datasets (>50K sequences)
//!
//! ## Performance Characteristics (from BioMetal)
//!
//! - **Dispatch overhead**: ~50-100ms per kernel launch
//! - **Break-even point**: ~50,000 sequences (batch size cliff)
//! - **Maximum speedup**: 4-8× for embarrassingly parallel operations
//!
//! ## Usage
//!
//! ```rust,ignore
//! use asbb_gpu::MetalBackend;
//!
//! let backend = MetalBackend::new()?;
//! let results = backend.count_bases(&sequences)?;
//! ```

use anyhow::{Context, Result};
use metal::*;
use std::time::Instant;

pub mod kernels;

/// Metal GPU backend for bioinformatics operations
pub struct MetalBackend {
    device: Device,
    command_queue: CommandQueue,
    library: Library,
}

/// Performance metrics for GPU operations
#[derive(Debug, Clone)]
pub struct GpuMetrics {
    /// Total execution time (including overhead)
    pub total_time_ms: f64,

    /// GPU kernel execution time only
    pub kernel_time_ms: f64,

    /// Dispatch overhead (setup + data transfer if any)
    pub overhead_ms: f64,

    /// Number of sequences processed
    pub num_sequences: usize,

    /// Throughput (sequences/second)
    pub throughput: f64,
}

impl MetalBackend {
    /// Create a new Metal backend
    ///
    /// This initializes the default Metal device and creates a command queue.
    /// On Apple Silicon, this gives access to the unified memory GPU.
    pub fn new() -> Result<Self> {
        // Get the default Metal device (Apple Silicon GPU)
        let device = Device::system_default()
            .context("No Metal device found - Apple Silicon required")?;

        // Create command queue for submitting work
        let command_queue = device.new_command_queue();

        // Compile Metal shaders
        let library = Self::compile_shaders(&device)?;

        Ok(Self {
            device,
            command_queue,
            library,
        })
    }

    /// Compile Metal shader library from source
    fn compile_shaders(device: &Device) -> Result<Library> {
        // Include the shader source at compile time
        let shader_source = include_str!("shaders/operations.metal");

        // Compile options
        let options = CompileOptions::new();

        // Compile the library
        device
            .new_library_with_source(shader_source, &options)
            .map_err(|e| anyhow::anyhow!("Failed to compile Metal shaders: {}", e))
    }

    /// Get the Metal device
    pub fn device(&self) -> &Device {
        &self.device
    }

    /// Get the command queue
    pub fn command_queue(&self) -> &CommandQueue {
        &self.command_queue
    }

    /// Get the shader library
    pub fn library(&self) -> &Library {
        &self.library
    }

    /// Create a GPU buffer from data
    ///
    /// On Apple Silicon, this uses unified memory, so the buffer is accessible
    /// to both CPU and GPU without copying.
    pub fn create_buffer<T>(&self, data: &[T]) -> Buffer {
        let size = std::mem::size_of_val(data) as u64;
        let ptr = data.as_ptr() as *const std::ffi::c_void;

        // MTLResourceStorageModeShared = unified memory (no copy)
        self.device.new_buffer_with_data(
            ptr,
            size,
            MTLResourceOptions::StorageModeShared,
        )
    }

    /// Create an empty GPU buffer
    pub fn create_empty_buffer(&self, size: u64) -> Buffer {
        self.device.new_buffer(size, MTLResourceOptions::StorageModeShared)
    }

    /// Dispatch a compute kernel
    ///
    /// This is a low-level method for executing Metal compute shaders.
    pub fn dispatch_kernel(
        &self,
        kernel_name: &str,
        buffers: &[&Buffer],
        grid_size: usize,
    ) -> Result<GpuMetrics> {
        let start_total = Instant::now();

        // Get the kernel function
        let function = self
            .library
            .get_function(kernel_name, None)
            .map_err(|e| anyhow::anyhow!("Kernel function '{}' not found: {}", kernel_name, e))?;

        // Create compute pipeline
        let pipeline = self
            .device
            .new_compute_pipeline_state_with_function(&function)
            .map_err(|e| anyhow::anyhow!("Failed to create pipeline: {}", e))?;

        // Create command buffer
        let command_buffer = self.command_queue.new_command_buffer();

        // Create compute encoder
        let encoder = command_buffer.new_compute_command_encoder();

        // Set the pipeline
        encoder.set_compute_pipeline_state(&pipeline);

        // Bind buffers
        for (i, buffer) in buffers.iter().enumerate() {
            encoder.set_buffer(i as u64, Some(*buffer), 0);
        }

        let overhead_start = Instant::now();

        // Calculate threadgroup size
        let threadgroup_size = MTLSize {
            width: pipeline.max_total_threads_per_threadgroup().min(grid_size as u64),
            height: 1,
            depth: 1,
        };

        // Calculate grid size
        let grid_size = MTLSize {
            width: grid_size as u64,
            height: 1,
            depth: 1,
        };

        // Dispatch threads
        encoder.dispatch_threads(grid_size, threadgroup_size);

        // End encoding
        encoder.end_encoding();

        let overhead_ms = overhead_start.elapsed().as_secs_f64() * 1000.0;

        // Commit and wait
        let kernel_start = Instant::now();
        command_buffer.commit();
        command_buffer.wait_until_completed();
        let kernel_time_ms = kernel_start.elapsed().as_secs_f64() * 1000.0;

        let total_time_ms = start_total.elapsed().as_secs_f64() * 1000.0;

        Ok(GpuMetrics {
            total_time_ms,
            kernel_time_ms,
            overhead_ms,
            num_sequences: grid_size.width as usize,
            throughput: grid_size.width as f64 / (total_time_ms / 1000.0),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metal_backend_creation() {
        let backend = MetalBackend::new();
        assert!(backend.is_ok(), "Failed to create Metal backend - Apple Silicon required");
    }

    #[test]
    fn test_device_info() {
        if let Ok(backend) = MetalBackend::new() {
            let device = backend.device();
            println!("Metal Device: {}", device.name());
            println!("Unified Memory: {}", device.has_unified_memory());
            println!("Max Threads Per Threadgroup: {}",
                     device.max_threads_per_threadgroup().width);
        }
    }
}
