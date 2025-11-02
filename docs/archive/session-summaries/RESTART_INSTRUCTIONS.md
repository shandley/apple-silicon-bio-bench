# Restart Instructions - After Reboot

**Date**: November 1, 2025
**Purpose**: Resume Level 1/2 harness execution after system reboot

---

## Pre-Restart Checklist

**System Preparation**:
- [x] ~~Reboot Mac~~ (you're doing this now)
- [ ] After reboot, close unnecessary apps:
  - [ ] Docker (if not needed)
  - [ ] Extra browser windows
  - [ ] Warp terminals (except one)
  - [ ] Any other memory-heavy apps

**Verify Configuration**:
- [x] ~~GPU configs commented out~~ (already done)
- [x] ~~"Huge" scale commented out~~ (already done)
- [x] ~~Stale checkpoint removed~~ (already done)

**Expected Configuration**:
- Total experiments: **2,200** (down from 2,640)
- 20 operations × 22 hardware configs × 5 data scales
- Parallel workers: 8
- Max memory per worker: ~300MB (1M sequences)
- Total memory demand: ~2.4GB (manageable)

---

## Startup Commands

### Step 1: Navigate to Project
```bash
cd ~/Code/apple-silicon-bio-bench
```

### Step 2: Verify Config
```bash
# Should show 5 uncommented scales (not 6)
grep -A 3 "^\[\[datasets.scales\]\]" experiments/level1_primitives/config.toml | grep -v "^#" | grep "name ="

# Should show 22 uncommented hardware configs (not 25)
grep -c "^\[\[hardware.configs\]\]" experiments/level1_primitives/config.toml
```

**Expected output**:
- 5 scales: tiny, small, medium, large, very_large
- 22 hardware configs (GPU configs commented out)

### Step 3: Start Level 1/2 Harness
```bash
# Start in background with output logging
nohup cargo run --release -p asbb-cli --bin run-level1 > results/level1_primitives/execution_output.log 2>&1 &

# Note the PID
echo "Process started (PID: $!)"
```

### Step 4: Monitor Progress
```bash
# Watch the log in real-time
tail -f results/level1_primitives/execution_output.log

# Or check progress periodically
tail -50 results/level1_primitives/execution_output.log

# Check checkpoint (experiments completed)
cat results/level1_primitives/checkpoint.json
```

---

## What to Expect

### Startup Phase (~30 seconds)
```
🚀 Apple Silicon Bio Bench - Level 1/2 Automated Harness
======================================================================

📋 Registering operations...
   Registered 20 implemented operations:
   ...

⚙️  Loading configuration...
   Config: experiments/level1_primitives/config.toml
   ✅ Configuration loaded successfully

🔬 Starting experiment execution...

Starting experiment execution:
  Total experiments: 2200
  Already completed: 0
  Remaining: 2200
  Parallel workers: 8
```

### Execution Phase (12-16 hours)
- Progress bar will update as experiments complete
- Checkpoint saved every 100 experiments
- Log shows: "DEBUG: run_experiment called for [operation] with [config]"
- You'll see: "Generating test data (X sequences)..." frequently

### Memory Usage
- **Expected**: 6-8GB used (well within 24GB capacity)
- **Monitor with**: Activity Monitor → Memory tab
- **Swap should stay low**: <5GB is fine

### Completion
```
✅ All experiments complete!
📊 Results saved to results/level1_primitives/level1_primitives_complete.parquet
```

---

## Troubleshooting

### If Process Dies Again

**Check memory**:
```bash
# macOS: Use Activity Monitor app
# Or check swap:
sysctl vm.swapusage
```

**If swap >80%**:
- Something else is using memory
- Close more apps
- Check for runaway processes in Activity Monitor

**If error about GPU**:
- Config issue - GPU configs should be commented
- Verify: `grep "gpu" experiments/level1_primitives/config.toml`
- All GPU lines should start with `#`

### Monitor Commands

**Check if process is running**:
```bash
ps aux | grep run-level1 | grep -v grep
```

**Check completed experiments**:
```bash
cat results/level1_primitives/checkpoint.json | grep -c "exp_"
```

**Estimate time remaining**:
```bash
# If 100 experiments done in 1 hour:
# 2200 total ÷ 100/hour = 22 hours
# Adjust based on actual rate
```

---

## After Completion

### Verify Results
```bash
# Check output file exists
ls -lh results/level1_primitives/level1_primitives_complete.parquet

# Check all experiments completed
cat results/level1_primitives/checkpoint.json | grep -c "exp_"
# Should show: 2200
```

### Commit Results
```bash
git add results/level1_primitives/
git commit -m "feat: Level 1/2 complete - 2,200 experiments (5 scales)"
git push
```

### Next Steps
1. Analyze parallel dimension data (720 experiments from Oct 31)
2. Analyze Level 1/2 results (2,200 experiments)
3. Document findings
4. Begin statistical analysis or continue with remaining dimensions

---

## Quick Reference

**Monitor**: `tail -f results/level1_primitives/execution_output.log`
**Check progress**: `cat results/level1_primitives/checkpoint.json`
**Expected runtime**: 12-16 hours
**Expected memory**: 6-8GB (peak)
**Total experiments**: 2,200

---

**Status**: Ready to run after reboot
**Last Updated**: November 1, 2025
