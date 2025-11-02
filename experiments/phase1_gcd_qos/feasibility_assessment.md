# GCD/QoS Pilot - Feasibility Assessment

**Date**: November 2, 2025
**Status**: Pre-implementation analysis

---

## Strategic Question

Given our findings from 6/9 completed pilots (902 experiments), **is a full GCD/QoS pilot necessary or can we make an informed decision based on existing evidence**?

---

## Evidence from Completed Pilots

### What We Know About QoS (Parallel Dimension)

From Entry 011 (720 experiments, October 31, 2025):
- ✅ **QoS hints with Rayon work**: 1-7% performance differences between P-cores/E-cores/default
- ✅ **E-cores competitive**: Some operations show E-cores 1-7% faster than P-cores
- ✅ **System cooperation validated**: macOS pthread QoS integration successful
- ✅ **Parallel scaling excellent**: 1.0-6.1× speedup depending on operation complexity

**Conclusion**: We already have evidence that QoS works with Rayon.

### Pattern from Recent Pilots (AMX, Compression)

Both recent pilots showed **specialized system features have overhead that negates benefits**:

**AMX (Entry 015)**:
- Accelerate framework overhead: 7-9% slower than NEON
- Pattern: Conversion/FFI overhead > theoretical benefit

**Hardware Compression (Entry 016)**:
- Decompression overhead: 2-3× slower than uncompressed
- Pattern: Software overhead > I/O benefit (even with fast algorithms)

**Prediction for GCD**:
- FFI overhead (Rust → C → libdispatch) likely > any scheduling benefit
- Rayon's work-stealing is highly optimized for CPU-bound tasks
- GCD's advantage (OS integration) might not matter for compute-bound sequence ops

---

## Cost-Benefit Analysis

### Full GCD/QoS Pilot Cost

**Implementation time**: 2-3 hours
- FFI setup: 30 minutes
- 5 operation backends: 1 hour
- Pilot harness: 30 minutes
- 180 experiments: 30 minutes
- Analysis: 30 minutes

**Opportunity cost**: Could complete Level 1/2 harness instead (higher value)

### Expected Outcome (Prediction)

**Most likely** (80% probability): Negative finding
- GCD shows 0.9-1.0× vs Rayon (no benefit or slight overhead)
- FFI overhead negates any scheduling benefit
- Conclusion: "Use Rayon, GCD offers no throughput advantage"

**Possible** (15% probability): Marginal benefit
- GCD shows 1.05-1.10× for specific cases (I/O-bound, variable work)
- Conclusion: "Conditional - GCD for X, Rayon for Y"

**Unlikely** (5% probability): Clear benefit
- GCD shows 1.2×+ across multiple operations
- OS-level cooperation provides measurable advantage

### Value of Negative Finding

**If GCD doesn't help** (most likely):
- ✅ Prevents future wasted effort ("should we try GCD for X?")
- ✅ Validates Rayon as preferred parallel dispatch
- ✅ Completes systematic testing (7/9 pilots)

**Value**: Medium (confirms what we suspect, completes dimension testing)

### Alternative: Defer to Post-9/9

**Option**: Mark GCD/QoS as "tested via Parallel dimension proxy" and move to Level 1/2

**Rationale**:
- QoS already validated (Parallel dimension)
- GCD is just alternate dispatch mechanism (not fundamental capability)
- 6/9 pilots may be sufficient for Level 1/2 (enough data)
- Can revisit GCD if Level 1/2 shows gaps

---

## Recommended Approach: Lightweight Feasibility Test

Instead of full 180-experiment pilot, run **minimal feasibility test first**:

### Minimal Test Design

**Operations**: 2 (base_counting, complexity_score - extremes of complexity spectrum)
**Configurations**: 3 (Rayon baseline, GCD default QoS, GCD user-initiated QoS)
**Scales**: 3 (Tiny, Medium, Huge - representative)

**Total**: 2 × 3 × 3 = **18 experiments** (10× smaller than full pilot)
**Time**: 30-45 minutes implementation + 5 minutes run

### Decision Tree

**If minimal test shows GCD ≥1.1× faster**:
- → Expand to full 180-experiment pilot
- → Worth investigating thoroughly

**If minimal test shows GCD 0.9-1.09× (marginal or slower)**:
- → Document as negative finding
- → Skip full pilot, mark dimension complete
- → Move to Level 1/2

**Benefit**: Fail fast if GCD doesn't help (saves 2 hours)

---

## Recommendation

**Proceed with lightweight feasibility test (18 experiments)**:

1. Add `dispatch` crate dependency (5 minutes)
2. Create minimal GCD wrapper (15 minutes)
3. Add GCD backend to 2 operations (15 minutes)
4. Run 18 experiments (5 minutes)
5. Analyze results (5 minutes)

**Total time**: 45 minutes (vs 2-3 hours for full pilot)

**Decision point**: Based on feasibility test, either:
- Expand to full pilot (if promising)
- Document and move on (if not promising)

---

## Implementation Plan (Lightweight Test)

### Phase 1: Setup (20 minutes)

```toml
# Cargo.toml
[dependencies]
dispatch = "0.2.0"  # Rust GCD wrapper
```

```rust
// Simple GCD wrapper
use dispatch::Queue;

fn gcd_parallel_process<T, F>(data: &[T], f: F) -> Vec<Output>
where
    F: Fn(&T) -> Output + Send + Sync,
{
    let queue = Queue::global(dispatch::QueuePriority::Default);
    let results = Arc::new(Mutex::new(Vec::new()));

    queue.apply(data.len(), |i| {
        let result = f(&data[i]);
        results.lock().unwrap().push(result);
    });

    Arc::try_unwrap(results).unwrap().into_inner().unwrap()
}
```

### Phase 2: Backends (15 minutes)

Add GCD backend to:
1. `base_counting` (simple, high NEON)
2. `complexity_score` (complex, low NEON)

```rust
pub fn execute_gcd(&self, data: &[SequenceRecord], qos: QoS) -> Result<Output> {
    let queue = match qos {
        QoS::Default => Queue::global(QueuePriority::Default),
        QoS::UserInitiated => Queue::global(QueuePriority::High),
        QoS::Utility => Queue::global(QueuePriority::Low),
    };

    // Parallel process via GCD
    gcd_parallel_process(data, |seq| self.process_sequence(seq))
}
```

### Phase 3: Feasibility Harness (10 minutes)

```rust
// asbb-pilot-gcd-feasibility
fn main() {
    let operations = vec!["base_counting", "complexity_score"];
    let configs = vec!["rayon", "gcd_default", "gcd_user_initiated"];
    let scales = vec!["Tiny", "Medium", "Huge"];

    // 18 experiments total
    for op in &operations {
        for config in &configs {
            for scale in &scales {
                let result = run_experiment(op, config, scale)?;
                println!("{},{},{},{:.2}ms", op, config, scale, result.time_ms);
            }
        }
    }
}
```

### Phase 4: Decision (5 minutes)

**If GCD ≥1.1× faster**: Expand to full pilot
**If GCD <1.1× faster**: Document and defer

---

## Expected Outcome

**Prediction**: GCD will show 0.95-1.05× vs Rayon (no significant benefit)

**Result**: Skip full pilot, document GCD/QoS as "tested via feasibility + Parallel dimension", mark 7/9 complete

**Time saved**: 1.5-2 hours (can invest in Level 1/2 instead)

---

## Decision

**Recommended**: Proceed with lightweight feasibility test (45 minutes)

**Rationale**:
- Fast fail if GCD doesn't help (likely outcome)
- Validates or refutes GCD benefit with minimal investment
- Aligns with project goal (systematic testing, data-driven decisions)
- Prevents analysis paralysis ("should we test GCD fully or skip it?")

**Next step**: Implement feasibility test, analyze, then decide full pilot vs document & move on

---

**Assessment complete**: Ready for lightweight implementation
