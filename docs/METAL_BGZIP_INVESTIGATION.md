# Metal GPU Bgzip Decompression Investigation

**Date**: November 4, 2025
**Goal**: Investigate Metal GPU implementation for 10-40× bgzip decompression speedup
**Platform**: Apple M4 Max (40 GPU cores, 15.8 TFLOPS, 546 GB/s memory bandwidth)

---

## Investigation Plan

### Phase 1: Feasibility Analysis (Current)

**Questions to answer**:
1. Can DEFLATE be implemented in Metal Shading Language (MSL)?
2. What are the memory access patterns (compatible with GPU)?
3. Is block-level parallelism sufficient (vs intra-block)?
4. What's the overhead of GPU dispatch?

### Phase 2: Prototype Implementation

**If feasible**:
1. Write minimal Metal compute shader for DEFLATE
2. Implement Rust wrapper (metal-rs crate)
3. Benchmark single block (GPU vs CPU)
4. Benchmark N blocks (measure dispatch overhead)

### Phase 3: Optimization

**If prototype shows promise**:
1. Optimize shader (reduce registers, improve occupancy)
2. Implement streaming (overlap compute + memory)
3. Profile with Metal System Trace
4. Compare to CPU parallel implementation

---

## Technical Challenges

### Challenge 1: DEFLATE Algorithm Complexity

**DEFLATE structure**:
```
Compressed data = [Block 1] [Block 2] ... [Block N]

Each block:
- Header: 3 bits (BFINAL, BTYPE)
- Data: Huffman-coded literals and length/distance pairs
- References: Backward pointers (LZ77 compression)
```

**GPU challenges**:
- **Bit-level operations**: DEFLATE uses bit streams (not byte-aligned)
- **Dynamic Huffman tables**: Requires building decode tables on GPU
- **Backward references**: LZ77 lookback requires sequential processing

**Feasibility**: Medium
- Bit operations: Doable in Metal (bitfield operations supported)
- Huffman decoding: Can be parallelized per-byte
- LZ77: Challenging (requires sequential writes to output buffer)

### Challenge 2: Memory Access Patterns

**DEFLATE memory patterns**:
```
Input:  Sequential reads (bit stream parsing)
Output: Random writes (LZ77 backward references)
        "Copy 10 bytes from 1000 bytes back"
```

**GPU implications**:
- Sequential reads: ✅ Good for GPU (coalesced memory access)
- Random writes: ⚠️ Problematic (may serialize on GPU)

**Mitigation strategy**:
- Use **local memory** for LZ77 window (32KB per workgroup)
- Limit backward references to within-block only
- Accept some serialization (still faster than CPU?)

### Challenge 3: Workgroup Size and Occupancy

**Metal constraints**:
- Max threads per workgroup: 1024
- Max threadgroup memory: 32 KB
- DEFLATE output buffer: Up to 64 KB per block

**Strategy**:
- 1 workgroup = 1 bgzip block
- 256-512 threads per workgroup (decode in parallel within block)
- Use threadgroup memory for LZ77 window (fits in 32 KB)

**Occupancy concern**: Only 40 GPU cores, each with limited workgroups
- M4 Max: 40 cores × 1024 threads/core = 40,960 max concurrent threads
- Our workload: 500 blocks × 256 threads/block = 128,000 threads
- **Good**: We have enough parallelism to saturate GPU

---

## Prior Art Research

### Existing GPU Decompression Implementations

**1. NVIDIA Rapids cuDF** (gzip decompression):
- Uses CUDA for parallel gzip decompression
- Block-level parallelism (like our approach)
- Reports 5-10× speedup vs CPU
- **Source**: https://github.com/rapidsai/cudf

**2. GPU-accelerated Huffman Decoding** (research papers):
- Multiple papers on GPU Huffman decoding
- Key insight: **Parallel Huffman** (decode multiple symbols simultaneously)
- Challenge: Dependencies in LZ77 phase

**3. libdeflate** (Intel, CPU-only):
- Highly optimized SIMD implementation
- No GPU version
- Shows DEFLATE can be optimized heavily

**Conclusion**: GPU DEFLATE is feasible but not widely implemented. Opportunity for novel contribution!

---

## Metal-Specific Advantages

### 1. Unified Memory Architecture

**Traditional GPU** (discrete):
```
CPU → PCIe transfer (slow!) → GPU memory → Decompress → PCIe transfer → CPU
```

**Apple Silicon** (unified):
```
CPU → Zero-copy pointer → GPU decompresses → Zero-copy pointer → CPU
```

**Benefit**: Eliminates transfer overhead entirely!
- No cudaMemcpy equivalent needed
- Compressed data is already accessible to GPU
- Decompressed data written directly to shared memory

### 2. Metal Shader Efficiency

**Metal advantages**:
- Low-latency dispatch (optimized for macOS)
- Excellent tooling (Metal Debugger, System Trace)
- Native integration with Swift/Rust (metal-rs)

### 3. High Memory Bandwidth

**M4 Max**: 546 GB/s memory bandwidth
- Much higher than CPU threads can saturate
- Ideal for memory-bound workloads like decompression

---

## Implementation Strategy

### Approach: Hybrid CPU-GPU Pipeline

Don't do everything on GPU. Instead:

```
CPU Thread 1: Read bgzip blocks from disk
                ↓
CPU Thread 2: Parse block headers, prepare GPU buffers
                ↓
GPU:          Decompress N blocks in parallel (N = 40-400)
                ↓
CPU Thread 3: Parse FASTQ records from decompressed data
                ↓
CPU Thread 4: NEON processing
```

**Why hybrid**:
- CPU good at: I/O, parsing, irregular workloads
- GPU good at: Parallel compute, regular memory access
- Use each for what it's best at

### Metal Shader Pseudocode

```metal
// Metal Shading Language (MSL)

kernel void deflate_decompress(
    device uint8_t* compressed [[buffer(0)]],      // Input: compressed block
    device uint8_t* decompressed [[buffer(1)]],    // Output: decompressed data
    device uint32_t* block_sizes [[buffer(2)]],    // Block sizes for indexing
    uint block_id [[threadgroup_position_in_grid]],
    uint thread_id [[thread_position_in_threadgroup]],
    threadgroup uint8_t* lz77_window [[threadgroup(0)]]  // Shared memory
) {
    // Each workgroup handles one bgzip block
    uint block_start = block_offsets[block_id];
    uint block_size = block_sizes[block_id];

    // Step 1: Build Huffman decode tables (parallel within workgroup)
    threadgroup HuffmanTable lit_table;
    threadgroup HuffmanTable dist_table;

    if (thread_id == 0) {
        parse_huffman_tables(compressed + block_start, &lit_table, &dist_table);
    }
    threadgroup_barrier(mem_flags::mem_threadgroup);

    // Step 2: Parallel Huffman decoding
    // Each thread decodes a chunk of the bit stream
    uint chunk_start = thread_id * CHUNK_SIZE;
    for (uint i = chunk_start; i < chunk_start + CHUNK_SIZE; i++) {
        uint symbol = huffman_decode(&lit_table, compressed, bit_offset);

        if (symbol < 256) {
            // Literal byte
            lz77_window[output_pos++] = symbol;
        } else {
            // Length-distance pair (LZ77)
            uint length = decode_length(symbol);
            uint distance = huffman_decode(&dist_table, compressed, bit_offset);

            // Copy from LZ77 window
            for (uint j = 0; j < length; j++) {
                lz77_window[output_pos + j] = lz77_window[output_pos - distance + j];
            }
            output_pos += length;
        }
    }

    // Step 3: Write output to device memory
    threadgroup_barrier(mem_flags::mem_device);
    if (thread_id == 0) {
        memcpy(decompressed + output_offset, lz77_window, output_size);
    }
}
```

**Complexity**: High (this is a simplified version)
**Estimate**: 500-1000 lines of Metal shader code

---

## Performance Projections

### Conservative Estimate

**Assumptions**:
- GPU dispatch overhead: 50 µs per kernel launch
- Per-block decompression: 10 µs (GPU) vs 90 µs (CPU single-thread)
- Memory bandwidth: Not saturated (we're compute-bound)

**For 485 blocks** (Large file):
```
CPU parallel (16 cores): 485 blocks ÷ 16 = 30 blocks/core × 90 µs = 2.7 ms
GPU (40 cores):          485 blocks ÷ 40 = 12 blocks/core × 10 µs = 0.12 ms + 50 µs dispatch
                         = 170 µs total
```

**Speedup**: 2.7 ms ÷ 0.17 ms = **15.9× faster** than CPU parallel!

### Optimistic Estimate

**If we can**:
- Reduce per-block time to 5 µs (optimized shader)
- Batch dispatch (1 launch for all blocks): 50 µs overhead total
- Use all 40 cores efficiently

**For 485 blocks**:
```
GPU optimized: 485 blocks ÷ 40 = 12 blocks/core × 5 µs = 60 µs + 50 µs = 110 µs
```

**Speedup**: 2.7 ms ÷ 0.11 ms = **24.5× faster** than CPU parallel!

### Realistic Estimate

**Accounting for**:
- DEFLATE complexity (not trivially parallelizable)
- Memory bandwidth limits (546 GB/s shared with CPU)
- GPU occupancy (may not be 100%)

**Expected speedup**: **10-15× vs CPU parallel, 60-100× vs sequential**

---

## Implementation Complexity

### Estimated Effort

**Metal shader** (DEFLATE implementation):
- Research: 1-2 days (understand DEFLATE format deeply)
- Implementation: 3-5 days (write shader, debug)
- Testing: 1-2 days (validate correctness)
- **Total**: 5-9 days for shader

**Rust integration** (metal-rs):
- Setup: 0.5 days (add metal-rs dependency)
- Buffer management: 1 day (create Metal buffers)
- Dispatch logic: 1 day (kernel launch, synchronization)
- **Total**: 2.5 days for Rust wrapper

**Testing and benchmarking**:
- Correctness: 1 day (compare GPU vs CPU output byte-by-byte)
- Performance: 1 day (benchmark various file sizes)
- Profiling: 1 day (Metal System Trace analysis)
- **Total**: 3 days for validation

**Grand total**: 10-15 days for complete GPU implementation

**Recommendation**: This is substantial effort. Should we proceed?

---

## Alternatives to Consider

### Alternative 1: Simpler Compression Format

**Instead of DEFLATE**, use a GPU-friendly format:
- **LZ4**: Simpler than DEFLATE, already GPU implementations exist
- **Zstandard**: Modern, fast, better than gzip
- **Snappy**: Google's format, designed for speed

**Pros**:
- Faster implementation (1-2 days vs 10-15 days)
- Better GPU performance (less complex algorithm)
- Already benchmarked zstd (3-4× faster than gzip)

**Cons**:
- Not compatible with existing bgzip/BAM files
- Requires ecosystem adoption

### Alternative 2: CPU SIMD + Parallel

**Optimize CPU implementation further**:
- Use NEON for Huffman decoding
- Optimize memory access patterns
- Better threading (reduce overhead)

**Potential**: 8-10× speedup (vs 6.5× current)

**Pros**: Simpler, works on all ARM (not just Apple)
**Cons**: Not as fast as GPU (10-15×)

### Alternative 3: Hybrid - GPU for Subset

**Use GPU only for most expensive phase**:
- CPU: Parse headers, build Huffman tables
- GPU: Parallel Huffman decoding (embarrassingly parallel)
- CPU: LZ77 processing (sequential)

**Pros**: Simpler GPU shader, faster to implement
**Cons**: Less speedup (maybe 8-10× instead of 15×)

---

## Decision Framework

### Proceed with GPU if:

1. **High impact**: Would provide ≥10× speedup over CPU parallel
2. **Feasible**: Can implement in 2-3 weeks
3. **Novel**: No existing tool does this (publication potential)
4. **Reusable**: Works with existing BAM/CRAM files (ecosystem benefit)

### Consider alternatives if:

1. **Complexity**: Takes >3 weeks to implement
2. **Performance**: GPU only 2-3× faster than optimized CPU
3. **Portability**: Only works on Apple Silicon (limits adoption)

---

## Recommendation

### Path Forward: **Phased Approach**

**Phase 1** (Now): Quick feasibility test
- Implement trivial Metal shader (just copy memory)
- Measure dispatch overhead + memory bandwidth
- **Time**: 0.5 days
- **Decision point**: If overhead <100 µs, proceed

**Phase 2** (Next): Minimal DEFLATE shader
- Implement Huffman decoding only (no LZ77)
- Measure per-block decompression time
- **Time**: 2-3 days
- **Decision point**: If <20 µs/block, proceed

**Phase 3** (If Phase 2 succeeds): Full implementation
- Add LZ77 processing
- Optimize for occupancy
- Full benchmarking
- **Time**: 5-7 days
- **Deliverable**: Production-ready GPU decompression

**Total risk-adjusted time**: 3-5 days (if we stop after Phase 2) to 8-11 days (full implementation)

### My Recommendation

**Yes, proceed with Phase 1 feasibility test**:
- Low risk (0.5 days)
- High potential reward (10-15× speedup)
- Novel contribution (no existing tool)
- Unique to Apple Silicon (unified memory advantage)

If Phase 1 shows promise (overhead <100 µs), we can decide whether to continue to Phase 2.

---

## Next Steps

1. **Add metal-rs dependency** to Cargo.toml
2. **Write trivial Metal shader** (memory copy only)
3. **Measure dispatch overhead** and memory bandwidth
4. **Report findings** and decide on Phase 2

Would you like me to proceed with Phase 1 feasibility test?

---

**Document Author**: Claude
**Date**: November 4, 2025
**Status**: Investigation complete, ready for Phase 1 implementation
**Risk**: Low (0.5-day feasibility test)
**Potential Reward**: High (10-15× speedup, novel contribution)
