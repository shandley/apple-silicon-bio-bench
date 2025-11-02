# Neural Engine Pilot: Decision Summary

**Date**: November 2, 2025
**Status**: Design complete, awaiting decision
**Current Progress**: 5/9 dimension pilots complete

---

## TL;DR

**Question**: Proceed with Neural Engine pilot (6/9) or defer to simpler pilots first?

**Recommendation**: **Defer Neural Engine**, complete Hardware Compression/GCD/QoS pilots first (faster iteration)

**Reason**: Neural Engine pilot is 2-3× more complex (1 week vs 2-3 days) with likely negative finding (like AMX)

---

## Pilot Comparison

| Pilot | Complexity | Time | Likely Result | Dependencies |
|-------|-----------|------|---------------|--------------|
| **Neural Engine** (6/9) | **HIGH** | **5-7 days** | Negative (conversion overhead) | Core ML FFI, models |
| **Hardware Compression** (7/9) | Low | 2-3 days | Positive (proven tech) | AppleArchive |
| **GCD/QoS** (8/9) | Low | 2-3 days | Positive (system integration) | GCD framework |
| **M5 GPU Neural Accel** (9/9) | Medium | 2-3 days | Unknown (M5 required) | M5 chip |

---

## Neural Engine Pilot Details

### Scope (if proceeding)

**Operations**: 3 (quality_filter, complexity_score, adapter_trimming)
**Backends**: 2 (Naive CPU, Neural Engine)
**Scales**: 6 (100, 1K, 10K, 100K, 1M, 10M)
**Total**: 36 experiments

### Implementation Complexity

**Required**:
1. Swift Core ML wrapper (1-2 days)
2. Rust FFI bindings (1 day)
3. Stub ML models (0.5 days)
4. Neural Engine backends for 3 operations (1 day)
5. Experiment harness (0.5 days)

**Total**: 4-5 days implementation + 1 day execution/analysis = **5-6 days**

**Comparison**:
- AMX pilot: 1 day (Accelerate framework)
- GPU pilot: 3 days (Metal shaders)
- NEON pilot: 2 days (SIMD intrinsics)
- **Neural Engine**: 5-6 days (most complex to date)

### Expected Result

**Most Likely** (80% probability):
- Neural Engine 0.5-0.8× slower than CPU
- Conversion overhead (sequence → tensor) dominates
- Critical negative finding (like AMX)
- **Decision**: Skip Neural Engine for remaining operations

**Possible** (15% probability):
- Neural Engine 1.0-1.5× faster at very large scale (1M+ sequences)
- Batch inference amortizes conversion
- **Decision**: Conditionally use for batch operations only

**Unlikely** (5% probability):
- Neural Engine 2-5× faster across all scales
- **Decision**: Major shift toward ML-based sequence operations

---

## Alternative Path: Hardware Compression First

### Hardware Compression Pilot (7/9)

**What**: Test Apple's hardware-accelerated compression (AppleArchive framework)

**Operations**: I/O-heavy operations
- fastq_parsing (with compressed input)
- format_conversion (with compression)
- Quality/base statistics (stream from compressed)

**Complexity**: Low (native macOS framework, well-documented)

**Expected Result**: Positive (hardware compression proven technology)

**Time**: 2-3 days

**Benefits**:
- Fast iteration toward 9/9 completion
- Broadly applicable (all I/O operations)
- Likely positive finding (builds momentum)
- Well-supported APIs (no complex FFI)

### Proposed Sequence

**Option A** (Recommended): Defer Neural Engine
1. Hardware Compression (7/9) → 2-3 days ✅
2. GCD/QoS (8/9) → 2-3 days ✅
3. M5 GPU Neural Accel (9/9) → 2-3 days (if M5 available) ✅
4. **Neural Engine (6/9)** → 5-6 days (after 9/9 complete)

**Total to 9/9**: ~7-9 days
**Then**: Neural Engine as "bonus" advanced pilot

**Option B**: Neural Engine Now
1. **Neural Engine (6/9)** → 5-6 days
2. Hardware Compression (7/9) → 2-3 days
3. GCD/QoS (8/9) → 2-3 days
4. M5 GPU Neural Accel (9/9) → 2-3 days

**Total to 9/9**: ~12-15 days

**Time difference**: 3-6 days saved by deferring Neural Engine

---

## Decision Matrix

### Proceed with Neural Engine Now

**Pros**:
✅ Sequential pilot order (6/9 → 7/9 → 8/9 → 9/9)
✅ Core ML integration useful for future work
✅ Complete Apple Silicon hardware coverage
✅ Negative finding equally valuable (prevents wasted effort)

**Cons**:
❌ 2-3× longer than other pilots (5-6 days vs 2-3 days)
❌ Complex FFI (Swift ↔ Rust)
❌ Likely negative result (conversion overhead, like AMX)
❌ Only 3 operations naturally ML-amenable
❌ Slows progress toward 9/9 baseline

### Defer Neural Engine

**Pros**:
✅ Faster iteration to 9/9 (7-9 days vs 12-15 days)
✅ Build momentum with likely-positive pilots
✅ Complete baseline 9 pilots, then explore advanced topics
✅ Neural Engine research remains valid (documented, ready)

**Cons**:
❌ Out-of-sequence (skip 6, do 7/8/9, return to 6)
❌ Neural Engine deferred (but not abandoned)

---

## Recommendation

### Defer Neural Engine to Post-9/9

**Rationale**:
1. **Velocity**: 3-6 days saved reaching 9/9 baseline
2. **Momentum**: Likely-positive pilots (compression, GCD) build confidence
3. **Methodology**: Baseline 9 pilots → advanced topics (Neural Engine, ML approaches)
4. **Pragmatism**: 5-6 day investment for likely-negative finding = low ROI now

**Proposed Timeline**:

**Week 1** (Nov 3-9):
- Hardware Compression pilot (7/9) → 2-3 days
- GCD/QoS pilot (8/9) → 2-3 days
- **Checkpoint**: 7/9 complete

**Week 2** (Nov 10-16):
- M5 GPU Neural Accel (9/9) → 2-3 days (or skip if no M5)
- Analysis of 9/9 baseline pilots
- **Checkpoint**: 9/9 complete ✅

**Week 3+** (Nov 17+):
- Neural Engine pilot (6/9, bonus) → 5-6 days
- Statistical analysis of full dataset
- Rule extraction and documentation

**Milestone**: 9/9 baseline complete in ~2 weeks vs ~3 weeks

---

## What's Ready (if proceeding with Neural Engine)

**Already Created**:
✅ Protocol document (`protocol.md`)
✅ Implementation research (`implementation_research.md`)
✅ Technical approach validated
✅ Model creation scripts designed

**To Implement** (if green-lit):
1. Swift Core ML wrapper (1-2 days)
2. Rust FFI bindings (1 day)
3. Create 3 stub models (0.5 days)
4. Neural Engine backends (1 day)
5. Experiment harness (0.5 days)
6. Run experiments + analyze (1 day)

**Total**: 5-6 days to completion

---

## User Decision Required

**Question**: Proceed with Neural Engine pilot now, or defer to post-9/9?

**Option 1** (Recommended): Defer Neural Engine
- ✅ Faster to 9/9 baseline (save 3-6 days)
- ✅ Next: Hardware Compression (7/9) - 2-3 days

**Option 2**: Proceed with Neural Engine
- ✅ Sequential pilot order
- ⏱️ Slower to 9/9 (5-6 days for this pilot)
- ✅ Next: Neural Engine implementation (start immediately)

**Default (if no input)**: Defer Neural Engine, proceed with Hardware Compression

---

**Created**: November 2, 2025
**Status**: Awaiting user decision
**Files Created**:
- `experiments/phase1_neural_dimension/protocol.md`
- `experiments/phase1_neural_dimension/implementation_research.md`
- `experiments/phase1_neural_dimension/DECISION_SUMMARY.md`

**Next Actions** (depending on decision):
- **If defer**: Design Hardware Compression pilot (7/9)
- **If proceed**: Begin Swift Core ML wrapper implementation
