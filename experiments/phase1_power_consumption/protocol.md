# Phase 1: Power Consumption Dimension - Experimental Protocol

**Date Created**: November 2, 2025
**Status**: Ready for execution
**Estimated Runtime**: 4 hours (unattended)
**Lab Notebook Entry**: 20251102-020-EXPERIMENT-power-consumption-pilot.md

---

## Objectives

### Primary Research Question

**Does ARM NEON + Parallel optimization reduce total energy consumption proportionally to runtime reduction?**

### Environmental Pillar Validation

Validate the claim: **"300× less energy per analysis (0.5 Wh vs 150 Wh)"**

### Key Metrics

1. **Absolute energy consumption**: Wh per 1M sequences
2. **Energy efficiency**: Energy savings vs time savings ratio
3. **Power draw**: Watts during active computation
4. **Environmental impact**: CO₂ savings extrapolation

---

## Experimental Design

### Operations (3 operations, focused pilot)

| Operation | Complexity | Known NEON Speedup | Expected Energy Reduction |
|-----------|------------|-------------------|--------------------------|
| base_counting | 0.40 | 45× | ~30-45× |
| gc_content | 0.32 | 43× | ~30-43× |
| quality_aggregation | 0.50 | 8× | ~5-10× |

**Rationale**:
- `base_counting` and `gc_content`: Best NEON speedups, highest impact
- `quality_aggregation`: Medium speedup, tests if pattern holds across complexity spectrum

### Configurations (4 configs per operation)

| Config | Description | Expected Power Draw | Expected Speedup |
|--------|-------------|-------------------|------------------|
| Naive | Scalar, 1 thread | 8-12W (baseline) | 1× |
| NEON | Vectorized, 1 thread | 10-15W | 30-45× |
| NEON+4t | Vectorized, 4 threads | 18-25W | 100-180× |
| NEON+8t | Vectorized, 8 threads | 25-35W | 150-250× |

### Scales (2 scales)

- **Medium**: 10,000 sequences (~3 MB)
- **Large**: 100,000 sequences (~30 MB)

**Rationale**:
- 10K represents small batch analysis
- 100K represents typical WGS QC scale
- Both scales show stable NEON + Parallel speedups

### Total Experiments

```
3 operations × 4 configs × 2 scales = 24 experiments
```

Each experiment runs for **60 seconds** (loop operation continuously for stable power readings).

**Total execution time**: ~30 minutes of active computation + overhead = **4 hours with warmup/cooldown**

---

## Measurement Equipment

### Primary: macOS powermetrics

**Command**:
```bash
sudo powermetrics --samplers cpu_power --sample-rate 100 -n 600 > powermetrics_log.txt
```

**Captures**:
- CPU package power (mW)
- Per-core power distribution
- GPU power (if active)
- ANE power (if active)
- Sampling rate: 100ms (10 samples/second)

**Advantages**:
- Automated logging with timestamps
- Precise CPU package power
- No manual intervention required

**Limitations**:
- Does not capture display power
- Does not capture peripheral power
- Requires sudo access

### Secondary: Kill-A-Watt P3 Meter

**Purpose**: Validation of powermetrics data

**Measurement**:
- Total wall power (W)
- Includes: CPU + GPU + display + system overhead
- Manual reading every 30 minutes
- Photo documentation

**Advantages**:
- Complete system power
- Independent validation
- Simple to use

**Limitations**:
- Manual readings (not automated)
- Lower precision than powermetrics
- Includes non-compute power (display, etc.)

---

## Measurement Protocol

### System Preparation (30 minutes)

1. **Hardware setup**:
   ```bash
   # Connect Kill-A-Watt meter between Mac charger and wall outlet
   # Verify meter displays power reading (watts)
   ```

2. **Software preparation**:
   ```bash
   # Run preparation script
   cd /Users/scotthandley/Code/apple-silicon-bio-bench
   ./scripts/prepare_for_power_test.sh
   ```

   **Script actions**:
   - Disable Time Machine: `sudo tmutil disable`
   - Disable Spotlight: `sudo mdutil -a -i off`
   - Kill background apps: iCloud sync, etc.
   - Set display brightness to minimum
   - Close all apps except Terminal

3. **Idle baseline** (15 minutes):
   ```bash
   # Let system settle to true idle
   # Record idle power from Kill-A-Watt
   # Take photo of meter display
   # Note: idle_watts = X.X W
   ```

### Execution (4 hours, unattended)

1. **Start power pilot**:
   ```bash
   ./scripts/run_power_pilot.sh
   ```

   **Script actions**:
   - Starts powermetrics in background
   - Runs power pilot binary (24 experiments)
   - Logs all results to CSV with timestamps
   - Stops powermetrics when complete

2. **User leaves to work on other computer**

3. **Periodic validation** (every 30 minutes):
   - User returns briefly
   - Takes photo of Kill-A-Watt display
   - Notes timestamp on phone
   - Returns to other computer

4. **Automatic completion**:
   - Pilot completes all 24 experiments
   - Results saved to CSV
   - powermetrics log saved
   - System remains idle

### Post-Execution Analysis (2 hours)

1. **Parse powermetrics logs**:
   ```bash
   python analysis/parse_powermetrics.py
   ```

2. **Calculate energy metrics**:
   - Energy per sequence
   - Energy efficiency vs naive
   - Power draw by configuration

3. **Generate findings document**:
   ```bash
   python analysis/generate_power_findings.py
   ```

---

## Implementation Details

### Power Pilot Binary

**Location**: `crates/asbb-cli/src/pilot_power.rs`

**Execution flow for each experiment**:
```rust
fn run_experiment(operation: &str, config: HardwareConfig, scale: Scale) {
    // 1. Load data
    let data = load_dataset(scale);

    // 2. Warmup (3 iterations, discard)
    for _ in 0..3 {
        execute_operation(operation, &data, config);
    }

    // 3. Start timing
    let start = Instant::now();

    // 4. Loop for 60 seconds (stable power measurement)
    let mut iterations = 0;
    while start.elapsed() < Duration::from_secs(60) {
        execute_operation(operation, &data, config);
        iterations += 1;
    }
    let duration = start.elapsed();

    // 5. Calculate metrics
    let sequences_processed = data.len() * iterations;
    let throughput = sequences_processed as f64 / duration.as_secs_f64();

    // 6. Log to CSV
    log_result(operation, config, scale, duration, sequences_processed, throughput);
}
```

### powermetrics Integration

**Parallel execution**:
- Main script starts powermetrics in background
- Power pilot runs experiments
- powermetrics logs continuously
- Kill powermetrics when pilot completes

**Log correlation**:
- CSV contains experiment timestamps
- powermetrics log contains power + timestamps
- Analysis script correlates: "What was power draw during experiment X?"

### Energy Calculation

**From powermetrics**:
```python
# Parse powermetrics log
samples = parse_powermetrics_log("powermetrics.txt")

# Find samples during experiment window
experiment_start = csv_row['timestamp']
experiment_duration = csv_row['loop_duration_s']
experiment_end = experiment_start + experiment_duration

power_samples = [s for s in samples
                 if experiment_start <= s.timestamp <= experiment_end]

# Calculate average power during experiment
avg_power_mw = sum(s.cpu_power_mw for s in power_samples) / len(power_samples)
avg_power_w = avg_power_mw / 1000.0

# Calculate energy
energy_wh = avg_power_w * (experiment_duration / 3600.0)
energy_per_seq_uwh = (energy_wh * 1e6) / sequences_processed
```

**From Kill-A-Watt** (validation):
```python
# Manual readings from photos
idle_watts = 8.5  # From idle baseline photo
active_watts = 22.3  # From photo during experiment
net_watts = active_watts - idle_watts

# Estimate energy (less precise)
energy_wh_estimate = net_watts * (experiment_duration / 3600.0)
```

---

## Output Format

### CSV Structure

**File**: `results/phase1_power_consumption/power_pilot_raw_TIMESTAMP.csv`

**Header**:
```csv
operation,config,scale,num_sequences,loop_duration_s,iterations,sequences_processed,throughput_seqs_per_sec,timestamp
```

**Example rows**:
```csv
base_counting,naive,Medium,10000,60.0,245,2450000,40833.33,2025-11-02T14:30:00
base_counting,neon,Medium,10000,60.0,11025,110250000,1837500.0,2025-11-02T14:32:15
base_counting,neon_4t,Medium,10000,60.0,44100,441000000,7350000.0,2025-11-02T14:34:30
base_counting,neon_8t,Medium,10000,60.0,66150,661500000,11025000.0,2025-11-02T14:36:45
```

### powermetrics Log

**File**: `results/phase1_power_consumption/powermetrics_TIMESTAMP.txt`

**Sample output**:
```
*** Sampled system activity (Sat Nov  2 14:30:05 2025 -0700) (100.50ms elapsed) ***

...

CPU Power: 12450 mW
GPU Power: 0 mW
ANE Power: 0 mW

...
```

**Parsing**: Extract timestamp + CPU power for correlation with CSV.

---

## Analysis Plan

### 1. Energy Per Sequence

For each experiment:
```python
energy_per_seq[operation][config][scale] = energy_wh / sequences_processed
```

**Expected pattern**:
- Naive: Highest energy per sequence (baseline)
- NEON: ~30× less energy per sequence
- NEON+4t: ~100× less energy per sequence
- NEON+8t: ~150× less energy per sequence

### 2. Energy Efficiency Metric

```python
time_reduction = time_naive / time_optimized
energy_reduction = energy_naive / energy_optimized
energy_efficiency = time_reduction / energy_reduction
```

**Interpretation**:
- Efficiency = 1.0: Energy scales with time (ideal)
- Efficiency > 1.0: Better than expected (bonus!)
- Efficiency < 1.0: Power-hungry optimization

**Expected**: ~0.9-1.1 (energy scales with time, minor overhead)

### 3. Power Draw Analysis

```python
power_draw[config] = avg_power_w during execution
```

**Questions**:
- Does NEON increase power draw? (Expected: slightly, +20-30%)
- Does parallelism increase power draw? (Expected: yes, +100-200%)
- Is power draw proportional to speedup? (Expected: no, sub-linear)

### 4. Environmental Impact

**Per-lab annual savings**:
```python
analyses_per_year = 10_000
energy_saved_per_analysis_wh = energy_naive - energy_optimized
annual_energy_saved_kwh = (energy_saved * analyses_per_year) / 1000.0
annual_co2_saved_kg = annual_energy_saved_kwh * 0.5  # 0.5 kg CO₂/kWh
```

**Field-wide impact** (10,000 labs):
```python
field_energy_saved_kwh = annual_energy_saved_kwh * 10_000
field_co2_saved_tons = (field_energy_saved_kwh * 0.5) / 1000.0
```

---

## Expected Outcomes

### Hypothesis 1: Energy Scales with Runtime

**Test**: Compare time speedup vs energy speedup

**Expected**: Energy speedup ≈ time speedup (efficiency ≈ 1.0)

**Example** (base_counting, Large scale):
- Naive: 450ms, 0.001 Wh
- NEON: 10ms, 0.000022 Wh
- Time speedup: 45×
- Energy speedup: 45×
- Efficiency: 1.0 ✅

### Hypothesis 2: NEON Increases Power Draw Slightly

**Test**: Compare power draw: naive vs NEON (single-threaded)

**Expected**: NEON increases power draw 20-40% (more CPU activity)

**Example**:
- Naive: 8W average
- NEON: 11W average (+38%)
- But 45× faster → net energy reduction

### Hypothesis 3: Parallel Increases Power Draw Proportionally

**Test**: Compare power draw: 1t vs 4t vs 8t

**Expected**: Power draw increases sub-linearly with threads

**Example**:
- 1t: 11W
- 4t: 25W (+127%, not 4× = 300%)
- 8t: 35W (+218%, not 8× = 700%)

### Hypothesis 4: Validation of 300× Claim

**Current claim**: 150 Wh (HPC naive) → 0.5 Wh (Mac optimized) = 300× reduction

**Test**: Measure actual energy for base_counting WGS QC workload (10M reads)

**Expected**:
- Mac naive: 0.01 Wh per 10M reads
- Mac optimized: 0.0002 Wh per 10M reads
- Reduction: 50× (not 300×)

**Conclusion**: 300× claim likely compares:
- HPC at full power (300W continuous) for 30 minutes = 150 Wh
- Mac optimized (30W peak) for 1 minute = 0.5 Wh
- **Different hardware + different optimization = 300×**

**Our data**: Mac-to-Mac comparison, expect ~30-50× energy reduction

---

## Success Criteria

✅ **Complete** when:

1. All 24 experiments executed without errors
2. CSV file contains complete results
3. powermetrics log captured for all experiments
4. At least 8 Kill-A-Watt validation photos collected
5. Energy per sequence calculated for all configs
6. Energy efficiency metric computed (expected: 0.9-1.1)
7. Power draw by configuration documented
8. Environmental impact extrapolated
9. `FINDINGS.md` published
10. Lab notebook entry updated
11. Decision: Expand to 80 experiments or 24 sufficient?

---

## Reproducibility

### Version Information

- **ASBB version**: 0.1.0
- **Protocol version**: 1.0
- **Date**: November 2, 2025
- **Hardware**: M4 MacBook Air (24GB RAM, 10 cores)

### Software Requirements

- macOS 14.0+ (Apple Silicon)
- Rust 1.70+
- Python 3.8+ (for analysis scripts)
- sudo access (for powermetrics)

### Data Provenance

- **Datasets**: `datasets/medium_10k_150bp.fq`, `datasets/large_100k_150bp.fq`
- **Operations**: Validated in prior experiments (Entries 002-018)

---

## Notes and Limitations

### Limitations

1. **Single hardware platform**: M4 MacBook Air only (need M1/M2/M3 validation)
2. **Kill-A-Watt includes non-compute**: Display, peripherals, system overhead
3. **No HPC comparison**: Mac-to-Mac comparison only (need separate HPC measurement)
4. **Synthetic data**: Real FASTQ may have different power characteristics

### Mitigation

1. Display at minimum brightness (minimize non-compute power)
2. powermetrics provides CPU-only power (cleaner metric)
3. Future work: Measure HPC cluster for direct comparison
4. Future work: Test on real FASTQ data

### Future Experiments

If 24 experiments validate patterns:
- **Expand to 80 experiments**: 10 operations, full coverage
- **Test on Mac Mini M4**: Lower base power than MacBook
- **Test on Mac Studio M4 Max**: Higher power, more cores
- **Measure HPC cluster**: Enable direct comparison

---

**Status**: Ready to execute
**Created**: November 2, 2025
**Estimated completion**: +6 hours (prep + execution + analysis)
