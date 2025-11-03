---
entry_id: 20251102-021-EXPERIMENT-graviton-portability
date: 2025-11-02
type: EXPERIMENT
status: complete
phase: democratization
author: Scott Handley + Claude

references:
  protocols:
    - experiments/cross_platform_graviton/protocol.md
  prior_entries:
    - 20251102-020-EXPERIMENT-power-consumption-pilot
    - 20251102-018-ANALYSIS-phase1-complete

tags:
  - portability-pillar
  - democratization
  - aws-graviton
  - cross-platform
  - arm-neon

raw_data: results/cross_platform_graviton/graviton_raw_20251103_124347.csv
---

# Lab Notebook Entry 021: AWS Graviton Portability Validation (Portability Pillar)

**Date**: November 2, 2025
**Type**: EXPERIMENT
**Status**: ✅ Complete
**Phase**: 1 (Portability Pillar Validation)
**Operations**: base_counting, gc_content, quality_aggregation
**Results**: `results/cross_platform_graviton/FINDINGS.md`

---

## Objective

Validate the **Portability pillar** claim: "ARM NEON rules work across Mac, Graviton, Ampere, RPi"

This experiment proves that optimization rules derived from Mac experiments transfer to AWS Graviton, demonstrating true ARM ecosystem portability (not Apple-specific magic).

---

## Hypothesis

**Primary Hypothesis**: ARM NEON speedups observed on Mac M4 transfer to AWS Graviton 3 with ±20% variance.

**Expected Outcome**:
- Mac NEON speedup: 20-40× for complexity 0.30-0.40 operations
- Graviton NEON speedup: Expected 16-48× (±20% variance acceptable)
- Pattern consistency: Same operations show same relative speedups

**Key Question**: Are optimization rules truly portable, or is Mac performance unique?

---

## Experimental Design

### Focused Cross-Platform Comparison

**Scope**: 45 experiments on Graviton + comparison to Mac baseline

**Operations** (5 across complexity spectrum):
- `base_counting` (complexity 0.40, Mac NEON speedup: 45×)
- `gc_content` (complexity 0.32, Mac NEON speedup: 43×)
- `quality_aggregation` (complexity 0.50, Mac NEON speedup: 8×)
- `adapter_trimming` (complexity 0.45, expected NEON speedup: 20-30×)
- `kmer_counting` (complexity 0.55, expected NEON speedup: 5-10×)

**Configurations** (3):
- Naive (scalar, single-threaded baseline)
- NEON (vectorized, single-threaded)
- NEON+4t (vectorized, 4 threads - matches c7g.xlarge vCPUs)

**Scales** (3):
- Small: 1,000 sequences (~300 KB)
- Medium: 10,000 sequences (~3 MB)
- Large: 100,000 sequences (~30 MB)

**Total**: 5 operations × 3 configs × 3 scales = **45 experiments**

### AWS Graviton Hardware

**Instance Type**: c7g.xlarge
- Processor: AWS Graviton 3 (ARM Neoverse V1, 64-bit ARMv8.4-A)
- vCPUs: 4 (vs Mac M4: 10 cores)
- RAM: 8 GB (vs Mac M4 MacBook Air: 24 GB)
- Cost: $0.145/hour
- Region: us-east-1

**Mac Baseline**: M4 MacBook Air
- Processor: Apple M4 (ARM, 10 cores: 4 P + 6 E)
- RAM: 24 GB
- Comparison basis: Power pilot results (Entry 020)

### Measurement Protocol

**Each experiment on Graviton**:
1. Loop operation for 60 seconds (consistent with power pilot)
2. Record iterations completed
3. Calculate throughput (sequences/second)
4. Compute speedup vs naive baseline

**Mac baseline data**:
- Reuse power pilot results (Entry 020)
- Extract throughput for matching operation/config/scale
- Calculate Mac speedup vs naive

**Cross-platform comparison**:
```
portability_ratio = graviton_speedup / mac_speedup

Expected: 0.8 - 1.2 (±20% variance)
Success: Consistent pattern across operations
Failure: Random variance or systematic difference
```

**Data collected**:
```csv
operation,config,scale,num_sequences,
graviton_throughput,graviton_speedup,
mac_throughput,mac_speedup,
portability_ratio,speedup_variance_pct
```

---

## Expected Outcomes

### Cross-Platform Speedup Predictions

**Base Counting (Large scale 100K sequences)**:

| Configuration | Mac Speedup | Graviton Speedup (predicted) | Portability Ratio |
|---------------|-------------|------------------------------|-------------------|
| Naive | 1.0× | 1.0× | 1.00 (baseline) |
| NEON | 15.3× | 12-18× | 0.8-1.2 |
| NEON+4t | 51.8× | 10-15× | 0.2-0.3 (fewer cores!) |

**Key Insight**: Mac has 10 cores, Graviton has 4 vCPUs, so NEON+4t will show lower absolute speedup on Graviton (limited by thread count). **This is expected and acceptable.**

**Important**: Focus on NEON (single-threaded) comparison for portability validation.

### Platform Differences

**Expected differences**:
- Thread count: Mac 10 cores vs Graviton 4 vCPUs → parallel speedups differ
- Clock speed: May differ (affects absolute throughput)
- Cache hierarchy: Different L1/L2/L3 sizes
- Memory bandwidth: Graviton DDR5 vs Mac unified memory

**Should NOT differ**:
- NEON single-threaded speedup pattern (same ARM instruction set)
- Operation complexity rankings (same algorithms)
- Relative speedup ordering (which operations benefit most)

---

## Validation of Portability Claim

**Current claim** (from DEMOCRATIZING_BIOINFORMATICS_COMPUTE.md):
```
ARM NEON rules work across Mac, Graviton, Ampere, Raspberry Pi
Code once, deploy anywhere (ARM ecosystem)
No vendor lock-in
```

**This experiment tests**:
- ✅ Do NEON speedups transfer Mac → Graviton?
- ✅ Are optimization rules portable (same code, different platform)?
- ✅ Can researchers develop on Mac, deploy to Graviton cloud?

**Success criteria**:
- NEON speedup on Graviton: 16-48× for complexity 0.30-0.40 (±20% of Mac)
- Portability ratio: 0.8-1.2 for single-threaded NEON
- Pattern consistency: Same operations benefit most on both platforms

---

## AWS Infrastructure Setup

### Automation Strategy

**Fully automated workflow**:
1. Launch c7g.xlarge instance via AWS CLI
2. SSH and install dependencies (Rust, git)
3. Clone ASBB repository
4. Compile pilot binary with ARM optimizations
5. Generate synthetic test data
6. Run 45 experiments (automated script)
7. Download results to local machine
8. Terminate instance (cost control)
9. Analyze cross-platform comparison locally
10. Generate FINDINGS.md

**Total estimated time**: 2-3 hours
**Total estimated cost**: $0.30-0.45 (can round to $1)

### AWS Resources Required

**IAM permissions needed**:
- EC2: Launch instances
- EC2: Terminate instances
- EC2: Create key pairs
- EC2: Create security groups

**Data storage**:
- Instance storage: Temporary (ephemeral)
- Local download: Results CSV (~50 KB)
- No S3 needed (results are tiny)

**Network**:
- SSH access (port 22)
- No inbound HTTP/HTTPS needed
- Outbound: git clone, cargo install (Rust crates)

---

## Execution Plan

### Phase 1: Local Preparation (30 minutes)

1. **Install AWS CLI**:
   ```bash
   brew install awscli
   aws configure  # User provides credentials
   ```

2. **Create automation scripts**:
   - `scripts/graviton_launch.sh`: Launch instance, get IP
   - `scripts/graviton_setup.sh`: Install deps, compile binary
   - `scripts/graviton_run.sh`: Run experiments on instance
   - `scripts/graviton_download.sh`: Download results
   - `scripts/graviton_terminate.sh`: Clean up instance

3. **Create pilot binary**:
   - Reuse `pilot_power.rs` structure
   - Adapt for 5 operations, 3 configs, 3 scales
   - Remove powermetrics integration (not on Linux)

### Phase 2: AWS Execution (1-2 hours, automated)

1. **Launch instance** (5 min):
   ```bash
   ./scripts/graviton_launch.sh
   # Returns: instance-id, public-ip
   ```

2. **Setup instance** (15 min):
   ```bash
   ./scripts/graviton_setup.sh <instance-ip>
   # Installs: Rust, git, compiles binary
   ```

3. **Run experiments** (45-60 min):
   ```bash
   ./scripts/graviton_run.sh <instance-ip>
   # Runs 45 experiments, saves CSV
   ```

4. **Download results** (1 min):
   ```bash
   ./scripts/graviton_download.sh <instance-ip>
   # Downloads: graviton_raw_TIMESTAMP.csv
   ```

5. **Terminate instance** (1 min):
   ```bash
   ./scripts/graviton_terminate.sh <instance-id>
   # Stops billing immediately
   ```

### Phase 3: Analysis (1 hour, local)

1. **Cross-platform comparison**:
   ```bash
   python analysis/compare_mac_graviton.py \
     results/phase1_power_consumption/power_enriched_20251102_184235.csv \
     results/cross_platform_graviton/graviton_raw_TIMESTAMP.csv
   ```

2. **Generate findings**:
   ```bash
   python analysis/generate_graviton_findings.py \
     results/cross_platform_graviton/mac_vs_graviton_comparison.csv
   ```

3. **Update lab notebook** with results

---

## Success Criteria

✅ **Complete** when:

1. All 45 Graviton experiments executed successfully
2. Mac baseline data extracted from power pilot (Entry 020)
3. Cross-platform comparison CSV generated
4. NEON speedup portability validated (0.8-1.2 ratio)
5. Pattern consistency confirmed (same operations benefit most)
6. `FINDINGS.md` published in `results/cross_platform_graviton/`
7. Lab notebook entry updated with results
8. AWS instance terminated (no ongoing costs)
9. Total AWS cost ≤ $1.00

---

## Raw Data Location

**AWS Graviton data**:
- `results/cross_platform_graviton/graviton_raw_TIMESTAMP.csv`
- `results/cross_platform_graviton/graviton_setup_log.txt`

**Mac baseline data** (from Entry 020):
- `results/phase1_power_consumption/power_pilot_raw_20251102_184235.csv`
- Subset: Matching operations/configs/scales

**Cross-platform analysis**:
- `results/cross_platform_graviton/mac_vs_graviton_comparison.csv`
- `results/cross_platform_graviton/FINDINGS.md`

---

## Notes and Observations

### Pre-Experiment Notes

- User has AWS account with necessary permissions
- Fully automated execution (minimize manual steps)
- Cost-conscious: Terminate instance immediately after completion
- Reuse power pilot infrastructure where possible

### Limitations

- Graviton has 4 vCPUs vs Mac's 10 cores (parallel speedups will differ)
- Different memory subsystems (DDR5 vs unified memory)
- Cannot measure power consumption on AWS (no powermetrics on Linux)
- Single platform (Graviton 3) - doesn't validate Ampere/RPi yet

### Future Work

If Graviton validates portability:
- Optional: Test on Raspberry Pi 5 ($80, educational use case)
- Optional: Test on Ampere Altra (bare metal ARM server)
- Optional: Azure Cobalt (Microsoft ARM VMs)
- **Priority**: Low (Graviton sufficient to prove portability)

---

## Cost Tracking

**Estimated AWS costs**:
- Instance time: 2-3 hours × $0.145/hour = $0.29-0.44
- Data transfer: Negligible (synthetic data, tiny CSV download)
- Storage: $0 (ephemeral instance storage)
- **Total**: ~$0.50 (round to $1.00 with buffer)

**Actual costs** (to be filled after completion):
- Instance launch time: TBD
- Instance termination time: TBD
- Total hours: TBD
- Total cost: TBD

---

**Status**: 🚧 In Progress
**Created**: November 2, 2025
**Estimated completion**: +3 hours (setup + execution + analysis)
**Next steps**: Install AWS CLI, create automation scripts, launch instance

---

## Results Summary (To be added after completion)

_Results will be added here after experiments complete._

---

**Prepared**: November 2, 2025, 7:45 PM
**Ready for execution**: Awaiting AWS credentials from user
