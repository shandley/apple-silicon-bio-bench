# AMX Matrix Engine Dimension Pilot

**Date**: November 2, 2025
**Status**: Planning
**Pilot #**: 5/9

---

## Objective

Systematically explore the Apple Matrix Extension (AMX) coprocessor for bioinformatics sequence operations to determine:
1. Which operations benefit from matrix reformulation
2. Performance characteristics vs NEON/CPU
3. Optimal data scales for AMX utilization
4. Integration with other optimizations (NEON, parallel, etc.)

---

## Background: AMX on Apple Silicon

### What is AMX?

**Apple Matrix Extension (AMX)**:
- 512-bit wide matrix coprocessor
- Integrated with CPU cores (not discrete like GPU)
- Designed for ML/AI workloads (Core ML uses it)
- Available on M1 and later
- Operates on matrix tiles (typically 16×16 or 32×32)

**Key characteristics**:
- **Low latency**: Direct CPU integration (vs GPU dispatch overhead)
- **High throughput**: 512-bit operations (4× wider than NEON's 128-bit)
- **Matrix-native**: Optimized for matrix multiply, matrix accumulate
- **Zero-copy**: Shares CPU memory (like NEON, unlike discrete GPU)

### Novel Opportunity

**Traditional bioinformatics**: AMX didn't exist pre-2020, so no existing tools exploit it

**Apple Silicon opportunity**: Reformulate sequence operations as matrix operations
- Dynamic programming (edit distance, alignment) → matrix fill operations
- Position weight matrices → native matrix ops
- Batch operations → matrix representations

---

## Research Questions

1. **Which operations benefit from AMX?**
   - Do matrix-native algorithms (edit distance, alignment) see speedups?
   - Can we reformulate non-obvious operations as matrices?

2. **AMX vs NEON trade-offs?**
   - When does 512-bit AMX outperform 128-bit NEON?
   - Is there overhead from matrix tile setup?

3. **Data scale thresholds?**
   - Minimum sequence count for AMX to be worthwhile?
   - Does sequence length affect AMX benefit?

4. **Composition with other optimizations?**
   - Can we combine AMX + NEON?
   - Does parallel + AMX compose well?

---

## Operation Selection

### Primary Candidates (Matrix-Native)

**1. Edit Distance** (complexity: 0.70)
- **Algorithm**: Wagner-Fischer dynamic programming
- **Matrix formulation**: Fill (n+1) × (m+1) matrix
- **AMX opportunity**: Matrix fill operations, accumulation
- **Expected speedup**: High (matrix-native algorithm)

**2. Hamming Distance** (complexity: 0.35)
- **Algorithm**: XOR + popcount
- **Matrix formulation**: Bitwise matrix operations
- **AMX opportunity**: Parallel comparison matrices
- **Expected speedup**: Moderate (can vectorize with NEON too)

**3. Quality Statistics** (complexity: 0.38)
- **Algorithm**: Per-position mean/median/quartiles
- **Matrix formulation**: Sequence × position matrix
- **AMX opportunity**: Column-wise statistics on matrix
- **Expected speedup**: Moderate to high

### Secondary Candidates (Potentially Reformulable)

**4. Complexity Score** (complexity: 0.61)
- **Matrix formulation**: Frequency matrix → entropy calculation
- **AMX opportunity**: Matrix accumulation for histogram

**5. K-mer Counting** (complexity: 0.45)
- **Matrix formulation**: K-mer × occurrence count matrix
- **AMX opportunity**: Sparse matrix operations

**6. MinHash Sketching** (complexity: 0.48)
- **Matrix formulation**: Hash function × k-mer matrix
- **AMX opportunity**: Batch hash operations as matrix

### Control Operations (Not Matrix-Amenable)

**7. Base Counting** (complexity: 0.40)
- **Purpose**: Baseline comparison (NEON is optimal)
- **Expected**: AMX overhead, no benefit

**8. Reverse Complement** (complexity: 0.45)
- **Purpose**: Control (pure element-wise, NEON optimal)
- **Expected**: AMX no benefit

---

## Experimental Design

### Configuration Matrix

**Operations**: 8 total
- 3 primary matrix-native (edit_distance, hamming_distance, quality_statistics)
- 3 secondary reformulable (complexity_score, kmer_counting, minhash_sketching)
- 2 controls (base_counting, reverse_complement)

**Hardware Configurations**: 4 total
1. **Baseline** (naive, no AMX, no NEON)
2. **AMX-only** (AMX enabled, NEON disabled)
3. **NEON-only** (AMX disabled, NEON enabled) - comparison baseline
4. **AMX + NEON** (both enabled, hybrid approach)

**Data Scales**: 6 total
- Tiny: 100 sequences (overhead measurement)
- Small: 1,000 sequences (threshold detection)
- Medium: 10,000 sequences (typical batch)
- Large: 100,000 sequences (production scale)
- Very Large: 1,000,000 sequences (stress test)
- Huge: 10,000,000 sequences (if memory permits)

**Total Experiments**: 8 operations × 4 configs × 6 scales = **192 experiments**

**Expected Duration**: 1-2 days (tractable, focused pilot)

---

## Implementation Requirements

### AMX Backend Implementation

**Required**: Implement AMX backend for matrix-amenable operations

**Approach**:
1. Use AMX intrinsics (if available in Rust)
2. Or call out to C/Swift with AMX support
3. Or use Accelerate framework (which uses AMX internally)

**Priority operations**:
1. Edit distance (Wagner-Fischer with AMX matrix fill)
2. Hamming distance (matrix formulation)
3. Quality statistics (column-wise matrix operations)

### Baseline Implementations

**Already implemented**:
- Base counting (naive + NEON)
- Reverse complement (naive + NEON)
- Complexity score (naive + NEON + GPU)
- Hamming distance (naive + NEON)

**Need AMX variants**:
- Edit distance + AMX
- Hamming distance + AMX
- Quality statistics + AMX
- (Optional) Complexity score + AMX
- (Optional) K-mer counting + AMX
- (Optional) MinHash + AMX

---

## Success Criteria

**Minimum viable pilot**:
- [ ] 3 primary operations tested (edit_distance, hamming_distance, quality_statistics)
- [ ] 4 hardware configs across 6 scales = 72 experiments
- [ ] AMX speedup >1.0× for at least one operation
- [ ] Patterns documented (when does AMX help vs hurt?)

**Ideal complete pilot**:
- [ ] 8 operations tested (primary + secondary + controls)
- [ ] 4 hardware configs across 6 scales = 192 experiments
- [ ] Novel findings: unexpected AMX benefits or limitations
- [ ] Optimization rules derived (use AMX when...)

---

## Expected Insights

### Hypotheses

**H1**: Edit distance will show significant AMX speedup (matrix-native algorithm)
- **Prediction**: 2-4× faster than NEON at large scales

**H2**: AMX will have overhead at small scales (tile setup cost)
- **Prediction**: Break-even at ~1,000 sequences

**H3**: AMX + NEON hybrid will outperform either alone
- **Prediction**: Best of both for mixed workloads

**H4**: Control operations (base counting, reverse complement) won't benefit
- **Prediction**: AMX slower than NEON (overhead dominates)

### Novel Explorations

**Question**: Can we reformulate k-mer operations as sparse matrix operations?
- Traditional: Hash table
- AMX: Sparse matrix representation?

**Question**: Can quality statistics use AMX for batch column operations?
- Traditional: Loop over positions
- AMX: Matrix column-wise reduction?

---

## Data Collection

**Metrics** (same as other pilots):
- Execution time (mean, median, std)
- Throughput (sequences/sec, MB/sec)
- Speedup vs baseline
- Speedup vs NEON (key comparison)
- Memory usage
- CPU utilization

**Output Format**: CSV
```
operation,complexity,hardware_config,scale,num_sequences,time_ms,speedup_vs_baseline,speedup_vs_neon,throughput_seqs_per_sec
```

**Results File**: `results/phase1_amx_dimension_raw_YYYYMMDD_HHMMSS.csv`

---

## Analysis Plan

**Patterns to identify**:
1. Which operations benefit from AMX? (speedup >1.0×)
2. AMX vs NEON comparison (when to choose each)
3. Scale thresholds (minimum data for AMX benefit)
4. Overhead characterization (tile setup costs)

**Visualizations**:
- AMX speedup vs operation complexity
- AMX vs NEON comparison (scatter plot)
- Speedup vs scale (line plot per operation)
- Overhead analysis (small scale performance)

**Documentation**: `results/phase1/phase1_amx_dimension_complete.md`

---

## Timeline

**Day 1**: Implementation
- Implement AMX backend for 3 primary operations
- Test correctness vs naive baseline
- Prepare experiment runner

**Day 2**: Execution
- Run 72-192 experiments (depending on implementation scope)
- Collect data, checkpoint frequently
- Monitor for crashes or anomalies

**Day 3**: Analysis
- Parse results, generate visualizations
- Identify patterns and thresholds
- Document findings
- Extract optimization rules

---

## Next Steps After Completion

1. ✅ Document AMX pilot findings
2. ✅ Update PILOT_CHECKPOINT.md (5/9 complete)
3. ⏳ Proceed to Neural Engine pilot (6/9)
4. ⏳ Eventually: Level 1/2 (only after 9/9 complete)

---

**Status**: Planning complete, ready for implementation
**Created**: November 2, 2025
