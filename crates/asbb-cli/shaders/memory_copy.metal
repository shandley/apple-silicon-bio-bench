#include <metal_stdlib>
using namespace metal;

/// Phase 1 Feasibility Test: Trivial Memory Copy Kernel
///
/// This shader simply copies data from input buffer to output buffer.
/// Purpose: Measure GPU dispatch overhead and memory bandwidth baseline.

kernel void memory_copy(
    const device uint8_t* input [[buffer(0)]],
    device uint8_t* output [[buffer(1)]],
    constant uint32_t& size [[buffer(2)]],
    uint id [[thread_position_in_grid]]
) {
    // Each thread copies one byte
    if (id < size) {
        output[id] = input[id];
    }
}

/// Block-based memory copy (more realistic for bgzip)
/// Each thread copies a chunk of bytes
kernel void memory_copy_blocks(
    const device uint8_t* input [[buffer(0)]],
    device uint8_t* output [[buffer(1)]],
    constant uint32_t* block_offsets [[buffer(2)]],
    constant uint32_t* block_sizes [[buffer(3)]],
    uint block_id [[threadgroup_position_in_grid]],
    uint thread_id [[thread_position_in_threadgroup]]
) {
    // Each threadgroup handles one block
    uint offset = block_offsets[block_id];
    uint size = block_sizes[block_id];

    // Simple parallel copy within block
    // Each thread copies 4 bytes (coalesced memory access)
    const uint stride = 256; // threads per threadgroup
    const uint bytes_per_thread = 4;

    for (uint i = thread_id * bytes_per_thread; i < size; i += stride * bytes_per_thread) {
        if (i + 3 < size) {
            // Copy 4 bytes at once (aligned)
            *((device uint32_t*)(output + offset + i)) =
                *((const device uint32_t*)(input + offset + i));
        } else {
            // Handle remainder bytes
            for (uint j = 0; j < bytes_per_thread && i + j < size; j++) {
                output[offset + i + j] = input[offset + i + j];
            }
        }
    }
}
