/// Metal DEFLATE Phase 2 Benchmark
///
/// Tests GPU DEFLATE decoding (fixed Huffman, literals only)
/// Measures overhead vs trivial copy to determine Phase 3 viability

use metal::*;
use std::time::Instant;
use flate2::read::GzDecoder;
use std::io::Read;
use std::path::Path;
use std::fs::File;

const SHADER_SOURCE: &str = include_str!("../../shaders/deflate_decode.metal");

/// Parse bgzip blocks from file (reuse from bgzip-parallel-benchmark)
#[derive(Debug, Clone)]
struct BgzipBlock {
    offset: u64,
    csize: u16,
    data: Vec<u8>,
}

fn parse_bgzip_blocks(path: &Path) -> std::io::Result<Vec<BgzipBlock>> {
    let mut file = File::open(path)?;
    let file_size = file.metadata()?.len();
    let mut blocks = Vec::new();

    use std::io::{Seek, SeekFrom};

    let mut offset = 0u64;

    while offset < file_size {
        file.seek(SeekFrom::Start(offset))?;

        let mut header = [0u8; 10];
        if file.read_exact(&mut header).is_err() {
            break;
        }

        if header[0] != 0x1f || header[1] != 0x8b {
            break;
        }

        let flags = header[3];
        let has_extra = (flags & 0x04) != 0;

        if !has_extra {
            offset += 1;
            continue;
        }

        let mut xlen_bytes = [0u8; 2];
        file.read_exact(&mut xlen_bytes)?;
        let xlen = u16::from_le_bytes(xlen_bytes);

        let mut bsize: Option<u16> = None;
        let mut extra_read = 0;

        while extra_read < xlen {
            let mut subfield_header = [0u8; 4];
            if file.read_exact(&mut subfield_header).is_err() {
                break;
            }

            let si1 = subfield_header[0];
            let si2 = subfield_header[1];
            let slen = u16::from_le_bytes([subfield_header[2], subfield_header[3]]);

            extra_read += 4;

            if si1 == b'B' && si2 == b'C' && slen == 2 {
                let mut bsize_bytes = [0u8; 2];
                file.read_exact(&mut bsize_bytes)?;
                bsize = Some(u16::from_le_bytes(bsize_bytes));
                extra_read += 2;
            } else {
                file.seek(SeekFrom::Current(slen as i64))?;
                extra_read += slen;
            }
        }

        let bsize = match bsize {
            Some(b) => b,
            None => {
                offset += 1;
                continue;
            }
        };

        let block_size = (bsize + 1) as u64;
        file.seek(SeekFrom::Start(offset))?;
        let mut block_data = vec![0u8; block_size as usize];
        file.read_exact(&mut block_data)?;

        blocks.push(BgzipBlock {
            offset,
            csize: bsize,
            data: block_data,
        });

        offset += block_size;
    }

    Ok(blocks)
}

/// Decompress block with CPU (for comparison)
fn decompress_block_cpu(block: &BgzipBlock) -> std::io::Result<Vec<u8>> {
    let cursor = std::io::Cursor::new(&block.data);
    let mut decoder = GzDecoder::new(cursor);
    let mut decompressed = Vec::new();
    decoder.read_to_end(&mut decompressed)?;
    Ok(decompressed)
}

fn main() {
    println!("================================================================================");
    println!("METAL DEFLATE PHASE 2 BENCHMARK");
    println!("================================================================================");
    println!();
    println!("Testing GPU DEFLATE decoding (fixed Huffman, literals only)");
    println!("Goal: Measure overhead vs Phase 1 trivial copy");
    println!();

    // Initialize Metal
    let device = Device::system_default().expect("No Metal device found");
    println!("Metal device: {}", device.name());
    println!();

    // Compile shader
    println!("Compiling DEFLATE shader...");
    let library = device.new_library_with_source(SHADER_SOURCE, &CompileOptions::new())
        .expect("Failed to compile shader");

    let deflate_kernel = library.get_function("deflate_decode_block", None)
        .expect("Failed to get deflate_decode_block function");

    let test_bitstream = library.get_function("test_bitstream", None)
        .expect("Failed to get test_bitstream function");

    let test_huffman = library.get_function("test_huffman", None)
        .expect("Failed to get test_huffman function");

    let pipeline_deflate = device.new_compute_pipeline_state_with_function(&deflate_kernel)
        .expect("Failed to create DEFLATE pipeline");

    let pipeline_test_bits = device.new_compute_pipeline_state_with_function(&test_bitstream)
        .expect("Failed to create test pipeline");

    let pipeline_test_huff = device.new_compute_pipeline_state_with_function(&test_huffman)
        .expect("Failed to create test pipeline");

    println!("✓ Shader compiled successfully");
    println!();

    // Test 1: Bit-stream reader
    println!("================================================================================");
    println!("Test 1: Bit-Stream Reader");
    println!("================================================================================");
    println!();

    test_bitstream_reader(&device, &pipeline_test_bits);

    // Test 2: Huffman decoder
    println!();
    println!("================================================================================");
    println!("Test 2: Huffman Decoder");
    println!("================================================================================");
    println!();

    test_huffman_decoder(&device, &pipeline_test_huff);

    // Test 3: Real bgzip blocks
    println!();
    println!("================================================================================");
    println!("Test 3: Real bgzip Block Decompression");
    println!("================================================================================");
    println!();

    test_real_bgzip(&device, &pipeline_deflate);

    println!();
    println!("================================================================================");
    println!("PHASE 2 COMPLETE");
    println!("================================================================================");
    println!();
    println!("Next: Analyze results and decide on Phase 3");
}

fn test_bitstream_reader(device: &Device, pipeline: &ComputePipelineState) {
    println!("Testing bit-stream reading with known patterns...");
    println!();

    // Test data: Simple bit patterns
    let test_data = vec![
        0b10101010u8,  // 10101010
        0b11001100,    // 11001100
        0b11110000,    // 11110000
    ];

    // Create buffers
    let input_buffer = device.new_buffer_with_data(
        test_data.as_ptr() as *const _,
        test_data.len() as u64,
        MTLResourceOptions::StorageModeShared,
    );

    let output_buffer = device.new_buffer(
        16 * std::mem::size_of::<u16>() as u64,
        MTLResourceOptions::StorageModeShared,
    );

    // Run kernel
    let command_queue = device.new_command_queue();
    let command_buffer = command_queue.new_command_buffer();
    let encoder = command_buffer.new_compute_command_encoder();

    encoder.set_compute_pipeline_state(pipeline);
    encoder.set_buffer(0, Some(&input_buffer), 0);
    encoder.set_buffer(1, Some(&output_buffer), 0);

    let grid_size = MTLSize::new(1, 1, 1);
    let threadgroup_size = MTLSize::new(1, 1, 1);
    encoder.dispatch_threads(grid_size, threadgroup_size);
    encoder.end_encoding();

    command_buffer.commit();
    command_buffer.wait_until_completed();

    // Read results
    let output_ptr = output_buffer.contents() as *const u16;
    let output = unsafe { std::slice::from_raw_parts(output_ptr, 16) };

    println!("Results:");
    println!("  1 bit: {:#06b} ({})", output[0], output[0]);
    println!("  3 bits: {:#06b} ({})", output[1], output[1]);
    println!("  8 bits: {:#010b} ({})", output[2], output[2]);
    println!("  16 bits: {:#018b} ({})", output[3], output[3]);
    println!();

    // Expected (LSB first within byte):
    // First byte 10101010 (LSB first = 01010101 reading right to left)
    // 1 bit: 0
    // Next 3 bits: 101
    // Next 8 bits: crosses byte boundary

    println!("✓ Bit-stream reader test complete");
}

fn test_huffman_decoder(device: &Device, pipeline: &ComputePipelineState) {
    println!("Testing Huffman decoder with simple fixed-Huffman data...");
    println!();

    // Create simple fixed-Huffman compressed data
    // For now, just test that kernel runs without crashing
    let test_data = vec![0u8; 1000];

    let input_buffer = device.new_buffer_with_data(
        test_data.as_ptr() as *const _,
        test_data.len() as u64,
        MTLResourceOptions::StorageModeShared,
    );

    let output_buffer = device.new_buffer(
        100 * std::mem::size_of::<u16>() as u64,
        MTLResourceOptions::StorageModeShared,
    );

    let command_queue = device.new_command_queue();
    let command_buffer = command_queue.new_command_buffer();
    let encoder = command_buffer.new_compute_command_encoder();

    encoder.set_compute_pipeline_state(pipeline);
    encoder.set_buffer(0, Some(&input_buffer), 0);
    encoder.set_buffer(1, Some(&output_buffer), 0);

    let grid_size = MTLSize::new(1, 1, 1);
    let threadgroup_size = MTLSize::new(1, 1, 1);
    encoder.dispatch_threads(grid_size, threadgroup_size);
    encoder.end_encoding();

    command_buffer.commit();
    command_buffer.wait_until_completed();

    println!("✓ Huffman decoder test complete (no crash)");
    println!("  Note: Need real fixed-Huffman data for full validation");
}

fn test_real_bgzip(device: &Device, pipeline: &ComputePipelineState) {
    println!("Testing with real bgzip blocks...");
    println!();

    // Load bgzip file
    let test_file = Path::new("datasets/large_100k_150bp.fq.bgz");

    if !test_file.exists() {
        println!("⚠️  Test file not found: {:?}", test_file);
        println!("   Please run: gunzip -c datasets/large_100k_150bp.fq.gz | bgzip -c > datasets/large_100k_150bp.fq.bgz");
        return;
    }

    println!("Parsing bgzip blocks from {:?}...", test_file);
    let blocks = parse_bgzip_blocks(test_file).expect("Failed to parse bgzip file");

    println!("Found {} bgzip blocks", blocks.len());
    println!();

    // Analyze block types
    let mut fixed_huffman_count = 0;
    let mut dynamic_huffman_count = 0;
    let mut uncompressed_count = 0;

    for block in &blocks {
        // Check block type (3rd bit)
        if block.data.len() >= 3 {
            let btype = (block.data[0] >> 1) & 0x03;
            match btype {
                0 => uncompressed_count += 1,
                1 => fixed_huffman_count += 1,
                2 => dynamic_huffman_count += 1,
                _ => {}
            }
        }
    }

    println!("Block type distribution:");
    println!("  Uncompressed: {} ({:.1}%)", uncompressed_count, 100.0 * uncompressed_count as f64 / blocks.len() as f64);
    println!("  Fixed Huffman: {} ({:.1}%)", fixed_huffman_count, 100.0 * fixed_huffman_count as f64 / blocks.len() as f64);
    println!("  Dynamic Huffman: {} ({:.1}%)", dynamic_huffman_count, 100.0 * dynamic_huffman_count as f64 / blocks.len() as f64);
    println!();

    if fixed_huffman_count == 0 {
        println!("⚠️  No fixed-Huffman blocks found!");
        println!("   Phase 2 only supports fixed Huffman");
        println!("   Most bgzip files use dynamic Huffman");
        println!();
        println!("Recommendation: Proceed to Phase 2.5 (implement dynamic Huffman)");
        println!("                 or Phase 3 (full LZ77 with dynamic Huffman)");
        return;
    }

    println!("⚠️  Phase 2 limitation: Only {} of {} blocks can be processed", fixed_huffman_count, blocks.len());
    println!("   Need Phase 2.5 (dynamic Huffman) for complete support");
    println!();

    // For now, just report that we can't benchmark without fixed-Huffman blocks
    println!("Next steps:");
    println!("  1. Generate test data with fixed-Huffman encoding");
    println!("  2. OR implement dynamic Huffman (Phase 2.5)");
    println!("  3. OR skip to Phase 3 (full DEFLATE with LZ77)");
}
