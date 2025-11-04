#include <metal_stdlib>
using namespace metal;

/// Phase 2: Minimal DEFLATE Implementation
///
/// Implements:
/// - Bit-stream reader (LSB-first, DEFLATE format)
/// - Fixed Huffman decoding (BTYPE=01)
/// - Literal-only output (no LZ77 for Phase 2)
///
/// Purpose: Measure DEFLATE overhead vs trivial copy

// ============================================================================
// Bit-Stream Reader
// ============================================================================

struct BitStream {
    const device uint8_t* data;
    uint byte_offset;
    uint bit_offset;  // 0-7, position within current byte
    uint total_bits;  // For bounds checking
};

/// Initialize bit stream
inline void init_bitstream(thread BitStream* bs, const device uint8_t* data, uint size_bytes) {
    bs->data = data;
    bs->byte_offset = 0;
    bs->bit_offset = 0;
    bs->total_bits = size_bytes * 8;
}

/// Read N bits from stream (up to 16 bits)
/// DEFLATE uses LSB-first bit order within each byte
inline uint16_t read_bits(thread BitStream* bs, uint n) {
    uint16_t result = 0;

    for (uint i = 0; i < n; i++) {
        // Check bounds
        if (bs->byte_offset * 8 + bs->bit_offset >= bs->total_bits) {
            return 0xFFFF;  // Error sentinel
        }

        // Read one bit (LSB first within byte)
        uint8_t byte_val = bs->data[bs->byte_offset];
        uint bit = (byte_val >> bs->bit_offset) & 1;

        // Append to result (also LSB first)
        result |= (bit << i);

        // Advance bit position
        bs->bit_offset++;
        if (bs->bit_offset == 8) {
            bs->bit_offset = 0;
            bs->byte_offset++;
        }
    }

    return result;
}

/// Peek N bits without advancing stream
inline uint16_t peek_bits(thread const BitStream* bs, uint n) {
    // Create temporary copy
    BitStream temp = *bs;
    return read_bits(&temp, n);
}

/// Skip N bits
inline void skip_bits(thread BitStream* bs, uint n) {
    for (uint i = 0; i < n; i++) {
        bs->bit_offset++;
        if (bs->bit_offset == 8) {
            bs->bit_offset = 0;
            bs->byte_offset++;
        }
    }
}

/// Align to byte boundary
inline void align_to_byte(thread BitStream* bs) {
    if (bs->bit_offset != 0) {
        bs->bit_offset = 0;
        bs->byte_offset++;
    }
}

// ============================================================================
// Fixed Huffman Tables (DEFLATE spec RFC 1951)
// ============================================================================

/// Fixed Huffman code lengths for literal/length alphabet
/// Lit values 0-143: 8 bits
/// Lit values 144-255: 9 bits
/// Len values 256-279: 7 bits
/// Len values 280-287: 8 bits
constant uint8_t FIXED_LIT_LENGTHS[288] = {
    8,8,8,8,8,8,8,8,8,8,8,8,8,8,8,8,  // 0-15
    8,8,8,8,8,8,8,8,8,8,8,8,8,8,8,8,  // 16-31
    8,8,8,8,8,8,8,8,8,8,8,8,8,8,8,8,  // 32-47
    8,8,8,8,8,8,8,8,8,8,8,8,8,8,8,8,  // 48-63
    8,8,8,8,8,8,8,8,8,8,8,8,8,8,8,8,  // 64-79
    8,8,8,8,8,8,8,8,8,8,8,8,8,8,8,8,  // 80-95
    8,8,8,8,8,8,8,8,8,8,8,8,8,8,8,8,  // 96-111
    8,8,8,8,8,8,8,8,8,8,8,8,8,8,8,8,  // 112-127
    8,8,8,8,8,8,8,8,8,8,8,8,8,8,8,8,  // 128-143
    9,9,9,9,9,9,9,9,9,9,9,9,9,9,9,9,  // 144-159
    9,9,9,9,9,9,9,9,9,9,9,9,9,9,9,9,  // 160-175
    9,9,9,9,9,9,9,9,9,9,9,9,9,9,9,9,  // 176-191
    9,9,9,9,9,9,9,9,9,9,9,9,9,9,9,9,  // 192-207
    9,9,9,9,9,9,9,9,9,9,9,9,9,9,9,9,  // 208-223
    9,9,9,9,9,9,9,9,9,9,9,9,9,9,9,9,  // 224-239
    9,9,9,9,9,9,9,9,9,9,9,9,9,9,9,9,  // 240-255
    7,7,7,7,7,7,7,7,7,7,7,7,7,7,7,7,  // 256-271
    7,7,7,7,7,7,7,7,8,8,8,8,8,8,8,8   // 272-287
};

/// Decode one symbol using fixed Huffman codes
/// Returns symbol value (0-287) or 0xFFFF on error
inline uint16_t decode_fixed_huffman(thread BitStream* bs) {
    // Fixed Huffman decoding based on code lengths:
    // - 7-bit codes: 256-279 (0000000-0010111)
    // - 8-bit codes: 0-143, 280-287 (00110000-10111111, 11000000-11000111)
    // - 9-bit codes: 144-255 (110010000-111111111)

    // Peek up to 9 bits
    uint16_t bits = peek_bits(bs, 9);

    // Try 7-bit codes first (256-279)
    // Codes: 0000000 (0) to 0010111 (23)
    uint16_t code_7 = bits & 0x7F;  // Bottom 7 bits
    if (code_7 <= 23) {
        skip_bits(bs, 7);
        return 256 + code_7;
    }

    // Try 8-bit codes (0-143, 280-287)
    uint16_t code_8 = bits & 0xFF;  // Bottom 8 bits

    // Range 00110000 (48) to 10111111 (191): symbols 0-143
    if (code_8 >= 48 && code_8 <= 191) {
        skip_bits(bs, 8);
        return code_8 - 48;
    }

    // Range 11000000 (192) to 11000111 (199): symbols 280-287
    if (code_8 >= 192 && code_8 <= 199) {
        skip_bits(bs, 8);
        return 280 + (code_8 - 192);
    }

    // Try 9-bit codes (144-255)
    // Range 110010000 (400) to 111111111 (511): symbols 144-255
    if (bits >= 400 && bits <= 511) {
        skip_bits(bs, 9);
        return 144 + (bits - 400);
    }

    // Invalid code
    return 0xFFFF;
}

// ============================================================================
// Length/Distance Decoding (Stub for Phase 2)
// ============================================================================

/// Decode extra length bits (for symbols 257-285)
/// Phase 2: Just skip these, don't process LZ77
inline uint16_t decode_length(thread BitStream* bs, uint16_t symbol) {
    // Symbol 257-264: length 3-10 (no extra bits)
    if (symbol <= 264) {
        return 3 + (symbol - 257);
    }

    // Symbol 265-284: lengths with extra bits
    // For Phase 2, just return approximate length
    // We won't use this anyway (skipping LZ77)
    return 11;  // Dummy value
}

/// Decode distance code (stub for Phase 2)
inline uint16_t decode_distance(thread BitStream* bs) {
    // Distance codes are 5 bits in fixed Huffman
    uint16_t code = read_bits(bs, 5);

    // Skip any extra bits (depends on code value)
    // For Phase 2, we don't care about actual distance
    return 1;  // Dummy value
}

// ============================================================================
// Block Decoder
// ============================================================================

/// Decode one DEFLATE block (fixed Huffman only, literals only)
kernel void deflate_decode_block(
    const device uint8_t* input [[buffer(0)]],
    device uint8_t* output [[buffer(1)]],
    device uint32_t* block_offsets [[buffer(2)]],
    device uint32_t* block_sizes [[buffer(3)]],
    device uint32_t* output_sizes [[buffer(4)]],  // Actual decompressed sizes
    uint block_id [[threadgroup_position_in_grid]],
    uint thread_id [[thread_position_in_threadgroup]],
    threadgroup uint8_t* temp_buffer [[threadgroup(0)]]  // 32 KB shared memory
) {
    // Only thread 0 does the work (sequential for Phase 2)
    if (thread_id != 0) return;

    // Initialize bit stream for this block
    BitStream bs;
    uint block_offset = block_offsets[block_id];
    uint block_size = block_sizes[block_id];
    init_bitstream(&bs, input + block_offset, block_size);

    // Parse block header (3 bits)
    uint bfinal = read_bits(&bs, 1);
    uint btype = read_bits(&bs, 2);

    // Only handle fixed Huffman (btype = 01)
    if (btype != 1) {
        // Unsupported block type for Phase 2
        output_sizes[block_id] = 0;
        return;
    }

    // Decode symbols until end-of-block (256)
    uint output_pos = 0;
    const uint max_output = 65536;  // Max uncompressed block size

    while (output_pos < max_output) {
        uint16_t symbol = decode_fixed_huffman(&bs);

        // Check for error
        if (symbol == 0xFFFF) {
            break;  // Decoding error
        }

        // End of block marker
        if (symbol == 256) {
            break;
        }

        // Literal byte (0-255)
        if (symbol < 256) {
            temp_buffer[output_pos++] = (uint8_t)symbol;
        }
        // Length code (257-285) - LZ77 reference
        else if (symbol >= 257 && symbol <= 285) {
            // Phase 2: Skip LZ77 processing
            // Just decode and discard length/distance
            uint16_t length = decode_length(&bs, symbol);
            uint16_t distance = decode_distance(&bs);

            // Don't actually copy anything (no LZ77 in Phase 2)
            // This means our output will be incomplete, but we can
            // still measure decoding overhead
        }
    }

    // Write output size
    output_sizes[block_id] = output_pos;

    // Copy temp buffer to output
    uint output_offset = block_offsets[block_id];
    for (uint i = 0; i < output_pos; i++) {
        output[output_offset + i] = temp_buffer[i];
    }
}

// ============================================================================
// Test Kernels (for development/debugging)
// ============================================================================

/// Test bit-stream reading
kernel void test_bitstream(
    const device uint8_t* input [[buffer(0)]],
    device uint16_t* output [[buffer(1)]],
    uint id [[thread_position_in_grid]]
) {
    if (id != 0) return;

    BitStream bs;
    init_bitstream(&bs, input, 100);

    // Test reading various bit lengths
    output[0] = read_bits(&bs, 1);   // 1 bit
    output[1] = read_bits(&bs, 3);   // 3 bits
    output[2] = read_bits(&bs, 8);   // 8 bits
    output[3] = read_bits(&bs, 16);  // 16 bits
}

/// Test Huffman decoding with known pattern
kernel void test_huffman(
    const device uint8_t* input [[buffer(0)]],
    device uint16_t* output [[buffer(1)]],
    uint id [[thread_position_in_grid]]
) {
    if (id != 0) return;

    BitStream bs;
    init_bitstream(&bs, input, 100);

    // Decode first 10 symbols
    for (uint i = 0; i < 10; i++) {
        output[i] = decode_fixed_huffman(&bs);
    }
}
