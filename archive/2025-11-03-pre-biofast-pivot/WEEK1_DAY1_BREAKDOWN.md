# Week 1, Day 1: DAG Testing Harness - Detailed Breakdown

**Date**: November 4, 2025 (planned)
**Goal**: Build unified test framework for systematic hardware exploration
**Estimated Time**: 8 hours (4 morning + 4 afternoon)
**Deliverable**: `crates/asbb-cli/src/dag_traversal.rs` (~500 lines)

---

## What You're Building

### The Big Picture

You're building a **unified testing harness** that can systematically explore the hardware optimization space (the DAG) with intelligent pruning, instead of running all 23,040 possible experiments.

**Key insight**: This is similar to your existing pilot binaries (like `pilot_parallel.rs`), but **more sophisticated**:
- Existing pilots: Test specific dimension (e.g., parallel threading)
- New DAG harness: Tests combinations with pruning logic

---

## Morning Session (4 hours): Architecture & Core Abstractions

### Task 1: Design Overall Architecture (~1 hour)

**What you're creating**: The structure of how experiments flow through the system

**Key components**:
```rust
// Main entry point
pub fn run_dag_traversal(config: DAGConfig) -> Result<Vec<ExperimentResult>>

// Configuration (what to test)
struct DAGConfig {
    operations: Vec<String>,          // e.g., ["base_counting", "gc_content"]
    scales: Vec<Scale>,                // e.g., [Medium(10K), Large(100K)]
    pruning_threshold: f64,            // e.g., 1.5 (speedup threshold)
    output_path: PathBuf,              // Where to save CSV
}

// Result from a single experiment
struct ExperimentResult {
    operation: String,
    config: HardwareConfig,
    scale: String,
    throughput: f64,
    speedup_vs_naive: f64,
    pruned: bool,                      // Was this config pruned?
}
```

**Design decisions to make**:
1. How to represent hardware configs (naive, NEON, NEON+2t, etc.)?
2. How to track which configs have been tested (avoid re-running)?
3. How to represent DAG relationships (parents, children)?

---

### Task 2: Implement `DAGNode` (~1 hour)

**What it represents**: A specific hardware configuration in the optimization space

```rust
/// Represents a single node in the DAG (one hardware config)
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct DAGNode {
    /// Configuration type (naive, neon, neon_parallel, etc.)
    config_type: ConfigType,

    /// Number of threads (1 for single-threaded)
    threads: usize,

    /// Core affinity (default, p_cores, e_cores)
    affinity: CoreAffinity,

    /// Parent nodes (dependencies that must be tested first)
    parents: Vec<DAGNode>,
}

enum ConfigType {
    Naive,
    Neon,
    Gpu,
    Amx,
}

enum CoreAffinity {
    Default,
    PerformanceCores,
    EfficiencyCores,
}
```

**Key methods**:
- `fn new(config_type, threads, affinity) -> DAGNode`
- `fn with_parent(parent: DAGNode) -> DAGNode` (for compositions)
- `fn is_alternative(&self, other: &DAGNode) -> bool` (mutually exclusive?)
- `fn is_composition(&self, base: &DAGNode) -> bool` (builds on base?)

**Example usage**:
```rust
let naive = DAGNode::new(ConfigType::Naive, 1, CoreAffinity::Default);
let neon = DAGNode::new(ConfigType::Neon, 1, CoreAffinity::Default);
let neon_4t = neon.with_threads(4); // Composition
let neon_4t_p = neon_4t.with_affinity(CoreAffinity::PerformanceCores); // Refinement
```

---

### Task 3: Implement `DAGTraversal` (~1.5 hours)

**What it does**: Executes the systematic exploration with pruning

```rust
/// Executes DAG traversal with pruning
struct DAGTraversal {
    config: DAGConfig,
    tested_nodes: HashMap<DAGNode, ExperimentResult>,
    pruned_nodes: HashSet<DAGNode>,
}

impl DAGTraversal {
    /// Main entry point
    pub fn run(&mut self) -> Result<Vec<ExperimentResult>> {
        let mut all_results = Vec::new();

        for operation in &self.config.operations {
            // Phase 1: Test alternatives
            let alt_results = self.test_alternatives(operation)?;
            all_results.extend(alt_results.clone());

            // Phase 2: Test compositions (only for successful alternatives)
            let successful = self.filter_successful(&alt_results);
            let comp_results = self.test_compositions(operation, successful)?;
            all_results.extend(comp_results.clone());

            // Phase 3: Test refinements (only for optimal configs)
            let optimal = self.find_optimal(&comp_results);
            let refined_results = self.test_refinements(operation, optimal)?;
            all_results.extend(refined_results);
        }

        Ok(all_results)
    }

    /// Test alternative optimizations (NEON vs GPU vs AMX)
    fn test_alternatives(&mut self, operation: &str) -> Result<Vec<ExperimentResult>> {
        // Test: naive, NEON, GPU, AMX
        // Prune if speedup < threshold
    }

    /// Test compositions (NEON+Parallel)
    fn test_compositions(&mut self, operation: &str, bases: Vec<DAGNode>)
        -> Result<Vec<ExperimentResult>> {
        // For each successful alternative (e.g., NEON)
        // Test: base+2t, base+4t, base+8t
        // Prune if no additional benefit
    }

    /// Test refinements (core affinity)
    fn test_refinements(&mut self, operation: &str, configs: Vec<DAGNode>)
        -> Result<Vec<ExperimentResult>> {
        // For parallel configs, test P-cores vs E-cores
    }
}
```

**Key logic** (from DAG_FRAMEWORK.md):
- **Pruning rule**: If speedup < 1.5×, prune this branch and all children
- **Composition check**: Only test compositions if parent succeeded
- **Result tracking**: Don't re-test nodes we've already seen

---

### Task 4: Implement `PruningStrategy` (~30 minutes)

**What it does**: Decides whether to prune branches

```rust
struct PruningStrategy {
    speedup_threshold: f64,  // e.g., 1.5
    diminishing_returns: f64, // e.g., 1.3 (30% additional benefit required)
}

impl PruningStrategy {
    /// Should we prune this alternative?
    fn should_prune_alternative(&self, result: &ExperimentResult) -> bool {
        result.speedup_vs_naive < self.speedup_threshold
    }

    /// Should we stop testing more threads?
    fn should_prune_composition(&self, result: &ExperimentResult, parent: &ExperimentResult) -> bool {
        result.speedup_vs_naive < parent.speedup_vs_naive * self.diminishing_returns
    }
}
```

**Example**:
- NEON gets 2.0× → Keep ✅ (above 1.5× threshold)
- GPU gets 0.8× → Prune ❌ (below 1.5× threshold)
- NEON+4t gets 10× (parent NEON was 5×) → 10/5 = 2.0× → Keep ✅
- NEON+8t gets 10.5× (parent NEON+4t was 10×) → 10.5/10 = 1.05× → Prune ❌

---

## Afternoon Session (4 hours): Test Execution & Validation

### Task 5: Implement Parallel Test Runner (~1.5 hours)

**What it does**: Actually runs the experiments efficiently

```rust
/// Runs a single experiment
fn run_experiment(
    operation: &str,
    config: &DAGNode,
    scale: &Scale,
) -> Result<ExperimentResult> {
    // 1. Load operation (e.g., base_counting, gc_content)
    // 2. Load dataset for this scale
    // 3. Configure hardware (threads, affinity)
    // 4. Run operation, measure throughput
    // 5. Return result
}

/// Runs experiments in parallel (to speed up total time)
fn run_experiments_parallel(
    experiments: Vec<(String, DAGNode, Scale)>,
    max_parallel: usize,
) -> Result<Vec<ExperimentResult>> {
    use rayon::prelude::*;

    experiments
        .par_iter()
        .with_max_len(max_parallel)
        .map(|(op, config, scale)| run_experiment(op, config, scale))
        .collect()
}
```

**Key challenge**: Balance parallelism vs measurement interference
- Running 8 experiments in parallel might affect each other's performance
- Solution: Limit to 2-4 parallel at a time, or run sequentially

**Pattern to reuse**: Look at `pilot_parallel.rs` for how to:
- Set thread counts
- Configure core affinity
- Measure throughput

---

### Task 6: Progress Tracking (~30 minutes)

**What it does**: Shows user what's happening during 740 experiments

```rust
use indicatif::{ProgressBar, ProgressStyle};

fn create_progress_bar(total_experiments: usize) -> ProgressBar {
    let pb = ProgressBar::new(total_experiments as u64);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} ({eta}) {msg}")
            .unwrap()
            .progress_chars("=>-")
    );
    pb
}

// Usage:
let pb = create_progress_bar(740);
for experiment in experiments {
    let result = run_experiment(...);
    pb.set_message(format!("Testing {} with {}", op, config));
    pb.inc(1);
}
pb.finish_with_message("DAG traversal complete!");
```

---

### Task 7: Error Handling & Recovery (~30 minutes)

**What it handles**: Experiments that fail or crash

```rust
fn run_experiment_with_recovery(
    operation: &str,
    config: &DAGNode,
    scale: &Scale,
) -> Result<ExperimentResult> {
    // Try running experiment
    match run_experiment(operation, config, scale) {
        Ok(result) => Ok(result),
        Err(e) => {
            // Log error
            eprintln!("⚠️  Experiment failed: {} with {:?} at scale {}",
                     operation, config, scale.name);
            eprintln!("   Error: {}", e);

            // Return a "failed" result (don't crash entire harness)
            Ok(ExperimentResult {
                operation: operation.to_string(),
                config: config.clone(),
                scale: scale.name.to_string(),
                throughput: 0.0,
                speedup_vs_naive: 0.0,
                pruned: true, // Mark as pruned (failed)
            })
        }
    }
}
```

---

### Task 8: CSV Output (~30 minutes)

**What it does**: Save results for later analysis

```rust
use csv::Writer;

fn write_results_csv(results: &[ExperimentResult], path: &Path) -> Result<()> {
    let mut wtr = Writer::from_path(path)?;

    // Header
    wtr.write_record(&[
        "operation", "config_type", "threads", "affinity",
        "scale", "num_sequences", "throughput", "speedup", "pruned"
    ])?;

    // Data rows
    for result in results {
        wtr.write_record(&[
            &result.operation,
            &format!("{:?}", result.config.config_type),
            &result.config.threads.to_string(),
            &format!("{:?}", result.config.affinity),
            &result.scale,
            &result.num_sequences.to_string(),
            &format!("{:.2}", result.throughput),
            &format!("{:.2}", result.speedup_vs_naive),
            &result.pruned.to_string(),
        ])?;
    }

    wtr.flush()?;
    Ok(())
}
```

**Output format** (example):
```csv
operation,config_type,threads,affinity,scale,num_sequences,throughput,speedup,pruned
base_counting,Naive,1,Default,Medium,10000,1425000.0,1.00,false
base_counting,Neon,1,Default,Medium,10000,28500000.0,20.00,false
base_counting,NeonParallel,4,Default,Medium,10000,85500000.0,60.00,false
```

---

### Task 9: Validation Testing (~1.5 hours)

**What it tests**: Verify the harness works before running all 740 experiments

**Test 1: Small scale test** (2 operations, 1 scale)
```rust
#[test]
fn test_dag_traversal_small() {
    let config = DAGConfig {
        operations: vec!["base_counting".to_string(), "gc_content".to_string()],
        scales: vec![Scale::Medium],  // Just 10K sequences
        pruning_threshold: 1.5,
        output_path: PathBuf::from("test_results.csv"),
    };

    let mut traversal = DAGTraversal::new(config);
    let results = traversal.run().unwrap();

    // Should have tested: naive, NEON, NEON+2t, NEON+4t, NEON+8t
    // That's ~5 configs × 2 operations = ~10 experiments
    assert!(results.len() >= 10);
    assert!(results.len() <= 20); // With pruning
}
```

**Test 2: Pruning logic verification**
```rust
#[test]
fn test_pruning_works() {
    // Manually create a result that should be pruned
    let result = ExperimentResult {
        speedup_vs_naive: 0.8, // Below 1.5 threshold
        ...
    };

    let strategy = PruningStrategy::new(1.5, 1.3);
    assert!(strategy.should_prune_alternative(&result));
}
```

**Test 3: CSV output verification**
```rust
#[test]
fn test_csv_output() {
    let results = vec![/* test data */];
    let temp_path = PathBuf::from("/tmp/test_dag.csv");

    write_results_csv(&results, &temp_path).unwrap();

    // Read back and verify
    let mut reader = csv::Reader::from_path(&temp_path).unwrap();
    let records: Vec<_> = reader.records().collect();

    assert_eq!(records.len(), results.len());
}
```

---

## Evening: Documentation & Commit

### Task 10: Lab Notebook Entry (30 minutes)

**Create**: `lab-notebook/2025-11/20251104-022-IMPLEMENTATION-dag-testing-harness.md`

**Frontmatter**:
```yaml
---
entry_id: 20251104-022-IMPLEMENTATION-dag-testing-harness
date: 2025-11-04
type: IMPLEMENTATION
status: complete
phase: DAG Completion (Week 1)
---
```

**Content structure**:
```markdown
# DAG Testing Harness Implementation

## Objective
Build unified test framework for systematic hardware exploration with pruning.

## Implementation Details
- File: `crates/asbb-cli/src/dag_traversal.rs` (500+ lines)
- Core abstractions: DAGNode, DAGTraversal, PruningStrategy
- Features: Parallel execution, progress tracking, CSV output

## Key Design Decisions
1. DAG representation: [describe]
2. Pruning thresholds: 1.5× speedup, 1.3× diminishing returns
3. Test execution: [sequential vs parallel decision]

## Validation
- ✅ Small scale test (2 ops × 1 scale)
- ✅ Pruning logic verified
- ✅ CSV output format validated

## Next Steps
- Day 2: Run Batch 1 (NEON+Parallel composition, 240 experiments)
- Day 3: Run Batch 2 & 3 (affinity + thresholds, 500 experiments)
- Day 4: Analyze all 740 results
```

---

### Task 11: Commit (10 minutes)

```bash
git add crates/asbb-cli/src/dag_traversal.rs
git add lab-notebook/2025-11/20251104-022-*.md
git add lab-notebook/INDEX.md

git commit -m "$(cat <<'EOF'
feat: Implement DAG testing harness for systematic hardware exploration

Built unified test framework that systematically explores hardware optimization
space with intelligent pruning (reduces 23,040 → 740 experiments).

Implementation:
- DAGNode: Represents hardware configurations
- DAGTraversal: Executes 3-phase exploration (alternatives, compositions, refinements)
- PruningStrategy: Threshold-based pruning (1.5× speedup minimum)
- Parallel test runner with progress tracking
- CSV output for analysis

Validation:
- Small scale tests (2 operations × 1 scale)
- Pruning logic verified
- Ready to run 740 experiments (Week 1 Day 2-3)

Lab Notebook: Entry 022

🤖 Generated with [Claude Code](https://claude.com/claude-code)

Co-Authored-By: Claude <noreply@anthropic.com>
EOF
)"
```

---

## What You're NOT Doing on Day 1

**NOT running the 740 experiments** - that's Day 2-3
- Day 1 is building the infrastructure
- Day 2-3 is running experiments
- Day 4 is analyzing results

**NOT analyzing results** - that's Day 4
- Day 1 just validates harness works
- Small test runs (2 operations × 1 scale = ~10 experiments)
- Full analysis happens after all 740 complete

**NOT documenting DAG framework** - that's Day 5
- DAG_FRAMEWORK.md already exists (Nov 3)
- Day 5 is updating it with empirical validation results

---

## Existing Infrastructure You Can Reuse

### 1. Operation Loading
From `pilot_parallel.rs`:
```rust
// You already know how to load operations
let gc = GcContent::new(Backend::Neon);
let base = BaseCounting::new(Backend::Neon);
```

### 2. Dataset Loading
From any pilot binary:
```rust
fn load_sequences(path: &Path) -> Result<Vec<SequenceRecord>> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);
    // ... existing FASTQ parsing code
}
```

### 3. Throughput Measurement
From `pilot_parallel.rs`:
```rust
let start = Instant::now();
// Run operation
let elapsed = start.elapsed();
let throughput = (num_sequences as f64) / elapsed.as_secs_f64();
```

### 4. Core Affinity Setup
From `pilot_parallel.rs` (lines 42-69):
- Already have `CoreAssignment` enum
- Already have QoS setup for P-cores vs E-cores

---

## Expected Timeline

### Morning (4 hours)
- 9am-10am: Design architecture
- 10am-11am: Implement DAGNode
- 11am-12:30pm: Implement DAGTraversal
- 12:30pm-1pm: Implement PruningStrategy

**Lunch break**

### Afternoon (4 hours)
- 2pm-3:30pm: Implement test runner + progress tracking
- 3:30pm-4pm: Error handling
- 4pm-4:30pm: CSV output
- 4:30pm-6pm: Validation testing

### Evening (1 hour)
- 6pm-6:30pm: Lab notebook entry
- 6:30pm-7pm: Git commit

**Total: ~9 hours** (includes buffer for debugging)

---

## Success Criteria

By end of Day 1, you should have:

1. ✅ `dag_traversal.rs` file created (~500 lines)
2. ✅ Core abstractions implemented (DAGNode, DAGTraversal, PruningStrategy)
3. ✅ Validation tests passing (2 operations × 1 scale)
4. ✅ CSV output format verified
5. ✅ Lab notebook Entry 022 created
6. ✅ Code committed to git

**Ready for Day 2**: Run 240 NEON+Parallel experiments

---

## Potential Challenges

### Challenge 1: DAG Representation Complexity
**Issue**: Modeling parent-child relationships can get complex

**Solution**: Start simple
- Phase 1: Just test alternatives (ignore compositions)
- Get that working first
- Then add Phase 2 (compositions)
- Then add Phase 3 (refinements)

**Incremental approach** is fine!

---

### Challenge 2: Pruning Logic Edge Cases
**Issue**: What if all alternatives fail? What if GPU works but NEON doesn't?

**Solution**: Handle gracefully
```rust
let successful = filter_successful(&alt_results);
if successful.is_empty() {
    // No successful alternatives, skip compositions
    return Ok(alt_results);
}
```

---

### Challenge 3: Time Management
**Issue**: 8 hours is tight for 500+ lines of new code

**Mitigation strategies**:
1. **Reuse existing code heavily** (copy from `pilot_parallel.rs`)
2. **Start simple**: Get basic traversal working, add features incrementally
3. **Skip perfection**: Day 1 goal is "works", not "beautiful"
4. **Test incrementally**: Don't write all 500 lines then test

**It's OK if you don't finish everything** - as long as basic traversal works!

---

## Questions to Ask During Implementation

1. **Should I test sequentially or in parallel?**
   - Start sequential (simpler, no interference)
   - Add parallel if time permits

2. **How many scales should I test?**
   - Roadmap says 3 scales for NEON+Parallel
   - 8 scales for threshold testing
   - Start with 1 scale for validation

3. **Should I implement all 3 phases on Day 1?**
   - Ideal: Yes (alternatives + compositions + refinements)
   - Minimum: Phase 1 + 2 (alternatives + compositions)
   - Phase 3 can be added on Day 2 if needed

4. **What if validation tests fail?**
   - Debug and fix (that's what validation is for!)
   - Don't proceed to Day 2 until validation passes
   - Better to take extra time on Day 1 than fail on Day 2

---

## Summary

**Day 1 is infrastructure day** - you're building the "experiment execution engine"

**What you're building**:
- A framework that systematically tests hardware configs
- With intelligent pruning (23,040 → 740 experiments)
- Parallel execution for speed
- Progress tracking so you know what's happening
- CSV output for analysis

**What you're NOT building**:
- Analysis tools (that's Day 4)
- The full DAG framework documentation (that's Day 5)
- The biofast library (that's Week 2)

**How to succeed**:
- Reuse existing pilot code heavily
- Start simple, add features incrementally
- Test frequently (don't write 500 lines before first test)
- It's OK to take 9-10 hours instead of 8

**End goal**: When you run this command on Day 2:
```bash
cargo run --release -p asbb-cli --bin asbb-dag-traversal
```

It should systematically run 240 experiments with progress tracking and save results to CSV.

---

**Ready to start?** Any questions about Day 1 before we begin?
