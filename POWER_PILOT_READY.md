# Power Consumption Pilot - Ready to Launch! 🔋

**Created**: November 2, 2025
**Status**: ✅ All prep work complete, ready for execution
**Lab Notebook**: Entry 020 created

---

## Summary

All implementation complete for the **focused pilot** (Option C):
- **3 operations**: base_counting, gc_content, quality_aggregation
- **4 configs**: naive, neon, neon_4t, neon_8t
- **2 scales**: Medium (10K), Large (100K)
- **Total**: 24 experiments (~30 minutes active execution + cooldown = ~1 hour)

---

## What's Been Created

### 1. Lab Notebook Entry ✅
**File**: `lab-notebook/2025-11/20251102-020-EXPERIMENT-power-consumption-pilot.md`
- Comprehensive experimental design
- Success criteria
- Environmental impact calculations
- All mandatory pre-experiment documentation

### 2. Protocol Document ✅
**File**: `experiments/phase1_power_consumption/protocol.md`
- Detailed measurement protocol
- Expected outcomes
- Analysis plan

### 3. Power Pilot Binary ✅
**File**: `crates/asbb-cli/src/pilot_power.rs`
- **Compiled and ready**: `cargo build --release -p asbb-cli --bin asbb-pilot-power` ✅
- Loops each operation for 60 seconds (stable power readings)
- Outputs CSV with timestamps for correlation

### 4. Automation Scripts ✅
All scripts are executable and ready:

**System Preparation**:
- `scripts/prepare_for_power_test.sh` - Disables background processes, sets up system

**Execution**:
- `scripts/run_power_pilot.sh` - Runs powermetrics + pilot in parallel, automates everything

**Cleanup**:
- `scripts/cleanup_after_power_test.sh` - Re-enables background processes after testing

### 5. Analysis Scripts ✅
**Python scripts** (executable):
- `analysis/parse_powermetrics.py` - Correlates powermetrics log with experiment CSV
- `analysis/generate_power_findings.py` - Generates FINDINGS.md from enriched data

---

## When You're Ready to Launch

### Quick Launch (Recommended)

```bash
# 1. Prepare system (~15 minutes)
./scripts/prepare_for_power_test.sh

# Follow prompts to:
# - Set display brightness to minimum
# - Close unnecessary apps
# - Verify Kill-A-Watt meter is connected

# 2. Let system idle for 15 minutes
# - Take photo of Kill-A-Watt meter (idle power baseline)
# - Note timestamp

# 3. Launch experiment (unattended, ~1 hour)
./scripts/run_power_pilot.sh

# Script will:
# - Start powermetrics automatically
# - Run all 24 experiments
# - Save results to timestamped CSV files
# - Stop powermetrics when complete

# 4. Periodic manual task (every 30 minutes)
# - Return briefly
# - Take photo of Kill-A-Watt display
# - Note timestamp on phone
# - Save photos to results/phase1_power_consumption/killawatt_photos/

# 5. After completion (~2 hours)
# - Re-enable background processes: ./scripts/cleanup_after_power_test.sh
# - Analyze results (see below)
```

### Analysis After Completion

```bash
# Parse powermetrics and correlate with experiments
python analysis/parse_powermetrics.py \
    results/phase1_power_consumption/powermetrics_TIMESTAMP.txt \
    results/phase1_power_consumption/power_pilot_raw_TIMESTAMP.csv

# Generate findings document
python analysis/generate_power_findings.py \
    results/phase1_power_consumption/power_enriched_TIMESTAMP.csv

# Review findings
cat results/phase1_power_consumption/FINDINGS.md
```

### Manual Launch (Step-by-Step)

If you prefer to run commands manually:

```bash
# Terminal 1: Start powermetrics
sudo powermetrics --samplers cpu_power --sample-rate 100 > results/phase1_power_consumption/powermetrics.txt

# Terminal 2: Run pilot
cargo run --release -p asbb-cli --bin asbb-pilot-power > results/phase1_power_consumption/power_pilot.csv 2> results/phase1_power_consumption/power_pilot.log

# When complete, stop powermetrics (Ctrl+C in Terminal 1)
```

---

## Important Reminders

### Before Starting
- ✅ Kill-A-Watt meter connected (Mac charger → meter → wall outlet)
- ✅ Display brightness at minimum
- ✅ All apps closed except Terminal
- ✅ You have a second computer to work on during testing
- ✅ Phone timer set for 30-minute intervals (Kill-A-Watt photos)

### During Execution
- ❌ **DO NOT use this computer** (contaminates power measurements)
- ✅ **DO return every 30 minutes** to photograph Kill-A-Watt meter
- ✅ Work on your other computer

### After Completion
- ✅ Run cleanup script to re-enable background processes
- ✅ Analyze results and generate findings
- ✅ Update lab notebook entry with results
- ✅ Decide: Expand to 80 experiments or 24 sufficient?

---

## Expected Results

### Power Efficiency
- **Energy efficiency ≈ 1.0**: Energy scales with time (ideal, expected)
- **Energy speedup ≈ Time speedup**: NEON saves energy proportionally

### Environmental Impact
- **Per-lab savings**: ~10 Wh/year for 10K analyses
- **Field-wide impact**: If 10K labs adopt, significant CO₂ reduction

### Validation of 300× Claim
- **Mac-to-Mac comparison**: Expect ~30-50× energy reduction
- **300× claim validation**: Requires separate HPC measurement for direct comparison

---

## Files and Locations

### Results Directory
```
results/phase1_power_consumption/
├── powermetrics_TIMESTAMP.txt       # Raw powermetrics log
├── power_pilot_raw_TIMESTAMP.csv    # Raw experiment results
├── power_pilot_log_TIMESTAMP.txt    # Experiment log (stderr)
├── power_enriched_TIMESTAMP.csv     # Analyzed data (after parse_powermetrics.py)
├── FINDINGS.md                       # Final findings document
└── killawatt_photos/                 # Manual Kill-A-Watt photos
    ├── killawatt_1430.jpg
    ├── killawatt_1500.jpg
    └── ...
```

### Lab Notebook
- **Entry 020**: `lab-notebook/2025-11/20251102-020-EXPERIMENT-power-consumption-pilot.md`
- **Update after completion** with findings and decision to expand/stop

### Experiment Protocol
- `experiments/phase1_power_consumption/protocol.md`

---

## Troubleshooting

### Build fails
```bash
cargo clean
cargo build --release -p asbb-cli --bin asbb-pilot-power
```

### Datasets missing
```bash
# Generate datasets
cargo run --release -p asbb-datagen -- --help
```

### powermetrics permission denied
```bash
# powermetrics requires sudo
sudo -v  # Refresh sudo credentials
```

### Python dependencies missing
```bash
# All scripts use standard library only (no dependencies)
# Ensure Python 3.8+ installed
python3 --version
```

---

## Next Steps After This Pilot

### If Energy Efficiency ≈ 1.0 (Validated)
- ✅ Patterns hold across operations
- ✅ Environmental pillar validated
- ✅ May not need full 80 experiments
- ⏭️ Move to Graviton validation (Portability pillar)

### If Energy Efficiency Varies Widely
- ⚠️ Need more data
- 🔄 Expand to 80 experiments (10 operations)
- 📊 Identify operation-specific patterns

---

## Questions or Issues?

**Lab notebook**: Entry 020 has full experimental details
**Protocol**: `experiments/phase1_power_consumption/protocol.md`
**Scripts**: All in `scripts/` directory with inline documentation

**Ready to launch whenever you are!** 🚀

---

**Last Updated**: November 2, 2025
**Status**: ✅ READY FOR EXECUTION
**Estimated Time**: 1 hour (system prep) + 1 hour (unattended execution) + 2 hours (analysis) = ~4 hours total
