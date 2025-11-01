# Phase 1 (continued): GPU Dimension Protocol

**Date Created**: October 31, 2025
**Status**: Planning
**Objective**: Systematically characterize GPU (Metal) performance across operations and batch sizes

---

## Research Questions

1. **Batch size cliff**: Does the 50K threshold generalize across operation types?
2. **Operation suitability**: Which operations benefit from GPU parallelism?
3. **Overhead characterization**: What's the fixed cost per GPU dispatch?
4. **Optimal batch sizes**: What's the sweet spot for each operation category?
5. **Unified memory impact**: How does Apple Silicon's unified memory affect performance?

---

## Background from BioMetal

### Validated Findings (Screen Command, Fuzzy K-mer Matching)

| Batch Size | Performance | Notes |
|------------|------------|-------|
| 100 | 25,000× SLOWER | GPU overhead (50-100ms) dominates 0.004ms CPU time |
| 1,000 | ~2,500× SLOWER | Still overhead-dominated |
| 10,000 | ~250× SLOWER | Approaching break-even |
| 50,000 | ~1× (break-even) | Cliff threshold |
| 100,000 | 6× FASTER | GPU parallelism wins |
| 1,000,000 | 6× FASTER | Sustained benefit |
| 5,000,000 | 6× FASTER | No degradation at scale |

**Pattern**: Fixed overhead (~50-100ms) + linear speedup for large batches

### Key Insight from BioMetal

> "The GPU is not faster for small batches. It's only faster when you can amortize the dispatch overhead across enough parallel work."

---

## Experimental Design

### Hypothesis

**GPU benefit depends on**:
1. **Batch size** (amortize fixed overhead)
2. **Operation complexity** (more compute = better GPU utilization)
3. **Memory access pattern** (coalesced vs scattered)
4. **Data parallelism** (embarrassingly parallel vs data-dependent)

### Operations to Test

**Priority 1** (simple, embarrassingly parallel):
- Base counting (N=1, complexity 0.40) - Expected: Similar to BioMetal pattern
- GC content (N=2, complexity 0.32) - Expected: Cliff at ~50K
- Reverse complement (N=3, complexity 0.45) - Expected: Transform may have lower threshold

**Priority 2** (filtering, data-dependent):
- Quality filter (N=8, complexity 0.55) - Expected: Harder to parallelize (branches)
- Length filter (N=9, complexity 0.50) - Expected: Minimal GPU benefit (trivial op)

**Priority 3** (complex, interesting):
- Complexity score (N=10, complexity 0.61) - Expected: May benefit if reformulated for GPU

### Scales (Batch Sizes)

Based on BioMetal cliff at 50K, test around the threshold:

| Scale | N_seqs | Purpose |
|-------|--------|---------|
| Tiny | 100 | Extreme overhead (expect 10,000-25,000× slower) |
| Small | 1,000 | High overhead (expect 1,000-2,500× slower) |
| Medium | 10,000 | Moderate overhead (expect 100-250× slower) |
| Large | 50,000 | **Cliff threshold** (expect ~1×, break-even) |
| VeryLarge | 100,000 | Post-cliff (expect 2-6× faster) |
| Huge | 500,000 | Sustained benefit (expect 4-6× faster) |
| Massive | 1,000,000 | Maximum benefit (expect 6-8× faster) |
| Ultra | 5,000,000 | Validate no degradation |

**8 scales** (vs 6 in Phase 2) - need finer granularity around cliff

### Configurations

For each operation × scale:

1. **CPU Naive** - Baseline scalar implementation
2. **CPU NEON** - Vectorized implementation (current best)
3. **CPU Parallel (4 threads)** - Multi-threaded NEON
4. **GPU** - Metal compute shader

**Total experiments**: 6 operations × 8 scales × 4 configs = **192 experiments**

### Measurements

For each experiment:
- **Execution time** (ms) - including GPU dispatch overhead
- **Dispatch overhead** (ms) - time to setup GPU kernel
- **Kernel time** (ms) - actual GPU execution time
- **Memory transfer time** (ms) - should be ~0 on unified memory
- **Speedup vs CPU NEON** - primary metric
- **GPU utilization** (%) - from Metal Performance HUD
- **Power consumption** (W) - if available

---

## Implementation Plan

### Step 1: Metal Infrastructure (1-2 days)

**Create Metal backend**:
- `crates/asbb-gpu/` - New crate for GPU operations
- `MetalBackend` - Initialize Metal device, command queue
- `MetalBuffer` - Unified memory buffer management
- `MetalKernel` - Compile and dispatch compute shaders

**Dependencies**:
```toml
[dependencies]
metal = "0.27"
metal-rs = "0.27"
```

**Key design**:
- Use Apple Silicon unified memory (zero-copy)
- Compile Metal shaders at runtime (`.metal` files)
- Profile dispatch overhead separately

### Step 2: Simple Operation (Base Counting) (1 day)

**Implement** `base_counting.metal`:
```metal
kernel void count_bases(
    device const uchar* sequences [[buffer(0)]],
    device const uint* seq_lengths [[buffer(1)]],
    device uint* counts [[buffer(2)]],        // [A, C, G, T] per sequence
    uint gid [[thread_position_in_grid]]
) {
    // Count bases for sequence[gid]
    // Each thread processes one sequence
}
```

**Add to** `base_counting.rs`:
```rust
pub fn execute_gpu(
    &self,
    data: &[SequenceRecord],
    batch_size: usize
) -> Result<OperationOutput> {
    let backend = MetalBackend::new()?;

    // Unified memory: data already accessible to GPU
    let results = backend.dispatch_kernel(
        "count_bases",
        data,
        batch_size
    )?;

    Ok(results)
}
```

### Step 3: Pilot Experiment (Base Counting) (0.5 days)

**Run** across 8 scales, measure:
- Cliff threshold (expected ~50K)
- Maximum speedup (expected 4-6×)
- Dispatch overhead (expected 50-100ms)

**Validate**:
- GPU output matches CPU NEON output
- Unified memory has zero transfer cost
- Overhead measurement is accurate

### Step 4: Expand to Other Operations (2-3 days)

**Implement GPU kernels for**:
- GC content (similar to base counting)
- Reverse complement (per-sequence transformation)
- Quality filter (conditional output)
- Length filter (trivial, negative control)
- Complexity score (reduction operation)

### Step 5: Complete Pilot (0.5 days)

**Run all experiments**:
- 192 total (6 ops × 8 scales × 4 configs)
- ~2-3 hours automated execution
- Capture Metal Performance HUD data

### Step 6: Analysis (1 day)

**Answer research questions**:
1. Does 50K cliff generalize? (Compare across operations)
2. Which operations benefit? (Speedup by category)
3. What's the overhead? (Fixed cost measurement)
4. Optimal batch sizes? (Per-operation thresholds)

---

## Expected Patterns (Hypotheses)

### Pattern 1: Universal Cliff

**Hypothesis**: All embarrassingly parallel operations have similar cliff (~50K)

**Reasoning**: Overhead is dispatch cost (fixed), benefit is parallelism (linear)

**Expected**:
- Base counting: Cliff at 50K, 4-6× speedup
- GC content: Cliff at 50K, 4-6× speedup
- Reverse complement: Cliff at 40-60K, 3-5× speedup

### Pattern 2: Operation-Dependent Benefit

**Hypothesis**: More complex operations see higher GPU speedup

**Reasoning**: GPU has massive parallelism, benefits compute-heavy work

**Expected**:
- Simple (GC content): 4-6× speedup
- Medium (reverse complement): 6-8× speedup
- Complex (complexity score): 8-10× speedup

### Pattern 3: Filtering Penalty

**Hypothesis**: Filtering operations (conditional output) see reduced GPU benefit

**Reasoning**: Branching and irregular output patterns reduce GPU efficiency

**Expected**:
- Quality filter: Cliff at 75-100K (higher due to branches)
- Maximum speedup: 2-3× (lower due to divergence)

### Pattern 4: Zero Memory Transfer Cost

**Hypothesis**: Unified memory eliminates traditional GPU bottleneck

**Reasoning**: Apple Silicon has no PCIe transfer overhead

**Expected**:
- Memory transfer time: <1ms (measurement noise)
- Confirms unified memory advantage
- Dispatch overhead is pure kernel launch cost

---

## Success Criteria

### Technical
- [ ] Metal backend functional and tested
- [ ] All GPU kernels produce identical output to CPU NEON
- [ ] Dispatch overhead measured accurately (<10% variance)
- [ ] 192 experiments run successfully
- [ ] Metal Performance HUD data captured

### Scientific
- [ ] Cliff threshold identified (±10K accuracy)
- [ ] Operation categories characterized (simple/medium/complex)
- [ ] Unified memory benefit quantified
- [ ] Decision rules formulated (when to use GPU)
- [ ] Negative results documented (which ops DON'T benefit)

### Practical
- [ ] Clear threshold rules (e.g., "use GPU for N >50K")
- [ ] Per-operation optimal batch sizes
- [ ] Integration path for BioMetal
- [ ] Performance predictions (±20% accuracy)

---

## Risks and Mitigation

### Risk 1: Metal Shader Compilation Complexity

**Risk**: Metal shaders are complex, may take longer than expected

**Mitigation**:
- Start with simplest operation (base counting)
- Reuse shader patterns across operations
- Use Metal debugger for validation

### Risk 2: Overhead Variability

**Risk**: GPU dispatch overhead may vary (thermal, system load)

**Mitigation**:
- Run experiments multiple times (3 replicates)
- Measure overhead separately (empty kernel dispatch)
- Report variance in results

### Risk 3: Unified Memory Assumptions

**Risk**: May still have hidden transfer costs

**Mitigation**:
- Profile with Metal System Trace
- Measure memory access patterns
- Compare to discrete GPU if possible (validation)

---

## Timeline

**Week 1** (Day 1-3):
- Day 1: Metal infrastructure + base counting kernel
- Day 2: Base counting pilot experiment + validation
- Day 3: Implement 5 additional GPU kernels

**Week 2** (Day 4-5):
- Day 4: Run complete pilot (192 experiments)
- Day 5: Analysis + documentation

**Total**: 5 days (1 week)

---

## Deliverables

1. **Metal Infrastructure** (`crates/asbb-gpu/`)
   - `MetalBackend` - Device/queue management
   - `MetalKernel` - Shader compilation/dispatch
   - `MetalBuffer` - Unified memory buffers

2. **GPU Kernels** (`.metal` files)
   - `base_counting.metal`
   - `gc_content.metal`
   - `reverse_complement.metal`
   - `quality_filter.metal`
   - `length_filter.metal`
   - `complexity_score.metal`

3. **Updated Operations** (`execute_gpu()` methods)
   - All 6 operations with GPU backends

4. **Pilot Program** (`asbb-pilot-gpu`)
   - Automated testing across scales
   - Overhead measurement
   - Performance comparison

5. **Results Document** (`results/phase1_gpu_results.md`)
   - Complete experimental data (192 experiments)
   - Analysis of cliff threshold
   - Decision rules for GPU usage
   - Comparison to BioMetal findings

6. **Integration Guide** (for BioMetal)
   - When to use GPU
   - Optimal batch sizes
   - Performance predictions

---

## Decision Rules (Provisional, to be validated)

Based on BioMetal findings, expected rules:

```rust
/// GPU strategy selector (to be refined by experiments)
fn should_use_gpu(operation: &Operation, data_size: usize) -> bool {
    // Rule 1: Check minimum threshold
    if data_size < 50_000 {
        return false; // Below cliff, overhead dominates
    }

    // Rule 2: Check operation suitability
    match operation.category() {
        OperationCategory::ElementWise => {
            // Embarrassingly parallel, good for GPU
            data_size >= 50_000
        }
        OperationCategory::Filter => {
            // Branching reduces efficiency, higher threshold
            data_size >= 100_000
        }
        OperationCategory::Aggregation => {
            // Reduction overhead, depends on complexity
            data_size >= 75_000
        }
        _ => {
            // Conservative default
            false
        }
    }
}
```

**These rules will be refined based on experimental data.**

---

## Relationship to Other Dimensions

### Encoding Dimension (Phase 2)

**Question**: Does GPU benefit from 2-bit encoding?

**Hypothesis**: Yes, due to memory bandwidth
- 2-bit has 4× less data to transfer (though unified memory reduces impact)
- GPU has higher memory bandwidth utilization
- May see benefit even for single operations

**Future test**: GPU + 2-bit encoding (Phase 2 revisited with GPU)

### Parallelism Dimension (Phase 1)

**Question**: GPU vs CPU parallelism (Rayon)

**Observation**: BioMetal saw 1.6× from Rayon (4 threads), 6× from GPU

**This experiment will formalize**: When to use GPU vs CPU parallelism

### Composition (Phase 3)

**Question**: Does GPU benefit pipelines more than single operations?

**Hypothesis**: Yes, if data stays on GPU between operations
- First operation: Convert + dispatch (overhead)
- Subsequent operations: Already on GPU (no overhead)
- Final operation: Return results

**Future test**: Multi-operation GPU pipelines

---

## Notes

- **This is systematic isolation**: Testing GPU in isolation, just like we isolated encoding
- **BioMetal as validation**: We already know GPU works, now we're characterizing it
- **Operation-dependent**: Like encoding, GPU benefit likely varies by operation type
- **Negative results matter**: Understanding when GPU doesn't help is as valuable as when it does

---

**Status**: Protocol complete, ready to implement
**Next step**: Create Metal infrastructure (`crates/asbb-gpu/`)
**Estimated effort**: 5 days (1 week)
**Expected outcome**: Formalized GPU decision rules with experimental validation
