# Known Issues and Deferred Tasks

## Issue #1: Level 1/2 GPU Backend Compatibility (Nov 1, 2025)

**Status**: DEFERRED - Will run GPU experiments separately

**Problem**:
- ExecutionEngine generates naive Cartesian product of operations × hardware configs × scales
- No filtering based on backend compatibility
- Attempted to run `gc_content` with `gpu_neon_fallback` config
- Error: "GPU execution not implemented for gc_content"

**Root Cause**:
```rust
// In execution_engine.rs generate_experiments()
for operation in &implemented_ops {
    for hardware in &config.hardware.configs {
        for scale in &config.datasets.scales {
            // No compatibility check!
            experiments.push(Experiment { ... });
        }
    }
}
```

**Impact**:
- Only 1 operation supports GPU: `complexity_score`
- Config has 3 GPU hardware configs
- 19 operations × 3 GPU configs × 6 scales = **342 invalid experiments**
- Harness crashed after 26 completed experiments

**Temporary Fix** (Nov 1, 2025):
- Commented out 3 GPU hardware configs in `experiments/level1_primitives/config.toml`
- Reduces total experiments from 3,000 → 2,640
- Lost 18 valid GPU experiments (complexity_score × 3 GPU configs × 6 scales)
- Will run these 18 GPU experiments separately later

**Permanent Fix** (TODO):
Add backend compatibility filtering to `generate_experiments()`:

```rust
fn is_compatible(op: &OperationConfigEntry, hw: &HardwareConfigEntry) -> bool {
    // Check if operation supports required backend
    if hw.use_gpu && !op.backends.contains(&"gpu".to_string()) {
        return false;
    }
    // Add more checks: 2bit encoding, parallel, etc.
    true
}

fn generate_experiments(config: &ExperimentConfig) -> Result<Vec<Experiment>> {
    // ... existing code ...
    for operation in &implemented_ops {
        for hardware in &config.hardware.configs {
            if !is_compatible(operation, hardware) {
                continue;  // Skip incompatible combinations
            }
            for scale in &config.datasets.scales {
                experiments.push(Experiment { ... });
            }
        }
    }
    Ok(experiments)
}
```

**Action Items**:
- [ ] Implement backend compatibility filter in ExecutionEngine
- [ ] Run 18 GPU-only experiments for complexity_score separately
- [ ] Consider adding similar filters for: 2bit encoding, parallel requirements, etc.
- [ ] Add validation tests for experiment generation logic

**Files Involved**:
- `crates/asbb-explorer/src/execution_engine.rs` (needs fix)
- `experiments/level1_primitives/config.toml` (GPU configs commented out)
- `results/level1_primitives/checkpoint.json` (26 experiments completed before crash)

---
