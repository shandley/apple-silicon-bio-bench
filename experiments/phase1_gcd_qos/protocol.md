# GCD/QoS Dimension Pilot - Protocol

**Date**: November 2, 2025
**Pilot**: Phase 1 GCD/QoS (7/9 dimensions)
**Status**: Design phase

---

## Research Question

**Does Apple's native Grand Central Dispatch (GCD) provide performance benefits over Rayon for bioinformatics sequence operations?**

### Sub-questions
1. Does GCD's native macOS integration improve throughput vs Rayon?
2. Do GCD's QoS classes provide better system cooperation than Rayon's thread pools?
3. Do GCD-specific features (serial queues, barriers, dispatch groups) help sequence operations?
4. What's the overhead of FFI (Rust → libdispatch) vs native Rust (Rayon)?

---

## Background

### What We Already Know (Parallel Dimension)

From the Parallel/Threading pilot (Entry 011, 720 experiments):
- ✅ **Rayon works well**: 1.0-6.1× speedup depending on complexity
- ✅ **QoS hints effective**: 1-7% differences between P-cores/E-cores/default
- ✅ **E-cores competitive**: For some operations, E-cores are 1-7% faster than P-cores
- ✅ **Parallel threshold**: ~1K sequences for most operations

**Key finding**: Rayon + QoS hints already provides good parallel performance and system cooperation.

### What GCD Might Offer

**Grand Central Dispatch (libdispatch)** is Apple's system-level concurrency framework:
- Native macOS API (C library)
- Deep OS integration (kernel-level thread management)
- QoS classes: user-interactive, user-initiated, default, utility, background
- Special features: Serial queues, concurrent queues, barrier dispatch, dispatch groups

**Potential benefits**:
- Better OS cooperation (GCD threads managed by kernel)
- Lower power consumption (OS can optimize for battery)
- Better thermal management (OS can throttle intelligently)
- Automatic adaptation to system load

**Potential costs**:
- FFI overhead (Rust → C → libdispatch)
- More complex implementation
- Less portable (macOS-specific)

---

## Hypothesis

**Expected outcome**: GCD provides **marginal or negative benefit** vs Rayon.

**Reasoning**:
1. **Pattern of recent pilots**: AMX (FFI overhead) and Hardware Compression (software overhead) both showed negative results
2. **FFI cost**: Crossing Rust → C boundary for each task dispatch adds overhead
3. **Rayon is well-optimized**: Rayon's work-stealing is very efficient for CPU-bound tasks
4. **QoS already works**: Parallel dimension showed QoS hints with Rayon are effective

**However**, GCD might win if:
- OS-level cooperation benefits (power, thermal) outweigh FFI overhead
- GCD's QoS classes provide better scheduling than manual thread pools
- Batch dispatch amortizes FFI cost (dispatch groups of work, not individual tasks)

---

## Experimental Design

### Operations (5)

Test across complexity spectrum:
1. **base_counting** (complexity 0.20, high NEON)
2. **quality_aggregation** (complexity 0.48, moderate NEON)
3. **complexity_score** (complexity 0.61, low NEON, best parallel)
4. **hamming_distance** (complexity 0.35, pairwise)
5. **sequence_length** (complexity 0.20, trivial)

**Rationale**: Sample entire complexity spectrum to see if GCD benefits vary by operation type.

### Configurations (6)

Compare GCD vs Rayon with different approaches:

1. **Rayon (baseline)**: Default Rayon thread pool, 4 threads
2. **Rayon + QoS**: Rayon with P-core affinity via QoS hints (from Parallel pilot)
3. **GCD concurrent (default QoS)**: Concurrent queue, default QoS
4. **GCD concurrent (user-initiated QoS)**: Concurrent queue, user-initiated QoS
5. **GCD concurrent (utility QoS)**: Concurrent queue, utility QoS (power-efficient)
6. **GCD barrier**: Concurrent queue with barrier synchronization (if applicable)

**Thread count**: Fixed at 4 threads for all configs (fair comparison)

### Scales (6)

Standard scales to test overhead at different sizes:
- **Tiny**: 100 sequences (~15 KB)
- **Small**: 1,000 sequences (~150 KB)
- **Medium**: 10,000 sequences (~1.5 MB)
- **Large**: 100,000 sequences (~15 MB)
- **VeryLarge**: 1,000,000 sequences (~150 MB)
- **Huge**: 10,000,000 sequences (~1.5 GB)

### Total Experiments

5 operations × 6 configurations × 6 scales = **180 experiments**

---

## Implementation Approach

### Rust FFI to libdispatch

Use `dispatch` crate or direct FFI:
```rust
// Option 1: dispatch crate (if exists)
use dispatch::Queue;

// Option 2: Direct FFI to libdispatch
extern "C" {
    fn dispatch_get_global_queue(priority: isize, flags: usize) -> *mut dispatch_queue_t;
    fn dispatch_apply_f(iterations: usize, queue: *mut dispatch_queue_t,
                        context: *mut c_void, work: extern "C" fn(*mut c_void, usize));
}
```

### GCD QoS Classes

Map macOS QoS to dispatch queue priorities:
```rust
const QOS_CLASS_USER_INTERACTIVE: isize = 0x21;
const QOS_CLASS_USER_INITIATED: isize = 0x19;
const QOS_CLASS_DEFAULT: isize = 0x15;
const QOS_CLASS_UTILITY: isize = 0x11;
const QOS_CLASS_BACKGROUND: isize = 0x09;
```

### Task Dispatch

Compare dispatch mechanisms:
```rust
// Rayon (baseline)
data.par_iter().map(|seq| process(seq)).collect()

// GCD concurrent
dispatch_apply(data.len(), queue, |i| {
    process(&data[i])
})

// GCD barrier (for operations requiring synchronization)
dispatch_barrier_async(queue, || {
    // Synchronization point
})
```

---

## Metrics

For each experiment, measure:

1. **Throughput** (sequences/second)
2. **Latency** (milliseconds)
3. **CPU utilization** (percentage)
4. **Power consumption** (if measurable via powermetrics)
5. **Overhead** (compare to Rayon baseline)

**Key metric**: Speedup vs Rayon baseline (>1.0 = GCD wins, <1.0 = Rayon wins)

---

## Success Criteria

### Positive Finding (GCD helps)
- GCD shows ≥10% speedup vs Rayon for ≥3/5 operations
- GCD provides power/thermal benefits (measurable via powermetrics)
- GCD-specific features (barriers, groups) enable new optimization patterns

**If positive**: Implement GCD backends for all operations, document when to use GCD vs Rayon

### Negative Finding (GCD doesn't help)
- GCD shows <10% speedup or is slower than Rayon
- FFI overhead dominates potential benefits
- Rayon's work-stealing is more efficient for compute-bound tasks

**If negative**: Document that Rayon is preferred, GCD offers no benefit. This is valuable (prevents wasted effort)!

### Marginal Finding (GCD helps for specific cases)
- GCD helps for specific operation types (e.g., I/O-bound, low CPU utilization)
- GCD's power/thermal benefits matter for long-running jobs
- Conditional rule: "Use GCD for X, Rayon for Y"

---

## Expected Outcomes

### Most Likely: Negative Finding

**Prediction**: GCD will show **0.9-1.0× speedup vs Rayon** (similar or slightly slower)

**Reasoning**:
1. FFI overhead adds latency to each dispatch
2. Rayon's work-stealing is highly optimized for CPU-bound work
3. Recent pilot pattern (AMX, Compression) suggests overhead dominates specialized features
4. QoS already works with Rayon (validated in Parallel pilot)

**Result**: "Use Rayon for sequence operations. GCD offers no throughput benefit."

### Less Likely: Positive Finding

**Scenario**: GCD shows 1.1-1.3× speedup for specific cases:
- I/O-bound operations (parsing, compression)
- Operations with variable work per task (barrier synchronization helps)
- Long-running operations (power/thermal benefits accumulate)

**Result**: Conditional rule for when to use GCD vs Rayon

### Possible Surprise: System-Level Benefits

**Scenario**: GCD throughput is similar, but system-level benefits are measurable:
- 10-20% lower power consumption (longer battery life)
- Better thermal management (less throttling under sustained load)
- Better behavior under high system load (OS can reallocate resources)

**Result**: "Use GCD for production, Rayon for benchmarking"

---

## Implementation Plan

### Phase 1: FFI Setup (30 minutes)
1. Research Rust `dispatch` crate or set up direct FFI
2. Create safe Rust wrappers for dispatch_apply
3. Implement QoS class selection

### Phase 2: Operation Backends (1 hour)
1. Add GCD backend to 5 selected operations
2. Implement dispatch_apply-based parallelism
3. Handle result collection (dispatch_apply doesn't return values directly)

### Phase 3: Pilot Harness (30 minutes)
1. Create `asbb-pilot-gcd` binary
2. Run 180 experiments (5 ops × 6 configs × 6 scales)
3. CSV output with Rayon baseline comparison

### Phase 4: Analysis (30 minutes)
1. Calculate speedup vs Rayon baseline
2. Identify patterns (if any)
3. Measure power consumption (optional, requires powermetrics)
4. Document findings

**Total estimated time**: 2-3 hours

---

## Risks and Mitigations

### Risk 1: FFI complexity
**Impact**: Implementation takes longer than estimated
**Mitigation**: Use existing `dispatch` crate if available, otherwise keep FFI minimal

### Risk 2: GCD doesn't work on non-macOS
**Impact**: Pilot only runs on macOS
**Mitigation**: This is acceptable (Apple Silicon-specific project)

### Risk 3: Power metrics require root
**Impact**: Can't measure power consumption without sudo
**Mitigation**: Focus on throughput first, power metrics optional

### Risk 4: Result is "it depends"
**Impact**: No clear rule, conditional logic
**Mitigation**: Document conditions clearly, prefer Rayon if marginal

---

## Comparison to Other Dimensions

### Similar to AMX and Hardware Compression

Like recent pilots, GCD/QoS tests whether **specialized system feature** helps:
- AMX: Specialized hardware (matrix coprocessor) → 0.91-0.93× (NO)
- Hardware Compression: Specialized I/O (decompression) → 0.30-0.67× (NO)
- **GCD/QoS**: Specialized dispatch (OS integration) → ???

**Pattern suggests**: System features often have overhead that negates benefits for simple operations

### Different from NEON/GPU/Parallel

Those dimensions tested **fundamental parallelism**:
- NEON: Data parallelism (SIMD) → 1.1-85× (YES, varies by operation)
- GPU: Massive parallelism (Metal) → 0.0001-2.7× (YES, for complex ops at large scale)
- Parallel: Task parallelism (threads) → 1.0-6.1× (YES, varies by complexity)

GCD/QoS is more about **dispatch mechanism** than fundamental parallelism.

---

## Success Definition

**Pilot is successful if we answer the research question definitively**:
- ✅ Positive: "GCD provides X× speedup for Y operations, use it"
- ✅ Negative: "GCD provides no benefit, use Rayon" (equally valuable!)
- ✅ Conditional: "GCD helps for X, Rayon for Y" (clear rules)
- ❌ Uncertain: "Sometimes helps, sometimes doesn't, unclear why"

**Negative findings are NOT failures** - they prevent wasted optimization effort!

---

## Next Steps

1. ✅ Create protocol (this document)
2. ⏳ Research Rust `dispatch` crate or setup FFI
3. ⏳ Implement GCD backends for 5 operations
4. ⏳ Create pilot harness (`asbb-pilot-gcd`)
5. ⏳ Run 180 experiments
6. ⏳ Analyze results and document findings
7. ⏳ Update PILOT_CHECKPOINT.md (7/9 complete)

---

**Protocol complete**: Ready for implementation
**Expected duration**: 2-3 hours (setup + experiments + analysis)
**Expected outcome**: Likely negative finding (GCD doesn't help due to FFI overhead)
