# Parallel/Threading Dimension Pilot - Quick Start

## Overview

Comprehensive testing of parallel performance across all 10 operations with core affinity (QoS-based).

**Scope**: 600 experiments (10 ops × 10 configs × 6 scales)
**Runtime**: 3-4 hours (automated)
**Hardware**: M1/M2/M3/M4 Mac required

---

## Quick Start

### 1. Build (release mode)

```bash
cargo build --release -p asbb-cli --bin asbb-pilot-parallel
```

### 2. Run with output to CSV

```bash
cd /Users/scotthandley/Code/apple-silicon-bio-bench

# Run and save results
cargo run --release -p asbb-cli --bin asbb-pilot-parallel \
  > results/parallel_dimension_raw_$(date +%Y%m%d_%H%M%S).csv \
  2> results/parallel_log_$(date +%Y%m%d_%H%M%S).txt
```

### 3. Monitor progress (optional, in separate terminal)

```bash
tail -f results/parallel_log_*.txt
```

### 4. Validate with Activity Monitor

While running:
1. Open Activity Monitor (Applications → Utilities)
2. Window → CPU History
3. Observe which cores are active:
   - **P-cores**: First 4-6 cores (M4 Pro)
   - **E-cores**: Last 4 cores
   - **Default**: Mixed activity

---

## Output Files

**CSV format** (stdout):
```csv
operation,complexity,scale,num_sequences,threads,assignment,time_ms,speedup_vs_1t,efficiency,throughput_seqs_per_sec
base_counting,0.40,Tiny,100,1,default,0.083,1.00,1.00,1204.82
base_counting,0.40,Tiny,100,2,default,0.087,0.95,0.48,1149.43
...
```

**Log format** (stderr):
```
═══════════════════════════════════════════════════════════════
🧬 Operation: base_counting (complexity 0.40)
═══════════════════════════════════════════════════════════════
📦       Tiny (     100 seqs): → No parallel benefit
📦      Small (    1000 seqs): → Best: 4t/default (3.45×)
...
```

---

## Configurations Tested

### Thread Counts
- 1 thread (baseline)
- 2 threads
- 4 threads (typical P-core count)
- 8 threads (all cores)

### Core Assignments (per thread count)
- **Default**: OS scheduler decides
- **P-cores**: QoS UserInitiated (high priority → P-cores preferred)
- **E-cores**: QoS Background (low priority → E-cores preferred)

### Total per Operation
- 1t × 1 assignment = 1 config
- 2t × 3 assignments = 3 configs
- 4t × 3 assignments = 3 configs
- 8t × 3 assignments = 3 configs
- **Total**: 10 configs per operation per scale

---

## Expected Patterns

### Parallel Threshold
- **Small batches (<1K)**: Overhead dominates, no benefit
- **Medium batches (1K-100K)**: Good scaling (2-4× speedup)
- **Large batches (>100K)**: Excellent scaling (4-8× speedup)

### Core Assignment
- **P-cores**: 10-20% faster for compute-intensive operations
- **E-cores**: Competitive for trivial operations (~80-90%)
- **Default**: Often within 5% of optimal (good scheduler)

### Scaling Efficiency
- **Ideal**: Linear (2t=2×, 4t=4×, 8t=8×)
- **Typical**: Sublinear (2t=1.9×, 4t=3.8×, 8t=7.5×)
- **Poor**: Overhead dominates (speedup <1.0 at small scales)

---

## Troubleshooting

### Missing Datasets

**Error**: `Skipping Tiny (file not found: datasets/tiny_100_150bp.fq)`

**Solution**: Generate datasets first:
```bash
cargo run --release -p asbb-datagen -- generate-all
```

### High Variability

**Symptom**: Speedup values vary widely between runs

**Causes**:
- Background processes (Spotlight, Time Machine)
- Thermal throttling
- Other applications running

**Mitigation**:
- Close other applications
- Disable Time Machine during testing
- Run during low system activity
- Results are averaged over 20 iterations

### QoS Not Affecting Core Assignment

**Symptom**: Activity Monitor shows same cores regardless of QoS setting

**Explanation**:
- QoS is a *hint*, not a guarantee
- macOS scheduler makes final decision
- System load and thermal state affect assignment
- This is expected behavior on macOS

**Action**: Document observed patterns; QoS effects are statistical, not absolute

---

## Analysis Scripts

After execution, analyze results:

```bash
# Count total experiments
wc -l results/parallel_dimension_raw_*.csv

# Extract base counting results
grep "^base_counting," results/parallel_dimension_raw_*.csv > base_counting.csv

# Find best configuration per operation/scale
# (Python/R script TBD)
```

---

## Next Steps

After completion:

1. **Verify**: 600 data rows (+ 1 header row)
2. **Analyze**: Generate speedup matrices
3. **Visualize**: Create plots (speedup curves, efficiency)
4. **Document**: Write comprehensive findings document
5. **Derive rules**: Formalize decision functions

See `protocol.md` for detailed analysis plan.

---

## Files in This Directory

- `README.md` - This file (quick start guide)
- `protocol.md` - Comprehensive experimental protocol
- `results/` - Output files (created during execution)
  - `parallel_dimension_raw_YYYYMMDD_HHMMSS.csv`
  - `parallel_log_YYYYMMDD_HHMMSS.txt`

---

## Related Documentation

- Phase 1 NEON results: `results/phase1_all_dimensions_complete.md`
- Phase 2 encoding results: `results/phase2_encoding_complete_results.md`
- GPU dimension results: `results/phase1_gpu_dimension_complete.md`
- Project guide: `CLAUDE.md`

---

**Ready to run!** Allocate 3-4 hours and execute when ready.
