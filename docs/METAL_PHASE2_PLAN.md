# Metal Phase 2: Minimal DEFLATE Implementation Plan

**Date**: November 4, 2025
**Goal**: Measure DEFLATE overhead to determine if GPU implementation is viable
**Time-box**: 2-3 days maximum
**Exit criteria**: If per-block time >5 µs, stop (overhead too high)

---

## Phase 2 Scope (Minimal)

### What We're Implementing

**Huffman decoding ONLY** - no LZ77 yet

```
DEFLATE full:
  Block header → Huffman tables → Decode symbols → LZ77 expansion → Output

Phase 2 (minimal):
  Block header → Huffman tables → Decode symbols (literals only) → Output
                                                   ^^^^^^^^^^^^^^
                                                   Skip LZ77 for now
```

**Why skip LZ77?**
- LZ77 is the complex part (backward references, sequential writes)
- Huffman decoding is embarrassingly parallel (good for GPU)
- If Huffman alone has high overhead, full DEFLATE will be worse
- Faster to implement (2-3 days vs 5-7 days)

---

## DEFLATE Format Quick Reference

### Block Structure

```
Block:
  [Header: 3 bits]
    BFINAL: 1 bit (is this the last block?)
    BTYPE: 2 bits (compression type)
      00 = uncompressed
      01 = fixed Huffman
      10 = dynamic Huffman
      11 = reserved (error)

  [Huffman tables] (if BTYPE = 10)
    Code lengths for literal/length alphabet
    Code lengths for distance alphabet

  [Compressed data]
    Huffman-coded symbols:
      0-255: Literal bytes
      256: End of block
      257-285: Length codes (for LZ77)
```

### Bit-Stream Reading

**DEFLATE is bit-packed, LSB first**:
```
Byte stream: [0xAB, 0xCD, ...]
Bit stream:  11010101 10110011 ... (LSB first within each byte)
             ^
             Read from right to left within byte
```

**Example**: Read 3 bits from 0xAB (10101011)
```
Bits: 011 (read LSB first)
Value: 3
```

### Fixed Huffman Codes (BTYPE=01)

**Literal/length codes**:
```
  0-143: 8 bits (00110000 - 10111111)
144-255: 9 bits (110010000 - 111111111)
256-279: 7 bits (0000000 - 0010111)
280-287: 8 bits (11000000 - 11000111)
```

**Distance codes**: All 5 bits (00000-11111)

### Dynamic Huffman Codes (BTYPE=10)

**Header** (14 bits):
```
HLIT: 5 bits (# of literal/length codes - 257)
HDIST: 5 bits (# of distance codes - 1)
HCLEN: 4 bits (# of code length codes - 4)
```

**Code lengths**: Encoded using a small Huffman table
- 19 possible code length symbols (0-18)
- Ordered by frequency (not sequential)

**Too complex for Phase 2** - start with fixed Huffman only!

---

## Implementation Strategy

### Step 1: Bit-Stream Reader (Day 1 Morning)

**Goal**: Read arbitrary bits from byte array

```metal
struct BitStream {
    device const uint8_t* data;
    uint byte_offset;
    uint bit_offset;  // 0-7
};

// Read N bits (up to 16 bits at a time)
uint16_t read_bits(thread BitStream* bs, uint n) {
    // Handle bit offset within current byte
    // May span multiple bytes
    // Return value in LSB-first order
}
```

**Test**: Read known bit patterns, verify correct values

**Time estimate**: 2-3 hours

### Step 2: Fixed Huffman Tables (Day 1 Afternoon)

**Goal**: Build fixed Huffman decode tables

```metal
struct HuffmanTable {
    uint16_t symbols[512];  // Symbol for each code
    uint8_t lengths[512];   // Bit length for each code
};

void build_fixed_huffman(thread HuffmanTable* lit_table) {
    // Literal/length codes (see spec)
    // Distance codes (all 5 bits)
}
```

**Approach**: Pre-compute tables (they're fixed!)
- Can generate at compile time
- Store as constants in shader

**Time estimate**: 2-3 hours

### Step 3: Huffman Decoder (Day 1 Evening)

**Goal**: Decode one symbol from bit stream

```metal
uint16_t huffman_decode(
    thread BitStream* bs,
    thread const HuffmanTable* table
) {
    // Read bits one at a time
    // Match against Huffman codes
    // Return decoded symbol
}
```

**Algorithm**:
1. Read 1 bit, check if it matches 1-bit code
2. Read 2nd bit, check if it matches 2-bit code
3. Continue until match found
4. Return corresponding symbol

**Time estimate**: 3-4 hours

### Step 4: Block Decoder (Day 2 Morning)

**Goal**: Decode entire block (literals only)

```metal
kernel void deflate_decode(
    device const uint8_t* input [[buffer(0)]],
    device uint8_t* output [[buffer(1)]],
    device const uint32_t* block_offsets [[buffer(2)]],
    device const uint32_t* block_sizes [[buffer(3)]],
    uint block_id [[threadgroup_position_in_grid]],
    threadgroup uint8_t* temp_buffer [[threadgroup(0)]]
) {
    // Each threadgroup = one block

    // 1. Parse block header (3 bits)
    BitStream bs;
    bs.data = input + block_offsets[block_id];

    uint bfinal = read_bits(&bs, 1);
    uint btype = read_bits(&bs, 2);

    // 2. Only handle fixed Huffman for now
    if (btype != 1) return;  // Skip non-fixed blocks

    // 3. Build Huffman table
    HuffmanTable lit_table;
    build_fixed_huffman(&lit_table);

    // 4. Decode symbols (parallel within threadgroup)
    uint thread_id = [[thread_position_in_threadgroup]];

    // Simple approach: Thread 0 decodes sequentially
    // (Parallelization is for later optimization)
    if (thread_id == 0) {
        uint output_pos = 0;
        while (true) {
            uint16_t symbol = huffman_decode(&bs, &lit_table);

            if (symbol == 256) break;  // End of block

            if (symbol < 256) {
                // Literal byte
                temp_buffer[output_pos++] = (uint8_t)symbol;
            } else {
                // Length code (257-285)
                // Skip LZ77 for Phase 2
                // Just ignore these symbols
            }
        }

        // Copy to output
        for (uint i = 0; i < output_pos; i++) {
            output[block_offsets[block_id] + i] = temp_buffer[i];
        }
    }
}
```

**Note**: This is SEQUENTIAL within each block (not optimal)
- Goal is to MEASURE overhead, not optimize yet
- Parallelization can come in Phase 3

**Time estimate**: 4-5 hours

### Step 5: Testing and Benchmarking (Day 2 Afternoon)

**Test data preparation**:
1. Create fixed-Huffman compressed test data
2. Decompress with CPU (flate2 crate)
3. Decompress with GPU
4. Compare outputs byte-by-byte

**Benchmark**:
1. Measure per-block time (average of 485 blocks)
2. Compare to trivial copy baseline (0.9 µs/block)
3. Calculate overhead multiplier

**Time estimate**: 3-4 hours

---

## Success Criteria

### Phase 2 Success

**Goal**: Per-block DEFLATE time <5 µs

**If achieved**:
- Overhead = 5 µs / 0.9 µs = 5.6× vs trivial
- Still faster than CPU: 2.86× / 5.6× = 0.51× (wait, that's slower!)

**Actually, let me recalculate**:
```
CPU parallel: 4,669 MB/s = ~214 µs total for 485 blocks = 0.44 µs/block
GPU trivial: 13,372 MB/s = ~435 µs total for 485 blocks = 0.9 µs/block

If GPU DEFLATE = 5 µs/block:
  Total time = 485 × 5 µs = 2,425 µs = 2.43 ms
  CPU parallel = 485 × 0.44 µs × (overhead) = needs measurement

Hmm, I need to measure CPU per-block time too.
```

**Better criteria**:
- Measure GPU DEFLATE throughput (MB/s)
- Compare to CPU parallel (4,669 MB/s)
- **Success**: GPU >2,000 MB/s (>0.5× CPU parallel)

**Why 0.5× is acceptable**:
- Phase 2 is NOT optimized (sequential within block)
- Phase 3 will parallelize within block
- Potential 2-4× speedup from parallelization

### Phase 2 Failure

**If GPU <2,000 MB/s**:
- DEFLATE overhead too high
- Unlikely Phase 3 optimization will help enough
- **Stop and use CPU parallel implementation**

---

## Risk Mitigation

### Risk 1: DEFLATE Complexity

**Risk**: Bit-stream operations are complex, may have bugs

**Mitigation**:
- Test bit-stream reader extensively
- Compare outputs to known-good decompression
- Use small test cases first

### Risk 2: Fixed Huffman Limitation

**Risk**: Most bgzip blocks use dynamic Huffman (more complex)

**Mitigation**:
- Measure distribution of block types in real bgzip files
- If >90% are dynamic, fixed-only won't help
- Consider implementing dynamic Huffman in Phase 2.5

### Risk 3: Sequential Decoding

**Risk**: Sequential decoding within block won't show GPU advantage

**Mitigation**:
- Accept this for Phase 2 (measurement phase)
- Phase 3 will parallelize if Phase 2 succeeds
- Focus on measuring inherent DEFLATE cost

---

## Timeline

### Day 1 (Nov 4, Afternoon/Evening)

**Morning** (already done):
- ✅ Phase 1 feasibility test
- ✅ Results analysis

**Afternoon** (3-4 hours):
- [ ] Implement bit-stream reader
- [ ] Test bit-stream reader
- [ ] Implement fixed Huffman table builder

**Evening** (2-3 hours):
- [ ] Implement Huffman decoder
- [ ] Unit test decoder with simple cases

**Status check**: If bit-stream + Huffman work, continue to Day 2

### Day 2 (Nov 5)

**Morning** (4-5 hours):
- [ ] Implement block decoder (full pipeline)
- [ ] Create test data (fixed Huffman blocks)
- [ ] Test GPU vs CPU output (correctness)

**Afternoon** (3-4 hours):
- [ ] Benchmark per-block time
- [ ] Calculate overhead vs trivial
- [ ] Compare to CPU parallel

**Decision point**:
- If GPU >2,000 MB/s: Proceed to Phase 3
- If GPU <2,000 MB/s: Stop, document findings

### Day 3 (Nov 6, if needed)

**Buffer day for debugging/optimization**:
- Fix bugs
- Profile with Metal System Trace
- Optimize hot paths

**Backup plan**: If Day 2 doesn't finish, use Day 3 to complete

---

## Deliverables

### Code

1. **Metal shader**: `crates/asbb-cli/shaders/deflate_decode.metal`
   - Bit-stream reader
   - Fixed Huffman tables
   - Symbol decoder
   - Block decoder

2. **Rust benchmark**: `crates/asbb-cli/src/bin/metal-deflate-benchmark.rs`
   - GPU dispatch
   - Test data generation
   - Correctness verification
   - Performance measurement

### Documentation

3. **Results report**: `results/bgzip_parallel/METAL_PHASE2_RESULTS.md`
   - Per-block time measurements
   - Overhead calculation
   - Comparison to CPU parallel
   - Decision for Phase 3

---

## Phase 3 Preview (If Phase 2 Succeeds)

### Phase 3 Scope

1. **Add LZ77 support**: Backward references, sliding window
2. **Parallelize within block**: Multiple threads decode chunks
3. **Dynamic Huffman**: Support all DEFLATE block types
4. **Optimize**: Reduce register usage, improve occupancy
5. **Production integration**: Add to biofast library

**Time**: 5-7 days
**Goal**: GPU >1.5× faster than CPU parallel

---

## Current Status

**Phase 1**: ✅ Complete
- GPU trivial copy: 13,372 MB/s (2.86× vs CPU parallel)
- Dispatch overhead: 272 µs (acceptable with batch)

**Phase 2**: 🚀 Starting now
- Day 1 afternoon: Bit-stream + Huffman
- Day 2: Block decoder + benchmarking
- Decision: Proceed to Phase 3 or stop

**Next step**: Implement bit-stream reader in Metal

---

**Document Author**: Claude
**Date**: November 4, 2025
**Status**: Phase 2 implementation starting
**Time-box**: 2-3 days (stop if >5 µs/block or <2,000 MB/s)
