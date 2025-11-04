/// Metal GPU Feasibility Test - Phase 1
///
/// Measures:
/// 1. GPU dispatch overhead (kernel launch latency)
/// 2. Memory bandwidth (CPU↔GPU via unified memory)
/// 3. Baseline GPU vs CPU performance
///
/// Purpose: Determine if Metal GPU implementation is viable for bgzip decompression

use metal::*;
use std::time::Instant;

const SHADER_SOURCE: &str = include_str!("../../shaders/memory_copy.metal");

fn main() {
    println!("================================================================================");
    println!("METAL GPU FEASIBILITY TEST - PHASE 1");
    println!("================================================================================");
    println!();
    println!("Testing GPU dispatch overhead and memory bandwidth");
    println!("Platform: Apple M4 Max (40 GPU cores, 546 GB/s memory bandwidth)");
    println!();

    // Initialize Metal
    let device = Device::system_default().expect("No Metal device found");
    println!("Metal device: {}", device.name());
    println!("Unified memory: {}", device.has_unified_memory());
    println!("Max threads per threadgroup: {}", device.max_threads_per_threadgroup().width);
    println!();

    // Compile shader
    println!("Compiling Metal shader...");
    let library = device.new_library_with_source(SHADER_SOURCE, &CompileOptions::new())
        .expect("Failed to compile shader");

    let memory_copy = library.get_function("memory_copy", None)
        .expect("Failed to get memory_copy function");

    let memory_copy_blocks = library.get_function("memory_copy_blocks", None)
        .expect("Failed to get memory_copy_blocks function");

    // Create pipeline states
    let pipeline_simple = device.new_compute_pipeline_state_with_function(&memory_copy)
        .expect("Failed to create pipeline state");

    let pipeline_blocks = device.new_compute_pipeline_state_with_function(&memory_copy_blocks)
        .expect("Failed to create pipeline state");

    println!("✓ Shader compiled successfully");
    println!();

    // Test 1: Dispatch Overhead
    println!("================================================================================");
    println!("Test 1: GPU Dispatch Overhead");
    println!("================================================================================");
    println!();

    test_dispatch_overhead(&device, &pipeline_simple);

    // Test 2: Memory Bandwidth
    println!();
    println!("================================================================================");
    println!("Test 2: Memory Bandwidth (Unified Memory)");
    println!("================================================================================");
    println!();

    test_memory_bandwidth(&device, &pipeline_simple);

    // Test 3: Block-based Processing
    println!();
    println!("================================================================================");
    println!("Test 3: Block-Based Processing (bgzip simulation)");
    println!("================================================================================");
    println!();

    test_block_processing(&device, &pipeline_blocks);

    // Summary and Decision
    println!();
    println!("================================================================================");
    println!("PHASE 1 RESULTS SUMMARY");
    println!("================================================================================");
    println!();
    println!("See results above for detailed measurements.");
    println!();
    println!("Decision criteria:");
    println!("  - Dispatch overhead <100 µs: ✅ Proceed to Phase 2");
    println!("  - Dispatch overhead >100 µs: ❌ GPU overhead too high");
    println!();
}

fn test_dispatch_overhead(device: &Device, pipeline: &ComputePipelineState) {
    println!("Measuring GPU kernel dispatch overhead...");
    println!("(Small workload to isolate dispatch cost)");
    println!();

    let size = 1024u32; // 1 KB
    let data = vec![0u8; size as usize];

    // Create buffers
    let input_buffer = device.new_buffer_with_data(
        data.as_ptr() as *const _,
        data.len() as u64,
        MTLResourceOptions::StorageModeShared,
    );

    let output_buffer = device.new_buffer(
        data.len() as u64,
        MTLResourceOptions::StorageModeShared,
    );

    let size_buffer = device.new_buffer_with_data(
        &size as *const u32 as *const _,
        std::mem::size_of::<u32>() as u64,
        MTLResourceOptions::StorageModeShared,
    );

    // Create command queue
    let command_queue = device.new_command_queue();

    // Warm-up (first dispatch may be slower)
    {
        let command_buffer = command_queue.new_command_buffer();
        let encoder = command_buffer.new_compute_command_encoder();
        encoder.set_compute_pipeline_state(pipeline);
        encoder.set_buffer(0, Some(&input_buffer), 0);
        encoder.set_buffer(1, Some(&output_buffer), 0);
        encoder.set_buffer(2, Some(&size_buffer), 0);

        let grid_size = MTLSize::new(size as u64, 1, 1);
        let threadgroup_size = MTLSize::new(256, 1, 1);
        encoder.dispatch_threads(grid_size, threadgroup_size);
        encoder.end_encoding();

        command_buffer.commit();
        command_buffer.wait_until_completed();
    }

    // Benchmark: Measure dispatch overhead (N=100 runs)
    let iterations = 100;
    let start = Instant::now();

    for _ in 0..iterations {
        let command_buffer = command_queue.new_command_buffer();
        let encoder = command_buffer.new_compute_command_encoder();
        encoder.set_compute_pipeline_state(pipeline);
        encoder.set_buffer(0, Some(&input_buffer), 0);
        encoder.set_buffer(1, Some(&output_buffer), 0);
        encoder.set_buffer(2, Some(&size_buffer), 0);

        let grid_size = MTLSize::new(size as u64, 1, 1);
        let threadgroup_size = MTLSize::new(256, 1, 1);
        encoder.dispatch_threads(grid_size, threadgroup_size);
        encoder.end_encoding();

        command_buffer.commit();
        command_buffer.wait_until_completed();
    }

    let elapsed = start.elapsed();
    let avg_dispatch = elapsed.as_micros() / iterations;

    println!("Results:");
    println!("  Total time: {:.2} ms ({} dispatches)", elapsed.as_secs_f64() * 1000.0, iterations);
    println!("  Average dispatch: {} µs", avg_dispatch);
    println!("  Workload: {} bytes copied", size);
    println!();

    if avg_dispatch < 100 {
        println!("✅ Dispatch overhead acceptable (<100 µs)");
        println!("   GPU implementation is viable!");
    } else {
        println!("⚠️  Dispatch overhead high (>100 µs)");
        println!("   May limit benefit for small blocks");
    }
}

fn test_memory_bandwidth(device: &Device, pipeline: &ComputePipelineState) {
    println!("Measuring memory bandwidth with unified memory...");
    println!();

    // Test various buffer sizes
    let sizes = vec![
        (1_000, "1 KB"),
        (10_000, "10 KB"),
        (100_000, "100 KB"),
        (1_000_000, "1 MB"),
        (10_000_000, "10 MB"),
    ];

    println!("| Size    | GPU Time | Bandwidth | vs CPU   |");
    println!("|---------| ---------|-----------|----------|");

    for (size, label) in sizes {
        let data = vec![0u8; size];

        // GPU timing
        let input_buffer = device.new_buffer_with_data(
            data.as_ptr() as *const _,
            data.len() as u64,
            MTLResourceOptions::StorageModeShared,
        );

        let output_buffer = device.new_buffer(
            data.len() as u64,
            MTLResourceOptions::StorageModeShared,
        );

        let size_u32 = size as u32;
        let size_buffer = device.new_buffer_with_data(
            &size_u32 as *const u32 as *const _,
            std::mem::size_of::<u32>() as u64,
            MTLResourceOptions::StorageModeShared,
        );

        let command_queue = device.new_command_queue();

        // Warm-up
        {
            let command_buffer = command_queue.new_command_buffer();
            let encoder = command_buffer.new_compute_command_encoder();
            encoder.set_compute_pipeline_state(pipeline);
            encoder.set_buffer(0, Some(&input_buffer), 0);
            encoder.set_buffer(1, Some(&output_buffer), 0);
            encoder.set_buffer(2, Some(&size_buffer), 0);

            let grid_size = MTLSize::new(size as u64, 1, 1);
            let threadgroup_size = MTLSize::new(256, 1, 1);
            encoder.dispatch_threads(grid_size, threadgroup_size);
            encoder.end_encoding();

            command_buffer.commit();
            command_buffer.wait_until_completed();
        }

        // GPU benchmark (N=30)
        let iterations = 30;
        let gpu_start = Instant::now();

        for _ in 0..iterations {
            let command_buffer = command_queue.new_command_buffer();
            let encoder = command_buffer.new_compute_command_encoder();
            encoder.set_compute_pipeline_state(pipeline);
            encoder.set_buffer(0, Some(&input_buffer), 0);
            encoder.set_buffer(1, Some(&output_buffer), 0);
            encoder.set_buffer(2, Some(&size_buffer), 0);

            let grid_size = MTLSize::new(size as u64, 1, 1);
            let threadgroup_size = MTLSize::new(256, 1, 1);
            encoder.dispatch_threads(grid_size, threadgroup_size);
            encoder.end_encoding();

            command_buffer.commit();
            command_buffer.wait_until_completed();
        }

        let gpu_elapsed = gpu_start.elapsed().as_secs_f64() / iterations as f64;
        let gpu_bandwidth = (size as f64 * 2.0) / gpu_elapsed / 1_000_000_000.0; // GB/s (read + write)

        // CPU baseline (memcpy)
        let mut cpu_output = vec![0u8; size];
        let cpu_start = Instant::now();

        for _ in 0..iterations {
            cpu_output.copy_from_slice(&data);
        }

        let cpu_elapsed = cpu_start.elapsed().as_secs_f64() / iterations as f64;
        let _cpu_bandwidth = (size as f64 * 2.0) / cpu_elapsed / 1_000_000_000.0;

        let speedup = cpu_elapsed / gpu_elapsed;

        println!("| {:<7} | {:>6.1} µs | {:>7.1} GB/s | {:>6.2}× |",
                 label,
                 gpu_elapsed * 1_000_000.0,
                 gpu_bandwidth,
                 speedup);
    }

    println!();
    println!("Note: Unified memory means no CPU→GPU transfer overhead!");
    println!("Bandwidth measured includes dispatch overhead.");
}

fn test_block_processing(device: &Device, pipeline: &ComputePipelineState) {
    println!("Simulating bgzip block-based decompression pattern...");
    println!();

    // Simulate 485 blocks (like our Large bgzip file)
    let num_blocks = 485;
    let block_size = 12_000; // ~12 KB average per block

    // Create input data
    let total_size = num_blocks * block_size;
    let data = vec![0x42u8; total_size];

    // Create block metadata
    let mut block_offsets = Vec::with_capacity(num_blocks);
    let mut block_sizes = Vec::with_capacity(num_blocks);

    for i in 0..num_blocks {
        block_offsets.push((i * block_size) as u32);
        block_sizes.push(block_size as u32);
    }

    // Create Metal buffers
    let input_buffer = device.new_buffer_with_data(
        data.as_ptr() as *const _,
        data.len() as u64,
        MTLResourceOptions::StorageModeShared,
    );

    let output_buffer = device.new_buffer(
        data.len() as u64,
        MTLResourceOptions::StorageModeShared,
    );

    let offsets_buffer = device.new_buffer_with_data(
        block_offsets.as_ptr() as *const _,
        (block_offsets.len() * std::mem::size_of::<u32>()) as u64,
        MTLResourceOptions::StorageModeShared,
    );

    let sizes_buffer = device.new_buffer_with_data(
        block_sizes.as_ptr() as *const _,
        (block_sizes.len() * std::mem::size_of::<u32>()) as u64,
        MTLResourceOptions::StorageModeShared,
    );

    let command_queue = device.new_command_queue();

    // Warm-up
    {
        let command_buffer = command_queue.new_command_buffer();
        let encoder = command_buffer.new_compute_command_encoder();
        encoder.set_compute_pipeline_state(pipeline);
        encoder.set_buffer(0, Some(&input_buffer), 0);
        encoder.set_buffer(1, Some(&output_buffer), 0);
        encoder.set_buffer(2, Some(&offsets_buffer), 0);
        encoder.set_buffer(3, Some(&sizes_buffer), 0);

        let grid_size = MTLSize::new(num_blocks as u64, 1, 1);
        let threadgroup_size = MTLSize::new(256, 1, 1);
        encoder.dispatch_thread_groups(grid_size, threadgroup_size);
        encoder.end_encoding();

        command_buffer.commit();
        command_buffer.wait_until_completed();
    }

    // Benchmark (N=10)
    let iterations = 10;
    let start = Instant::now();

    for _ in 0..iterations {
        let command_buffer = command_queue.new_command_buffer();
        let encoder = command_buffer.new_compute_command_encoder();
        encoder.set_compute_pipeline_state(pipeline);
        encoder.set_buffer(0, Some(&input_buffer), 0);
        encoder.set_buffer(1, Some(&output_buffer), 0);
        encoder.set_buffer(2, Some(&offsets_buffer), 0);
        encoder.set_buffer(3, Some(&sizes_buffer), 0);

        let grid_size = MTLSize::new(num_blocks as u64, 1, 1);
        let threadgroup_size = MTLSize::new(256, 1, 1);
        encoder.dispatch_thread_groups(grid_size, threadgroup_size);
        encoder.end_encoding();

        command_buffer.commit();
        command_buffer.wait_until_completed();
    }

    let elapsed = start.elapsed().as_secs_f64() / iterations as f64;
    let throughput = (total_size as f64) / elapsed / 1_000_000.0; // MB/s

    println!("Results:");
    println!("  Blocks: {}", num_blocks);
    println!("  Total size: {:.2} MB", total_size as f64 / 1_000_000.0);
    println!("  GPU time: {:.3} ms", elapsed * 1000.0);
    println!("  Throughput: {:.1} MB/s", throughput);
    println!("  Per-block time: {:.1} µs", (elapsed * 1_000_000.0) / num_blocks as f64);
    println!();

    println!("Comparison to CPU parallel (from bgzip benchmark):");
    println!("  CPU parallel: 4668.85 MB/s (485 blocks, 6.5× vs sequential)");
    println!("  GPU (trivial copy): {:.1} MB/s", throughput);
    println!();

    if throughput > 4600.0 {
        println!("✅ GPU matches or exceeds CPU parallel on trivial workload!");
        println!("   DEFLATE implementation has potential for speedup.");
    } else {
        println!("⚠️  GPU slower than CPU parallel on trivial workload.");
        println!("   DEFLATE overhead may make GPU uncompetitive.");
    }
}
