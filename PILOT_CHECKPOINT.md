# Individual Dimension Pilot Checkpoint

**Date**: November 2, 2025 (Updated: AMX complete)
**Purpose**: Track progress on systematic dimension pilots and prevent premature automation

---

## ⛔ CRITICAL RULE ⛔

**DO NOT proceed to Level 1/2 automated harness until ALL 9 dimension pilots are complete.**

**Reason**: Each pilot reveals unexpected patterns critical for understanding the performance space.

**Evidence**: 4/4 completed pilots found unexpected patterns:
- NEON: Complexity-speedup relationship (R² = 0.536)
- GPU: First GPU win, unified memory validation
- 2-bit Encoding: Overhead quantification (challenges conventional wisdom)
- Parallel: E-cores competitive, super-linear speedups

**Lesson from Nov 1-2**: We attempted Level 1/2 prematurely. Result: 4 crashes, 5+ hours wasted, 0 progress.

---

## Pilot Status Tracker

### ✅ Completed Pilots (5/9)

#### 1. NEON SIMD Dimension ✅
- **Experiments**: 60 (10 operations × 6 scales)
- **Date**: October 2025
- **Key Finding**: Complexity-speedup relationship quantified
- **Data**: Various individual operation files
- **Documentation**: Analysis in results/phase1/

#### 2. 2-bit Encoding Dimension ✅
- **Experiments**: 72 (2 operations × 6 backends × 6 scales)
- **Date**: October 2025
- **Key Finding**: 2-4× overhead from conversion cost
- **Data**: results/phase2/phase2_encoding_complete_results.md
- **Documentation**: Comprehensive analysis complete

#### 3. GPU Metal Dimension ✅
- **Experiments**: 32 (4 operations × 8 scales)
- **Date**: October 2025
- **Key Finding**: First GPU win @ 10M seqs (2.74× for complexity_score)
- **Data**: results/phase1/phase1_gpu_dimension_complete.md
- **Documentation**: GPU cliff at 10K sequences documented

#### 4. Parallel/Threading Dimension ✅
- **Experiments**: 720 (10 operations × 12 configs × 6 scales)
- **Date**: October 31, 2025
- **Key Finding**: E-cores competitive, super-linear speedups (150-268% efficiency)
- **Data**: results/parallel_dimension_raw_20251031_152922.csv
- **Documentation**: PENDING (needs analysis)

#### 5. AMX Matrix Engine Dimension ✅
- **Experiments**: 24 (edit_distance × 4 backends × 6 scales)
- **Date**: November 2, 2025
- **Key Finding**: AMX does NOT provide speedup (0.91-0.93× vs NEON)
- **Data**: results/phase1_amx_dimension/amx_pilot_raw_20251102_090714.csv
- **Documentation**: results/phase1_amx_dimension/amx_pilot_summary.md
- **Optimization Rule**: Use NEON + parallel, skip AMX

---

### ⏳ Remaining Pilots (4/9)

#### 6. Neural Engine Dimension ⏳ **← NEXT**
- **Experiments**: TBD (~240 planned)
- **Operations**: ML-amenable operations
  - sequence_classification
  - quality_prediction
  - adapter_detection (as image recognition)
  - taxonomy classification
- **Configurations**: Neural Engine vs CPU, Core ML models
- **Expected Insights**: ML-based sequence analysis performance

#### 7. Hardware Compression Dimension ⏳
- **Experiments**: TBD (~240 planned)
- **Operations**: I/O and memory-intensive operations
  - fastq_parsing with compression
  - intermediate result compression
  - format_conversion with compression
- **Configurations**: AppleArchive framework variants
- **Expected Insights**: Zero-cost compression opportunities

#### 8. GCD/QoS Dimension ⏳
- **Experiments**: TBD (~240 planned)
- **Operations**: All operations tested with GCD dispatch
- **Configurations**: Different QoS levels (user-initiated, background, etc.)
- **Expected Insights**: System-level optimization cooperation

#### 9. M5 GPU Neural Accelerators ⏳
- **Experiments**: TBD (~240 planned, if M5 available)
- **Operations**: AI-amenable operations
- **Configurations**: GPU Neural Accelerators vs Neural Engine vs CPU
- **Expected Insights**: 4× AI performance on GPU cores

---

## Pilot Completion Checklist

For each pilot to be considered **complete**, it must have:

- [ ] **Experiments run**: All planned experiments executed
- [ ] **Data collected**: Raw results in CSV or similar format
- [ ] **Patterns analyzed**: Key findings identified and documented
- [ ] **Documentation written**: Comprehensive analysis document created
- [ ] **Unexpected findings**: Novel insights captured
- [ ] **Rules derived**: Optimization guidelines extracted
- [ ] **Lab notebook entry**: Progress documented in chronological log

---

## Level 1/2 Prerequisites

**Before building Level 1/2 automated harness**:

1. ✅ All 9 dimension pilots complete
2. ✅ All pilots documented with findings
3. ✅ Optimization rules extracted from each dimension
4. ✅ Hardware compatibility matrix understood
5. ✅ Memory requirements characterized
6. ✅ Failure modes identified and mitigated

**Only then**: Build robust Level 1/2 harness with:
- Backend compatibility filtering
- Memory-aware scheduling
- Graceful error handling
- Comprehensive logging

---

## Lessons Learned (Nov 1-2, 2025)

**What went wrong**:
- Attempted Level 1/2 with only 4/9 pilots complete
- Harness crashed 4 times (experiments 211)
- 5+ hours wasted debugging
- No clear resolution found
- Violated systematic methodology

**What we should have done**:
- Continue with AMX pilot (next in sequence)
- Complete remaining 5 dimension pilots
- **Then** build Level 1/2 with full understanding

**Corrective action**:
- Document this checkpoint to prevent repeat
- Proceed with AMX pilot immediately
- Follow systematic methodology strictly

---

**Current Status**: 5/9 pilots complete ✅
**Next Action**: Neural Engine pilot (240 experiments) **← NEXT**
**DO NOT**: Attempt Level 1/2 until 9/9 complete

**Recent Completion** (Nov 2): AMX pilot - Found AMX does NOT help sequence ops (critical negative finding!)

**Last Updated**: November 2, 2025
