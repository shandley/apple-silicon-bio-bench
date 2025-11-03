# Graviton Validation Ready 🚀

**Lab Notebook**: Entry 021
**Status**: Infrastructure Complete, Ready for AWS Credentials
**Created**: November 2, 2025

---

## What's Been Prepared

### ✅ Complete Infrastructure

1. **Lab Notebook Entry**: `lab-notebook/2025-11/20251102-021-EXPERIMENT-graviton-portability.md`
2. **Protocol Document**: `experiments/cross_platform_graviton/protocol.md`
3. **Pilot Binary**: `crates/asbb-cli/src/pilot_graviton.rs`
4. **AWS Automation Scripts**:
   - `scripts/graviton_launch.sh` - Launch c7g.xlarge instance
   - `scripts/graviton_setup.sh` - Install deps, compile binary
   - `scripts/graviton_run.sh` - Run 45 experiments
   - `scripts/graviton_download.sh` - Download results
   - `scripts/graviton_terminate.sh` - Cleanup instance
   - `scripts/graviton_full_automation.sh` - **Full automation (one command!)**

5. **Analysis Scripts**:
   - `analysis/extract_mac_baseline.py` - Extract Mac data from power pilot
   - `analysis/compare_mac_graviton.py` - Cross-platform comparison
   - `analysis/generate_graviton_findings.py` - Generate FINDINGS.md

---

## Quick Start (Fully Automated)

### Option A: One-Command Full Automation

```bash
# This runs everything: launch, setup, run, download, terminate, analyze
./scripts/graviton_full_automation.sh
```

**Duration**: ~2-3 hours
**Cost**: ~$0.30-0.45
**User interaction**: AWS credentials (first time only)

---

### Option B: Step-by-Step (Manual Control)

```bash
# 1. Configure AWS credentials (one-time)
aws configure

# 2. Launch instance (~5 min, $0.01)
./scripts/graviton_launch.sh
# → Outputs: instance-id, public-ip

# 3. Setup instance (~15 min, $0.04)
./scripts/graviton_setup.sh <public-ip>

# 4. Run experiments (~60 min, $0.15)
./scripts/graviton_run.sh <public-ip>

# 5. Download results (~1 min, $0.00)
./scripts/graviton_download.sh <public-ip>

# 6. Terminate instance (~2 min, $0.00)
./scripts/graviton_terminate.sh

# 7. Analyze results (local, free)
python3 analysis/extract_mac_baseline.py \
    results/phase1_power_consumption/power_pilot_raw_20251102_184235.csv

GRAVITON_CSV=$(ls -t results/cross_platform_graviton/graviton_raw_*.csv | head -1)

python3 analysis/compare_mac_graviton.py \
    results/cross_platform_graviton/mac_baseline.csv \
    "$GRAVITON_CSV"

python3 analysis/generate_graviton_findings.py \
    results/cross_platform_graviton/mac_vs_graviton_comparison.csv
```

---

## AWS Configuration (First Time Only)

You'll need to provide:

```bash
aws configure

# You'll be prompted for:
# - AWS Access Key ID: [your access key]
# - AWS Secret Access Key: [your secret key]
# - Default region name: us-east-1
# - Default output format: json
```

**Required IAM permissions**:
- EC2: Launch instances, terminate instances
- EC2: Create key pairs, security groups
- EC2: Describe instances, wait operations

---

## What Will Happen

### Automated Workflow

1. **Launch** (5 min):
   - Creates SSH key pair (saved to `~/.ssh/asbb-graviton-key.pem`)
   - Creates security group (allows SSH from anywhere)
   - Launches c7g.xlarge instance (Graviton 3, 4 vCPUs, 8GB RAM)
   - Waits for SSH to be ready

2. **Setup** (15 min):
   - Updates system packages
   - Installs Rust toolchain
   - Transfers ASBB code to instance
   - Compiles `asbb-pilot-graviton` binary

3. **Run** (45-60 min):
   - Runs 45 experiments:
     - 5 operations (base_counting, gc_content, quality_aggregation, adapter_trimming, kmer_counting)
     - 3 configs (naive, neon, neon_4t)
     - 3 scales (Small 1K, Medium 10K, Large 100K)
   - Each experiment: 60-second loop
   - Saves results to CSV

4. **Download** (1 min):
   - Transfers results CSV to local machine
   - Verifies file integrity (should have 46 lines)

5. **Terminate** (2 min):
   - Terminates instance (stops billing immediately)
   - Saves instance info for cost tracking

6. **Analyze** (5 min, local):
   - Extracts Mac baseline from power pilot
   - Compares Mac vs Graviton speedups
   - Calculates portability ratios
   - Generates FINDINGS.md

---

## Expected Results

### Portability Validation

**Success criteria**:
- NEON speedup on Graviton: 80-120% of Mac NEON speedup
- Portability ratio: 0.8-1.2 (within ±20%)
- Pattern consistency: Same operations benefit most on both platforms

**Example**:
```
Operation: base_counting, NEON (single-threaded)
Mac speedup:      15.3×
Graviton speedup: 13.2× (expected: 12-18×)
Portability ratio: 0.86 ✅ (within 0.8-1.2)
```

### Deliverables

**Files created**:
- `results/cross_platform_graviton/graviton_raw_TIMESTAMP.csv` - Raw Graviton data
- `results/cross_platform_graviton/mac_baseline.csv` - Mac baseline data
- `results/cross_platform_graviton/mac_vs_graviton_comparison.csv` - Cross-platform comparison
- `results/cross_platform_graviton/FINDINGS.md` - Portability validation report
- `results/cross_platform_graviton/instance_info.txt` - AWS instance details

**Lab notebook update**:
- Update Entry 021 with results
- Mark Portability pillar as ✅ in CURRENT_STATUS.md

---

## Cost Tracking

### Estimated Costs

**AWS EC2**:
- Instance type: c7g.xlarge
- Rate: $0.145/hour
- Duration: ~2 hours (setup + experiments)
- **Total**: ~$0.30

**Data transfer**:
- Code upload: <1 MB (negligible)
- Results download: ~50 KB (negligible)
- **Total**: $0.00

**Grand total**: ~$0.30 (can round to $1.00 with safety margin)

### Cost Safeguards

1. **Automated termination**: Scripts terminate instance when done
2. **Timeout protection**: Maximum 3-hour runtime
3. **Manual override**: Instance ID printed at launch (terminate via AWS console if needed)

---

## Troubleshooting

### AWS Configuration Issues

**Problem**: `aws: command not found`
- **Solution**: AWS CLI already installed at `/opt/homebrew/bin/aws`

**Problem**: `UnauthorizedOperation` error
- **Solution**: Check IAM permissions (EC2 launch, terminate)

**Problem**: `InvalidKeyPair.NotFound`
- **Solution**: Delete `~/.ssh/asbb-graviton-key.pem` and re-run launch script

### Instance Issues

**Problem**: SSH connection timeout
- **Solution**: Security group may be misconfigured. Check port 22 is open.

**Problem**: Compilation fails on instance
- **Solution**: Increase instance size to c7g.2xlarge (16GB RAM)

### Experiment Issues

**Problem**: Fewer than 45 experiments in CSV
- **Solution**: Check instance logs, may have run out of memory

**Problem**: Portability ratio outside expected range
- **Solution**: This is a valid finding! Document in FINDINGS.md

---

## Next Steps After Completion

1. **Review** `results/cross_platform_graviton/FINDINGS.md`
2. **Update** lab notebook (Entry 021) with results
3. **Update** `CURRENT_STATUS.md`:
   - Change Portability pillar from ⏳ to ✅
4. **Celebrate** 🎉 - 3 of 4 pillars validated!

**Publication status**:
- ✅ Economic pillar (validated)
- ✅ Environmental pillar (validated, power pilot)
- ✅ Portability pillar (validated, Graviton validation)
- ✅ Data Access pillar (validated, memory pilot)

**All 4 pillars validated → Ready for publication!** 🚀

---

## Manual Execution Log

If you prefer to track progress manually:

```
[ ] AWS credentials configured
[ ] Instance launched (instance-id: ________, ip: ________)
[ ] Instance setup complete
[ ] Experiments running (45-60 min)
[ ] Results downloaded
[ ] Instance terminated
[ ] Mac baseline extracted
[ ] Cross-platform comparison complete
[ ] FINDINGS.md generated
[ ] Lab notebook updated
```

---

**Status**: ✅ Ready for execution
**Created**: November 2, 2025, 8:15 PM
**Estimated completion**: +2-3 hours from start
**Waiting for**: AWS credentials from user

---

## To Begin

Choose one:

### A. Full Automation (Recommended)
```bash
./scripts/graviton_full_automation.sh
```

### B. Step-by-Step
```bash
./scripts/graviton_launch.sh
# ... follow prompts
```

**First time**: Will prompt for AWS credentials via `aws configure`

---

**Ready to validate the Portability pillar!** 🎯
