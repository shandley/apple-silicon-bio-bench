# AMX Matrix Engine Pilot - Ready to Begin

**Date**: November 2, 2025
**Status**: Planning complete, ready for implementation
**Pilot**: 5/9 dimension pilots

---

## ✅ Documentation Updates Complete

**Files updated**:
1. **CLAUDE.md**: Added strong warnings about premature Level 1/2
   - Documented Nov 1-2 failure (4 crashes, 5+ hours wasted)
   - Reinforced systematic pilot approach
   - Updated "For Claude" reminders with mandatory checks

2. **PILOT_CHECKPOINT.md**: NEW - Tracks pilot completion status
   - 4/9 pilots complete (NEON, Encoding, GPU, Parallel)
   - 5/9 remaining (AMX ← current, Neural Engine, HW Compression, GCD/QoS, M5)
   - Clear completion criteria for each pilot

3. **Git commit**: Changes committed to prevent future premature attempts

---

## 📋 AMX Pilot Overview

### Scope

**Experiments**: 192 total (tractable, focused)
- 8 operations × 4 hardware configs × 6 data scales
- **Primary** (matrix-native): edit_distance, hamming_distance, quality_statistics
- **Secondary** (reformulable): complexity_score, kmer_counting, minhash_sketching
- **Controls** (baseline): base_counting, reverse_complement

**Hardware configs**: 4
1. Baseline (naive, no AMX, no NEON)
2. AMX-only (via Accelerate framework)
3. NEON-only (existing implementations)
4. AMX + NEON hybrid

**Data scales**: 6 (100, 1K, 10K, 100K, 1M, 10M sequences)

**Expected duration**: 1-2 days (much more tractable than Level 1/2's 2,200 experiments)

---

### Why AMX?

**Novel opportunity**: AMX (Apple Matrix Extension) didn't exist pre-2020
- 512-bit wide matrix coprocessor
- Integrated with CPU (low latency vs GPU)
- Used internally by Core ML, Accelerate framework
- No traditional bioinformatics tools exploit it

**Research questions**:
1. Which sequence operations benefit from matrix reformulation?
2. When does AMX outperform NEON (128-bit SIMD)?
3. Can we discover novel matrix-based algorithms for sequences?

**Expected insights** (based on 4/4 previous pilots finding unexpected patterns):
- Edit distance: Likely significant speedup (matrix-native algorithm)
- Threshold discovery: When does AMX overhead pay off?
- Novel formulations: Can k-mer operations use sparse matrices?

---

## 🛠️ Implementation Approach

**Practical strategy**: Use **Accelerate framework** (not undocumented AMX intrinsics)

**Rationale**:
- AMX intrinsics are undocumented (unstable, risky)
- Accelerate framework uses AMX internally (official, stable)
- We can measure AMX benefit without diving into assembly

**Implementation plan**:
1. Add `accelerate-src` crate dependency
2. Implement edit_distance using Accelerate BLAS/LAPACK
3. Implement quality_statistics using Accelerate vDSP
4. Compare: Accelerate (uses AMX) vs NEON vs baseline
5. Measure speedup, identify patterns

---

## 📊 Expected Results

Based on 4/4 previous pilots finding unexpected patterns:

**Hypothesis 1**: Edit distance will show AMX speedup
- **Prediction**: 2-4× faster than NEON at large scales
- **Reason**: Matrix-native algorithm (Wagner-Fischer DP)

**Hypothesis 2**: AMX will have overhead at small scales
- **Prediction**: Break-even at ~1,000 sequences
- **Reason**: Matrix tile setup cost

**Hypothesis 3**: Controls won't benefit
- **Prediction**: Base counting, reverse complement slower with AMX
- **Reason**: Element-wise operations are NEON-optimal

**Novel discovery potential**:
- Can quality statistics use AMX for batch column operations?
- Can k-mer operations be reformulated as sparse matrices?
- Does AMX + NEON hybrid enable new algorithmic approaches?

---

## 🎯 Success Criteria

**Minimum viable pilot** (to proceed to next dimension):
- [ ] 3 primary operations tested (72 experiments minimum)
- [ ] AMX benefit identified for at least one operation
- [ ] Patterns documented (when does AMX help vs hurt?)
- [ ] Optimization rules derived

**Ideal complete pilot**:
- [ ] All 8 operations tested (192 experiments)
- [ ] Novel findings documented
- [ ] Comprehensive analysis written
- [ ] Lab notebook entry created

---

## 📁 Project Structure

**Created**:
```
experiments/phase1_amx_dimension/
├── protocol.md                  # Experimental design (complete)
├── implementation_research.md   # AMX approach (complete)
└── (results will go here)

results/phase1_amx_dimension/
└── (CSV data, analysis docs)
```

---

## 🚦 Next Steps

**Immediate** (Day 1):
1. Check if `edit_distance` operation already exists
2. Add Accelerate framework dependency
3. Implement AMX backend for edit_distance
4. Test correctness vs naive implementation

**Day 2**:
1. Run experiments (192 total, automated)
2. Collect data, monitor progress
3. Checkpoint frequently

**Day 3**:
1. Analyze results, visualize patterns
2. Document findings
3. Extract optimization rules
4. Update PILOT_CHECKPOINT.md (5/9 complete)

**Then**:
1. Proceed to Neural Engine pilot (6/9)
2. Continue systematic approach
3. **DO NOT** attempt Level 1/2 until all 9 pilots complete

---

## 📝 Lessons from Level 1/2 Mistake

**What we learned**:
- Systematic pilot approach works (proven 4/4 times)
- Individual pilots are tractable (240 experiments each)
- Level 1/2 is complex, hard to debug (4 crashes, no resolution)
- **Follow the methodology** - it exists for good reasons

**What we're doing differently**:
- AMX pilot: 192 experiments (manageable)
- Focused scope: One hardware dimension at a time
- Clear success criteria: Patterns first, automation later
- Documentation: Strong warnings to prevent premature Level 1/2

---

## ⏱️ Timeline to Level 1/2

**Current status**: 4/9 pilots complete
**Remaining work**:
1. AMX pilot (1-2 days) ← **YOU ARE HERE**
2. Neural Engine pilot (2-3 days)
3. Hardware Compression pilot (1-2 days)
4. GCD/QoS pilot (1-2 days)
5. M5 GPU Neural Accelerators (2-3 days, if M5 available)

**Total**: ~8-12 days of focused pilot work
**Then**: Level 1/2 with full understanding (no crashes, clear patterns)

---

**Status**: AMX pilot ready to begin
**Next action**: Implement AMX backend for edit_distance
**Expected completion**: November 4-5, 2025

**Created**: November 2, 2025
