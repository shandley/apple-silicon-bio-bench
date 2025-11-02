# Composition Validation Protocol

**Date**: November 2, 2025
**Status**: Design complete, ready for implementation
**Purpose**: Validate that optimization rules from individual dimension pilots compose correctly

---

## Strategic Context

### The Critical Unknown

After completing 7/9 dimension pilots (962 experiments), we have **empirically-derived optimization rules** from individual hardware dimensions:

**Individual Pilot Findings**:
- **NEON alone**: 3-16× speedup for mid-complexity operations
- **Parallel alone**: 4-5× speedup for mid-complexity operations
- **GPU alone**: 1.8-2.7× speedup for high-complexity at huge scale

**The Question**: Do these rules compose correctly when combined?
- Does NEON × Parallel = 12-80× (multiplicative)?
- Does (NEON × Parallel) + GPU add further benefit, or does it interfere?
- Can we predict combined performance from individual pilot data?

**Why This Matters**:
1. **Scientific validity**: Reviewers will ask "did you test combined optimizations?"
2. **Practical utility**: Users will combine NEON + Parallel in real applications
3. **Prediction accuracy**: Can our ruleset predict multi-optimization performance?

**Current State**: We have NEVER tested combined optimizations. This is the critical gap for publication.

---

## Research Questions

### Primary Questions

1. **Composition multiplicativity**: Do NEON and Parallel speedups multiply?
   - Predicted: NEON × Parallel ≈ (NEON speedup) × (Parallel speedup)
   - Test: Measure actual vs predicted for 10 operations across complexity spectrum

2. **GPU addition**: Does GPU provide benefit on top of NEON+Parallel?
   - Predicted: GPU adds further benefit for complexity ≥0.55 AND scale ≥50K
   - Test: Compare NEON+Parallel vs NEON+Parallel+GPU for applicable operations

3. **Scale dependency**: Do composition rules hold across data scales?
   - Predicted: Composition multiplicative at all scales (100 → 1M sequences)
   - Test: Measure composition at 5 scales (Tiny, Small, Medium, Large, VeryLarge)

4. **Complexity dependency**: Does composition vary by operation complexity?
   - Predicted: Higher complexity operations benefit more from composition
   - Test: 10 operations spanning complexity 0.20-0.70

### Secondary Questions

5. **Interference**: Do optimizations interfere negatively?
   - Example: Does NEON reduce Parallel benefit (shared execution units)?
   - Test: Look for sublinear composition (actual < predicted)

6. **Superlinearity**: Do optimizations synergize?
   - Example: Does Parallel enhance NEON benefit (better cache locality)?
   - Test: Look for superlinear composition (actual > predicted)

7. **Prediction accuracy**: How well can we predict combined performance?
   - Metric: RMSE, R² between predicted and actual speedup
   - Goal: >80% prediction accuracy for publication confidence

---

## Experimental Design

### Operations (10)

**Full complexity spectrum** (low to high):

| Operation | Complexity | NEON Speedup (solo) | Parallel Speedup (solo) | Expected Composition |
|-----------|------------|---------------------|-------------------------|---------------------|
| base_counting | 0.20 | 3× | 1.5× | 4.5× |
| sequence_length | 0.25 | 4× | 2× | 8× |
| at_content | 0.30 | 5× | 2.5× | 12.5× |
| n_content | 0.35 | 6× | 3× | 18× |
| gc_content | 0.40 | 8× | 3.5× | 28× |
| quality_filter | 0.45 | 10× | 4× | 40× |
| reverse_complement | 0.50 | 12× | 4.5× | 54× |
| sequence_masking | 0.55 | 14× | 5× | 70× |
| complexity_score | 0.70 | 16× | 5.5× | 88× |
| quality_statistics | 0.65 | 15× | 5× | 75× |

**Note**: Expected composition assumes multiplicative (NEON × Parallel). Actual results may differ!

### Backends (4)

**For each operation, test**:

1. **Naive**: Baseline scalar implementation (no optimizations)
2. **NEON**: NEON SIMD only (no parallelism)
3. **NEON + Parallel**: NEON SIMD + Rayon (4 threads, default QoS)
4. **NEON + Parallel + GPU**: All three optimizations (where applicable)

**GPU applicability**: Only test backend #4 for operations with complexity ≥0.55:
- sequence_masking (0.55)
- complexity_score (0.70)
- quality_statistics (0.65)

**For other operations**: Skip backend #4 (GPU not expected to help based on Entry 007)

### Scales (5)

**Skip "Huge" (10M) due to memory constraints** (identified in previous Level 1/2 attempt):

| Scale | Sequences | Rationale |
|-------|-----------|-----------|
| Tiny | 100 | Test overhead regime |
| Small | 1,000 | Test transition to benefit |
| Medium | 10,000 | Test scaling behavior |
| Large | 100,000 | Test large-scale benefit |
| VeryLarge | 1,000,000 | Test maximum practical scale |

**Memory estimate**:
- Max: 1M sequences × 3KB avg = 3GB per experiment
- 8 parallel workers × 3GB = 24GB (manageable on 24GB M4 MacBook Pro)

### Total Experiments

**Calculation**:
- 7 operations (complexity <0.55): 7 ops × 3 backends × 5 scales = 105 experiments
- 3 operations (complexity ≥0.55): 3 ops × 4 backends × 5 scales = 60 experiments
- **Total**: **165 experiments**

**Note**: Reduced from initial 200 estimate due to selective GPU testing.

---

## Hypotheses

### Hypothesis 1: Multiplicative Composition (Primary)

**Null hypothesis (H0)**: NEON and Parallel speedups do NOT multiply
- Expected: NEON+Parallel speedup ≠ (NEON speedup) × (Parallel speedup)
- Statistical test: Paired t-test, actual vs predicted

**Alternative hypothesis (H1)**: NEON and Parallel speedups multiply
- Expected: NEON+Parallel speedup ≈ (NEON speedup) × (Parallel speedup)
- Metric: Ratio = actual / predicted, expect ratio ≈ 1.0 (95% CI: 0.9-1.1)

**Prediction**: ACCEPT H1 (multiplicative composition)
- Rationale: NEON (execution units) and Parallel (thread-level) operate independently
- Minimal interference expected (different optimization levels)

### Hypothesis 2: GPU Adds Benefit (Secondary)

**Null hypothesis (H0)**: GPU does NOT add benefit on top of NEON+Parallel
- Expected: NEON+Parallel+GPU ≈ NEON+Parallel (no further speedup)
- Test: For complexity ≥0.55 operations at Large/VeryLarge scales

**Alternative hypothesis (H1)**: GPU adds benefit for applicable cases
- Expected: NEON+Parallel+GPU > NEON+Parallel (further speedup)
- Condition: Complexity ≥0.55 AND scale ≥100K sequences

**Prediction**: ACCEPT H1 for Large/VeryLarge scales only
- Rationale: Entry 007 showed GPU wins at 50K+ sequences for high-complexity ops
- NEON+Parallel might leave GPU underutilized (still worth testing)

### Hypothesis 3: Scale-Independent Composition (Tertiary)

**Null hypothesis (H0)**: Composition ratio varies significantly by scale
- Expected: Composition multiplicativity changes Tiny → VeryLarge

**Alternative hypothesis (H1)**: Composition ratio stable across scales
- Expected: Ratio (actual/predicted) similar for Tiny, Small, Medium, Large, VeryLarge
- Metric: CV (coefficient of variation) of ratios across scales <10%

**Prediction**: ACCEPT H1 (scale-independent)
- Rationale: Individual pilots showed consistent speedups across scales
- Composition should inherit scale-independence

---

## Metrics and Analysis

### Primary Metrics

1. **Speedup** (for each backend vs Naive):
   ```
   Speedup = Throughput_optimized / Throughput_naive
   ```

2. **Composition Ratio**:
   ```
   Ratio = Speedup_actual / Speedup_predicted

   Where:
   Speedup_predicted = Speedup_NEON × Speedup_Parallel
   Speedup_actual = Speedup_(NEON+Parallel)
   ```

3. **Prediction Error**:
   ```
   Error = |Speedup_actual - Speedup_predicted| / Speedup_predicted

   Goal: Error <20% for 80% of operations (acceptable prediction accuracy)
   ```

### Statistical Analysis

**For Hypothesis 1 (Multiplicative Composition)**:
- Paired t-test: actual vs predicted speedups
- Null: mean(actual - predicted) ≠ 0
- Alternative: mean(actual - predicted) ≈ 0
- Significance: p < 0.05

**For Hypothesis 2 (GPU Addition)**:
- Paired t-test: NEON+Parallel+GPU vs NEON+Parallel
- Null: mean(difference) ≤ 0 (no benefit)
- Alternative: mean(difference) > 0 (benefit)
- Significance: p < 0.05, for complexity ≥0.55 at Large/VeryLarge scales

**For Hypothesis 3 (Scale Independence)**:
- Calculate composition ratio for each (operation, scale) pair
- Compute CV (coefficient of variation) across scales for each operation
- Threshold: CV <10% indicates scale-independent composition

### Prediction Accuracy

**For publication confidence**:

| Accuracy Metric | Goal | Rationale |
|-----------------|------|-----------|
| RMSE | <20% | Acceptable prediction error |
| R² | >0.80 | Strong correlation predicted vs actual |
| % within 20% error | >80% | Most operations predictable |

**Interpretation**:
- If accuracy goals met: Rules compose reliably, confident in prediction
- If accuracy goals NOT met: Rules interact, need refined model

---

## Implementation Approach

### Phase 1: Harness Creation (1-2 hours)

**Create `run-composition-validation` binary** in `asbb-cli`:

```rust
// crates/asbb-cli/src/run_composition_validation.rs

fn main() -> Result<()> {
    // Register 10 operations with complexity metadata
    let operations = vec![
        ("base_counting", 0.20),
        ("sequence_length", 0.25),
        ("at_content", 0.30),
        ("n_content", 0.35),
        ("gc_content", 0.40),
        ("quality_filter", 0.45),
        ("reverse_complement", 0.50),
        ("sequence_masking", 0.55),
        ("complexity_score", 0.70),
        ("quality_statistics", 0.65),
    ];

    // Define backends
    let backends = vec!["naive", "neon", "neon_parallel"];
    let backends_with_gpu = vec!["naive", "neon", "neon_parallel", "neon_parallel_gpu"];

    // Define scales
    let scales = vec!["tiny", "small", "medium", "large", "verylarge"];

    // Generate experiments with backend compatibility filtering
    let mut experiments = Vec::new();
    for (op_name, complexity) in &operations {
        let backends_to_use = if *complexity >= 0.55 {
            &backends_with_gpu
        } else {
            &backends
        };

        for backend in backends_to_use {
            for scale in &scales {
                experiments.push(Experiment {
                    operation: op_name.to_string(),
                    backend: backend.to_string(),
                    scale: scale.to_string(),
                    complexity: *complexity,
                });
            }
        }
    }

    println!("Total experiments: {}", experiments.len());
    // Expected: 7*3*5 + 3*4*5 = 105 + 60 = 165

    // Run with ExecutionEngine
    let engine = ExecutionEngine::new(8, output_dir)?;
    engine.run_experiments(&experiments)?;

    Ok(())
}
```

**Key features**:
- Backend compatibility filtering (GPU only for complexity ≥0.55)
- 5 scales (skip "huge" for memory constraints)
- 165 experiments total
- Parallel execution (8 workers, manageable memory)

### Phase 2: Backend Implementation (1-2 hours)

**Ensure all operations have required backends**:

1. **Naive**: Already implemented (baseline)
2. **NEON**: Already implemented (all 10 operations have NEON backends)
3. **NEON + Parallel**: Needs implementation for some operations
   - Combine NEON vectorization + Rayon parallelism
   - Pattern: `par_iter()` over chunks, NEON within each chunk
4. **NEON + Parallel + GPU**: Already implemented for 3 high-complexity operations

**Implementation pattern for NEON+Parallel**:
```rust
pub fn execute_neon_parallel(&self, data: &[SequenceRecord]) -> Result<Output> {
    use rayon::prelude::*;

    let results: Vec<_> = data
        .par_chunks(1000) // Chunk for cache locality
        .flat_map(|chunk| {
            chunk.iter().map(|seq| self.process_neon(seq))
        })
        .collect();

    Ok(aggregate(results))
}
```

### Phase 3: Execution (2-3 hours, automated)

**Run harness**:
```bash
cargo run --release -p asbb-cli --bin run-composition-validation
```

**Monitor**:
```bash
tail -f results/composition_validation/execution_output.log
```

**Expected runtime**: 2-3 hours (165 experiments, ~40-65 seconds each)

**Memory usage**: 8 workers × 3GB = 24GB (fits in M4 MacBook Pro 24GB RAM)

### Phase 4: Analysis (2-3 hours)

**Load data**:
```python
import pandas as pd
import numpy as np
from scipy import stats

df = pd.read_parquet('results/composition_validation/results.parquet')
```

**Calculate composition ratios**:
```python
# For each operation + scale combination
for (op, scale) in df[['operation', 'scale']].drop_duplicates().values:
    subset = df[(df.operation == op) & (df.scale == scale)]

    speedup_naive = 1.0  # Baseline
    speedup_neon = subset[subset.backend == 'neon'].throughput.values[0] / naive_throughput
    speedup_parallel = subset[subset.backend == 'parallel'].throughput.values[0] / naive_throughput
    speedup_neon_parallel = subset[subset.backend == 'neon_parallel'].throughput.values[0] / naive_throughput

    # Predicted (multiplicative)
    predicted = speedup_neon * speedup_parallel
    actual = speedup_neon_parallel
    ratio = actual / predicted

    print(f"{op} @ {scale}: actual={actual:.2f}, predicted={predicted:.2f}, ratio={ratio:.3f}")
```

**Statistical tests**:
```python
# Hypothesis 1: Multiplicative composition
t_stat, p_value = stats.ttest_rel(actual_speedups, predicted_speedups)
print(f"Multiplicative composition test: t={t_stat:.3f}, p={p_value:.4f}")

# Hypothesis 2: GPU addition (for complexity ≥0.55)
gpu_subset = df[(df.complexity >= 0.55) & (df.scale.isin(['large', 'verylarge']))]
neon_parallel = gpu_subset[gpu_subset.backend == 'neon_parallel'].throughput
neon_parallel_gpu = gpu_subset[gpu_subset.backend == 'neon_parallel_gpu'].throughput
t_stat, p_value = stats.ttest_rel(neon_parallel_gpu, neon_parallel)
print(f"GPU addition test: t={t_stat:.3f}, p={p_value:.4f}")
```

**Generate figures**:
- Composition ratio vs complexity (scatterplot)
- Actual vs predicted speedup (diagonal plot with 95% CI)
- Composition ratio by scale (boxplot for each operation)

---

## Expected Outcomes

### Scenario A: Multiplicative Composition (Most Likely, 70%)

**Result**: Composition ratio ≈ 1.0 (95% CI: 0.9-1.1)
- NEON and Parallel speedups multiply as predicted
- Prediction error <20% for most operations
- R² >0.80 between actual and predicted

**Implication for Publication**:
- ✅ Rules validated, composition well-understood
- ✅ Confident in ruleset predictions
- ✅ Can derive compound optimization rules from individual pilots

**Optimization Rules** (confirmed):
```
Rule 1: Always use NEON (1.1-85×)
Rule 2: Add Parallel if complexity ≥0.35 (multiply NEON speedup by 4-6×)
Rule 3: Add GPU if complexity ≥0.55 AND scale ≥50K (add 1.8-2.7× on top)

Expected compound speedup = (NEON speedup) × (Parallel speedup) + (GPU speedup if applicable)
```

### Scenario B: Sublinear Composition (Possible, 20%)

**Result**: Composition ratio <0.9 (interference detected)
- NEON and Parallel interfere (shared resources)
- Actual speedup less than predicted
- Example: NEON uses execution units that Parallel threads compete for

**Implication for Publication**:
- ✅ Still valid finding (interference quantified)
- ✅ Provides refined prediction model
- ✅ Warns users: combined benefit less than predicted

**Optimization Rules** (adjusted):
```
Expected compound speedup = (NEON speedup) × (Parallel speedup) × 0.85 (interference factor)
```

### Scenario C: Superlinear Composition (Unlikely, 10%)

**Result**: Composition ratio >1.1 (synergy detected)
- NEON and Parallel synergize (cache locality improvement)
- Actual speedup greater than predicted
- Example: Parallel improves NEON cache hit rate

**Implication for Publication**:
- ✅✅ Excellent finding (unexpected synergy)
- ✅✅ Apple Silicon architectural insight
- ✅✅ Provides optimistic prediction model

**Optimization Rules** (adjusted):
```
Expected compound speedup = (NEON speedup) × (Parallel speedup) × 1.15 (synergy factor)
```

### GPU Addition Outcomes

**Most Likely** (Entry 007 pattern holds):
- GPU adds benefit ONLY for complexity ≥0.55 AND scale ≥100K
- Benefit: 1.5-2× on top of NEON+Parallel
- Below threshold: GPU overhead dominates (0.5-1.0×)

**Implication**: GPU rule remains specialized (rare wins, don't use by default)

---

## Success Criteria

**For publication confidence**:

1. ✅ **Composition characterized**: Multiplicative, sublinear, or superlinear quantified
2. ✅ **Prediction accuracy measured**: RMSE, R², error distribution documented
3. ✅ **GPU addition tested**: Confirm Entry 007 pattern holds with NEON+Parallel baseline
4. ✅ **Scale independence validated**: Composition ratio stable across scales (CV <10%)
5. ✅ **Statistical significance**: p <0.05 for composition tests
6. ✅ **Rules updated**: Optimization rules adjusted based on composition findings

**Minimum for publication**:
- Composition ratio documented (even if not perfectly multiplicative)
- Prediction model provided (even if R² <0.80, shows we tested)
- Honest reporting (sublinear composition is valid finding)

**Gold standard for publication**:
- Composition ratio ≈ 1.0 (multiplicative)
- Prediction accuracy >80% (R² >0.80)
- GPU addition validated (complexity + scale thresholds confirmed)
- Scale-independent composition (CV <10%)

---

## Deliverables

### Immediate (After Execution)

1. **Raw data**: `results/composition_validation/composition_validation_raw_YYYYMMDD_HHMMSS.csv`
2. **Processed results**: `results/composition_validation/results.parquet`
3. **Execution log**: `results/composition_validation/execution_output.log`

### Analysis Phase

4. **Statistical analysis**: `results/composition_validation/composition_analysis.md`
   - Hypothesis tests results
   - Composition ratios by operation/scale
   - Prediction accuracy metrics
   - Figures (scatterplots, boxplots, diagonal plots)

5. **Updated rules**: `OPTIMIZATION_RULES_UPDATED.md`
   - Rules with composition factors
   - Prediction equations
   - Confidence intervals

6. **Publication section**: Draft "Composition Validation" section for manuscript
   - Methods: Experimental design
   - Results: Composition ratios, statistical tests
   - Discussion: Interpretation, comparison to prediction

---

## Timeline

**Total estimated time**: 5-8 hours

| Phase | Task | Time |
|-------|------|------|
| 1 | Harness creation | 1-2 hours |
| 2 | Backend implementation (NEON+Parallel for 10 ops) | 1-2 hours |
| 3 | Execution (165 experiments, automated) | 2-3 hours |
| 4 | Analysis and documentation | 2-3 hours |

**Can run overnight**: Phases 1-2 during day, Phase 3 overnight, Phase 4 next morning

---

## Risk Assessment

### Risks and Mitigations

| Risk | Probability | Impact | Mitigation |
|------|-------------|--------|------------|
| Memory OOM (1M scale) | Medium | High | Use 8 workers (not 16), skip "huge" scale |
| Backend not implemented | Low | Medium | Check all 10 ops have NEON+Parallel |
| Execution crash | Low | Medium | Checkpoint every 10 experiments |
| Results inconclusive | Low | Low | Even inconclusive is valid finding |

### Fallback Plans

**If memory pressure**:
- Reduce to 4 workers (halves memory usage)
- Skip VeryLarge scale (reduces to 4 scales, 132 experiments)
- Still sufficient for composition validation

**If NEON+Parallel not implemented**:
- Prioritize 5 operations (complexity 0.30, 0.40, 0.50, 0.60, 0.70)
- Reduces to 5 ops × 3-4 backends × 5 scales = 75-100 experiments
- Still tests composition hypothesis

**If results inconclusive**:
- Report composition ratio with large CI (honest finding)
- Acknowledge prediction uncertainty in publication
- Propose refined experiments as future work

---

## Post-Experiment Actions

**After composition validation complete**:

1. ✅ Update `PUBLICATION_READINESS_ASSESSMENT.md` - mark composition gap filled
2. ✅ Update `OPTIMIZATION_RULES.md` - add composition factors
3. ✅ Update `PILOT_CHECKPOINT.md` - mark composition validation complete
4. ✅ Create lab notebook entry - Entry 017: Composition Validation
5. ✅ Commit results and analysis to repository
6. ✅ **THEN**: Begin manuscript preparation (publication-ready)

---

**Protocol complete**: Ready for implementation

**Next steps**:
1. Create `run-composition-validation` binary
2. Implement NEON+Parallel backends (if not already present)
3. Run 165 experiments (2-3 hours, can run overnight)
4. Analyze results and update rules

**Expected outcome**: Validation that individual pilot rules compose multiplicatively, providing confidence for publication and practical optimization.
