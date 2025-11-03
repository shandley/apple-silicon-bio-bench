# Power Consumption Pilot - Findings

**Date**: November 2, 2025
**Experiment**: Environmental Pillar Validation
**Lab Notebook**: Entry 020

---

## Executive Summary

**Total experiments**: 24
**Operations tested**: 3
**Configurations**: 4 (naive, neon, neon_4t, neon_8t)
**Scales**: 2 (Medium 10K, Large 100K)

**Key Finding**: Energy scales with runtime
- Average time speedup: **33.3×**
- Average energy speedup: **49.9×**
- Average energy efficiency: **1.95** (1.0 = ideal)

---

## Results by Operation

### base_counting

**Large scale** (100000 sequences):

| Config | CPU Power (W) | Energy (mWh) | Energy/Seq (μWh) | Time Speedup | Energy Speedup | Efficiency |
|--------|--------------|--------------|------------------|--------------|----------------|------------|
| naive    |    4.8 |  80.385 |    0.001 |    1.0× |    1.0× |   1.00 |
| neon     |   13.8 | 230.609 |    0.000 |   15.3× |    5.3× |   2.87 |
| neon_4t  |   14.7 | 245.002 |    0.000 |   51.8× |   17.0× |   3.05 |
| neon_8t  |    3.7 |  61.523 |    0.000 |   70.2× |   91.8× |   0.77 |

**Medium scale** (10000 sequences):

| Config | CPU Power (W) | Energy (mWh) | Energy/Seq (μWh) | Time Speedup | Energy Speedup | Efficiency |
|--------|--------------|--------------|------------------|--------------|----------------|------------|
| naive    |    5.7 |  95.204 |    0.001 |    1.0× |    1.0× |   1.00 |
| neon     |   12.4 | 207.053 |    0.000 |   16.7× |    7.7× |   2.17 |
| neon_4t  |   13.3 | 222.345 |    0.000 |   41.4× |   17.7× |   2.34 |
| neon_8t  |    4.3 |  72.040 |    0.000 |   39.4× |   52.0× |   0.76 |

---

### gc_content

**Large scale** (100000 sequences):

| Config | CPU Power (W) | Energy (mWh) | Energy/Seq (μWh) | Time Speedup | Energy Speedup | Efficiency |
|--------|--------------|--------------|------------------|--------------|----------------|------------|
| naive    |    4.2 |  70.682 |    0.001 |    1.0× |    1.0× |   1.00 |
| neon     |   13.2 | 219.435 |    0.000 |   15.1× |    4.9× |   3.11 |
| neon_4t  |   13.9 | 230.893 |    0.000 |   52.2× |   16.0× |   3.27 |
| neon_8t  |    3.6 |  59.430 |    0.000 |   70.2× |   83.4× |   0.84 |

**Medium scale** (10000 sequences):

| Config | CPU Power (W) | Energy (mWh) | Energy/Seq (μWh) | Time Speedup | Energy Speedup | Efficiency |
|--------|--------------|--------------|------------------|--------------|----------------|------------|
| naive    |    4.5 |  75.080 |    0.001 |    1.0× |    1.0× |   1.00 |
| neon     |   12.1 | 201.411 |    0.000 |   17.1× |    6.4× |   2.68 |
| neon_4t  |   12.9 | 215.701 |    0.000 |   43.2× |   15.0× |   2.87 |
| neon_8t  |    3.9 |  65.281 |    0.000 |   38.3× |   44.1× |   0.87 |

---

### quality_aggregation

**Large scale** (100000 sequences):

| Config | CPU Power (W) | Energy (mWh) | Energy/Seq (μWh) | Time Speedup | Energy Speedup | Efficiency |
|--------|--------------|--------------|------------------|--------------|----------------|------------|
| naive    |    5.2 |  86.296 |    0.000 |    1.0× |    1.0× |   1.00 |
| neon     |   13.2 | 219.561 |    0.000 |   12.7× |    5.0× |   2.54 |
| neon_4t  |   12.5 | 207.946 |    0.000 |   29.2× |   12.1× |   2.41 |
| neon_8t  |    0.3 |   5.751 |    0.000 |   31.9× |  478.6× |   0.07 |

**Medium scale** (10000 sequences):

| Config | CPU Power (W) | Energy (mWh) | Energy/Seq (μWh) | Time Speedup | Energy Speedup | Efficiency |
|--------|--------------|--------------|------------------|--------------|----------------|------------|
| naive    |    6.1 | 101.562 |    0.000 |    1.0× |    1.0× |   1.00 |
| neon     |   12.2 | 203.083 |    0.000 |   20.3× |   10.2× |   2.00 |
| neon_4t  |   11.5 | 192.094 |    0.000 |   22.6× |   11.9× |   1.89 |
| neon_8t  |    3.6 |  60.572 |    0.000 |   11.7× |   19.7× |   0.60 |

---

## Power Draw Analysis

Does optimization increase power draw per unit time?

| Configuration | Average CPU Power (W) | vs Naive |
|---------------|----------------------|----------|
| naive          |                  5.1 |     1.00× |
| neon           |                 12.8 |     2.52× |
| neon_4t        |                 13.1 |     2.58× |
| neon_8t        |                  3.2 |     0.64× |

**Insight**: Power draw increases with parallelism, but total energy decreases due to faster completion.

---

## Environmental Impact Extrapolation

**Scenario**: Small lab running 10,000 analyses/year (base_counting, 100K sequences)

- Naive energy per analysis: 80.385 mWh
- Optimized energy per analysis: 61.523 mWh
- Energy saved per analysis: **18.862 mWh**

**Per-lab annual savings**:
- Energy saved: 188.6 Wh/year (0.189 kWh/year)
- CO₂ avoided: 0.09 kg/year

**Field-wide impact** (10,000 labs adopt):
- Energy saved: 1886.2 kWh/year
- CO₂ avoided: 0.9 tons/year

---

## Validation of "300× Less Energy" Claim

**Current claim** (from DEMOCRATIZING_BIOINFORMATICS_COMPUTE.md):
- Traditional HPC: 150 Wh (naive, 30 minutes)
- Mac Mini optimized: 0.5 Wh (NEON+Parallel, 1 minute)
- Reduction: 300×

**Our measurements** (Mac-to-Mac comparison):
- Naive (Mac): 80.385 mWh
- Optimized (Mac): 61.523 mWh
- Reduction: **1.3×**

**Conclusion**: Mac-to-Mac comparison shows ~1× energy reduction. The 300× claim likely compares HPC (different hardware) to Mac, not Mac-to-Mac.

---

## Next Steps

### Expand to Full 80 Experiments?

**Decision criteria**:
- ✅ If energy efficiency ≈ 1.0 (validated): Patterns hold, may not need full 80
- ❌ If energy efficiency varies widely: Expand to more operations

### Additional Validation

1. **Test on Mac Mini M4**: Lower base power than MacBook
2. **Measure HPC cluster**: Enable direct comparison for 300× claim
3. **Test on real FASTQ data**: Validate synthetic results

---

**Generated**: November 2, 2025
**Data source**: power_enriched_20251102_184235.csv
