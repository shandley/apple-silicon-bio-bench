# Cross-Platform Validation Protocol: AWS Graviton

**Experiment**: Portability Pillar Validation
**Lab Notebook**: Entry 021
**Date**: November 2, 2025

---

## Objective

Validate that ARM NEON optimization rules transfer from Apple Silicon (Mac M4) to AWS Graviton 3, proving true ARM ecosystem portability.

---

## Experimental Design

### Hardware Platforms

**Platform 1: Mac M4 MacBook Air** (baseline, already measured)
- Processor: Apple M4 (ARM, 10 cores: 4P + 6E)
- RAM: 24 GB unified memory
- OS: macOS Sequoia 15.x
- Data source: Power pilot results (Entry 020)

**Platform 2: AWS Graviton 3** (validation target)
- Instance: c7g.xlarge
- Processor: AWS Graviton 3 (ARM Neoverse V1, ARMv8.4-A)
- vCPUs: 4
- RAM: 8 GB DDR5
- OS: Amazon Linux 2023 (ARM64)
- Region: us-east-1
- Cost: $0.145/hour

### Operations Under Test

**5 operations across complexity spectrum**:

1. **base_counting** (complexity 0.40)
   - Mac NEON speedup: 15.3× (Medium), 15.3× (Large)
   - Expected Graviton: 12-18×

2. **gc_content** (complexity 0.32)
   - Mac NEON speedup: 17.1× (Medium), 15.1× (Large)
   - Expected Graviton: 14-20×

3. **quality_aggregation** (complexity 0.50)
   - Mac NEON speedup: 20.3× (Medium), 12.7× (Large)
   - Expected Graviton: 10-15×

4. **adapter_trimming** (complexity 0.45)
   - Mac NEON speedup: ~20-30× (estimated from complexity)
   - Expected Graviton: 16-36×

5. **kmer_counting** (complexity 0.55)
   - Mac NEON speedup: ~5-10× (estimated)
   - Expected Graviton: 4-12×

### Configurations

**3 configurations per operation**:

1. **Naive**: Scalar implementation, single-threaded
2. **NEON**: ARM NEON SIMD, single-threaded
3. **NEON+4t**: ARM NEON SIMD, 4 threads (matches Graviton vCPUs)

**Note**: Mac has 10 cores, Graviton has 4 vCPUs. Parallel speedups will differ due to thread count, but single-threaded NEON speedup should be similar.

### Scales

**3 data scales**:

1. **Small**: 1,000 sequences (~300 KB)
2. **Medium**: 10,000 sequences (~3 MB)
3. **Large**: 100,000 sequences (~30 MB)

**Total experiments**: 5 ops × 3 configs × 3 scales = **45 experiments**

---

## Measurement Protocol

### Graviton Experiments

**For each experiment**:
1. Generate synthetic sequences on-instance (no download needed)
2. Loop operation for 60 seconds (consistent with power pilot)
3. Record:
   - Iterations completed
   - Sequences processed
   - Throughput (sequences/second)
4. Calculate speedup vs naive baseline

**Data format**:
```csv
operation,config,scale,num_sequences,loop_duration_s,
iterations,sequences_processed,throughput_seqs_per_sec,
timestamp
```

### Mac Baseline Extraction

**Source**: Power pilot results (Entry 020)
- File: `results/phase1_power_consumption/power_pilot_raw_20251102_184235.csv`
- Extract matching experiments:
  - Operations: base_counting, gc_content, quality_aggregation
  - Configs: naive, neon, neon_4t
  - Scales: Medium (10K), Large (100K)

**For adapter_trimming, kmer_counting**:
- New Mac experiments needed (not in power pilot)
- Run locally before Graviton launch
- Same protocol: 60-second loops

### Cross-Platform Comparison

**For each matched experiment**:
```python
# Single-threaded NEON portability
portability_ratio = graviton_neon_speedup / mac_neon_speedup

# Expected: 0.8 - 1.2 (within ±20%)
# Success: Consistent pattern across operations
# Failure: Random variance or systematic platform difference
```

**Analysis dimensions**:
1. **Speedup correlation**: Do same operations benefit most on both platforms?
2. **Absolute variance**: What's the magnitude of platform difference?
3. **Pattern consistency**: Are optimization rules transferable?
4. **Threading behavior**: How do 4 threads on Graviton compare to Mac?

---

## AWS Infrastructure Setup

### Instance Configuration

**Launch parameters**:
```bash
aws ec2 run-instances \
  --image-id ami-0c55b159cbfafe1f0 \  # Amazon Linux 2023 ARM64
  --instance-type c7g.xlarge \
  --key-name asbb-graviton-key \
  --security-group-ids sg-XXXXXXXX \
  --subnet-id subnet-XXXXXXXX \
  --tag-specifications 'ResourceType=instance,Tags=[{Key=Name,Value=asbb-graviton-pilot}]'
```

**Security group**: SSH only (port 22)

**Key pair**: Generated via AWS CLI or console

### Software Dependencies

**Install on Graviton instance**:
```bash
# Update system
sudo yum update -y

# Install development tools
sudo yum install -y gcc git

# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
source ~/.cargo/env

# Clone ASBB repository
git clone <repo-url> asbb
cd asbb

# Compile pilot binary
cd crates/asbb-cli
cargo build --release --bin asbb-pilot-graviton
```

### Data Generation

**Synthetic sequences** (generated on-instance):
- No downloads from SRA/ENA (avoids data transfer costs)
- Same `generate_synthetic_sequences()` function
- Quality scores: Random distribution Q20-Q40
- Sequence lengths: 150 bp (Illumina-like)

---

## Automation Workflow

### Phase 1: Local Setup

**scripts/graviton_launch.sh**:
- Create SSH key pair if needed
- Create security group if needed
- Launch c7g.xlarge instance
- Wait for instance to be running
- Output: instance-id, public-ip

**scripts/graviton_setup.sh**:
- SSH to instance
- Install dependencies (Rust, git)
- Clone ASBB repository
- Compile pilot binary
- Verify compilation success

### Phase 2: Experiment Execution

**scripts/graviton_run.sh**:
- Transfer experiment script to instance
- Execute 45 experiments via SSH
- Save results to CSV on instance
- Monitor progress (print status updates)

### Phase 3: Results Collection

**scripts/graviton_download.sh**:
- SCP results CSV from instance to local
- Save to `results/cross_platform_graviton/`
- Verify file integrity

### Phase 4: Cleanup

**scripts/graviton_terminate.sh**:
- Terminate instance
- Verify termination (stop billing)
- Delete temporary resources

---

## Analysis Pipeline

### Step 1: Mac Baseline Extraction

**analysis/extract_mac_baseline.py**:
```python
# Extract matching experiments from power pilot
# Operations: base_counting, gc_content, quality_aggregation
# Configs: naive, neon, neon_4t
# Scales: Medium, Large
# Output: mac_baseline.csv
```

### Step 2: Cross-Platform Comparison

**analysis/compare_mac_graviton.py**:
```python
# Join Mac and Graviton results
# Calculate portability ratios
# Compute speedup correlations
# Output: mac_vs_graviton_comparison.csv
```

### Step 3: Findings Generation

**analysis/generate_graviton_findings.py**:
```python
# Generate FINDINGS.md
# Tables: Speedup comparison by operation
# Charts: Portability ratios
# Analysis: Pattern consistency
# Verdict: Portability validated or not?
```

---

## Success Criteria

### Primary Validation

✅ **NEON portability validated** if:
- Graviton NEON speedup: 80-120% of Mac NEON speedup (portability ratio 0.8-1.2)
- Pattern consistency: Same operations benefit most on both platforms
- Statistical significance: Correlation coefficient >0.8

### Secondary Observations

**Expected findings**:
- Absolute throughput may differ (clock speed, cache hierarchy)
- Parallel speedups will differ (4 vCPUs vs 10 cores)
- Memory-bound operations may show platform differences

**Acceptable variance**:
- ±20% speedup variance (0.8-1.2× ratio)
- Platform-specific outliers (1-2 operations)
- Threading differences (different core counts)

**Unacceptable variance**:
- Random speedup patterns (no correlation)
- Systematic platform failure (NEON doesn't work)
- Opposite optimization rules (what works on Mac fails on Graviton)

---

## Cost Control

### Estimated Costs

**Instance time**:
- Setup: 15 min = $0.04
- Experiments: 60 min = $0.15
- Buffer: 15 min = $0.04
- Total: 90 min = $0.23

**Data transfer**:
- Code upload: <1 MB (negligible)
- Results download: ~50 KB (negligible)
- Total: $0.00

**Storage**: $0.00 (ephemeral instance storage)

**Grand total**: ~$0.25 (round to $1.00 with safety margin)

### Cost Safeguards

1. **Automated termination**: Script terminates instance when done
2. **Timeout protection**: Maximum 3-hour runtime limit
3. **Manual override**: User can terminate via AWS console anytime
4. **Monitoring**: Print instance ID at launch (for manual cleanup if needed)

---

## Expected Outcomes

### Portability Validation

**Hypothesis**: ARM NEON speedups transfer Mac → Graviton with ±20% variance

**Expected results**:

| Operation | Mac NEON Speedup | Graviton NEON Speedup | Portability Ratio |
|-----------|------------------|----------------------|-------------------|
| base_counting | 15.3× | 12-18× | 0.8-1.2 |
| gc_content | 17.1× | 14-20× | 0.8-1.2 |
| quality_aggregation | 20.3× | 16-24× | 0.8-1.2 |
| adapter_trimming | 25× | 20-30× | 0.8-1.2 |
| kmer_counting | 7× | 6-8× | 0.8-1.2 |

**Pattern**: Consistent NEON benefit across platforms (proves portability)

### Platform Differences

**Factors that may differ**:
1. **Absolute throughput**: Clock speed, memory bandwidth
2. **Parallel scaling**: 4 vCPUs vs 10 cores
3. **Cache effects**: Different L1/L2/L3 sizes
4. **Memory latency**: DDR5 vs unified memory

**Should NOT differ**:
1. **NEON effectiveness pattern**: Which operations benefit most
2. **Optimization rule applicability**: Same rules apply
3. **Algorithm behavior**: Same complexity characteristics

---

## Deliverables

**Results files**:
1. `results/cross_platform_graviton/graviton_raw_TIMESTAMP.csv` - Graviton experiment data
2. `results/cross_platform_graviton/mac_baseline.csv` - Extracted Mac data
3. `results/cross_platform_graviton/mac_vs_graviton_comparison.csv` - Cross-platform analysis
4. `results/cross_platform_graviton/FINDINGS.md` - Portability validation report

**Scripts**:
1. `scripts/graviton_launch.sh`
2. `scripts/graviton_setup.sh`
3. `scripts/graviton_run.sh`
4. `scripts/graviton_download.sh`
5. `scripts/graviton_terminate.sh`

**Analysis**:
1. `analysis/extract_mac_baseline.py`
2. `analysis/compare_mac_graviton.py`
3. `analysis/generate_graviton_findings.py`

**Documentation**:
1. Lab notebook entry (Entry 021) - updated with results
2. This protocol document

---

## Timeline

**Total duration**: 2-3 hours

**Phase breakdown**:
- Local setup: 30 min (scripts creation, testing)
- AWS launch: 5 min
- Instance setup: 15 min
- Experiment execution: 45-60 min
- Results download: 5 min
- Instance termination: 2 min
- Analysis: 30-60 min (local)
- FINDINGS generation: 15 min

**AWS billable time**: ~90 min = $0.22

---

## Troubleshooting

### Instance Launch Issues

**Problem**: Instance fails to launch
- **Check**: AWS quota limits (c7g.xlarge availability)
- **Solution**: Try different region or instance type

**Problem**: SSH connection refused
- **Check**: Security group rules (port 22 open)
- **Solution**: Update security group or create new one

### Compilation Issues

**Problem**: Cargo build fails
- **Check**: Rust toolchain installation
- **Solution**: Reinstall Rust, verify ARM64 target

**Problem**: Missing dependencies
- **Check**: gcc, git installed
- **Solution**: `sudo yum install -y gcc git`

### Experiment Issues

**Problem**: Out of memory during experiments
- **Check**: 8GB RAM limit on c7g.xlarge
- **Solution**: Reduce scale or use c7g.2xlarge (16GB)

**Problem**: Slow execution
- **Check**: 4 vCPUs may be slower than Mac
- **Solution**: Expected, document in FINDINGS

---

**Created**: November 2, 2025
**Ready for execution**: After AWS credentials provided
**Expected completion**: +3 hours from launch
