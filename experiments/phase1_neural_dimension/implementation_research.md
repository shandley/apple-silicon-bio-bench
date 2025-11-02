# Neural Engine Pilot: Implementation Research

**Date**: November 2, 2025
**Status**: Technical feasibility analysis
**Purpose**: Evaluate implementation approaches for Neural Engine integration

---

## Core ML Integration Options

### Option 1: Swift/Objective-C Wrapper (Recommended)

**Approach**:
```
Rust (asbb-ops) → C FFI → Swift wrapper → Core ML → Neural Engine
```

**Pros**:
- Core ML fully supported in Swift/Objective-C
- Direct access to MLModel, MLMultiArray
- Neural Engine dispatch guaranteed
- Well-documented Apple APIs

**Cons**:
- FFI complexity (Rust ↔ C ↔ Swift)
- Build system complexity (cargo + xcodebuild)
- Need to maintain Swift codebase

**Implementation Steps**:
1. Create Swift package with Core ML inference
2. Export C-compatible functions
3. Create Rust FFI bindings
4. Integrate into asbb-ops crate

**Estimated Time**: 2-3 days (initial setup), 1 day per operation

### Option 2: Pure Rust via `onnxruntime` (Alternative)

**Approach**:
```
Convert Core ML → ONNX → ONNX Runtime (with CoreML EP)
```

**Pros**:
- Pure Rust (no FFI)
- Cross-platform (ONNX models portable)
- `onnxruntime` crate available

**Cons**:
- CoreML Execution Provider support unclear on macOS
- May not dispatch to Neural Engine reliably
- Additional conversion step (Core ML → ONNX)
- Less "Apple Silicon First" approach

**Verdict**: Not recommended for this pilot

### Option 3: Python Bridge (Prototyping Only)

**Approach**:
```
Rust → PyO3 → Python → coremltools → Neural Engine
```

**Pros**:
- Rapid prototyping
- Easy model creation with coremltools
- Good for feasibility testing

**Cons**:
- Python overhead
- Not production-ready
- Can't measure true performance

**Verdict**: Useful for initial feasibility, not for pilot experiments

---

## Recommended Architecture

### Swift Core ML Wrapper

**File**: `crates/asbb-neural/swift/CoreMLWrapper.swift`
```swift
import Foundation
import CoreML

@objc public class CoreMLWrapper: NSObject {
    private var model: MLModel?

    @objc public func loadModel(path: String) -> Bool {
        guard let url = URL(fileURLWithPath: path) else { return false }

        do {
            let config = MLModelConfiguration()
            config.computeUnits = .all  // Allow Neural Engine dispatch
            self.model = try MLModel(contentsOf: url, configuration: config)
            return true
        } catch {
            print("Error loading model: \(error)")
            return false
        }
    }

    @objc public func predict(input: UnsafeMutablePointer<Float>,
                              inputSize: Int,
                              output: UnsafeMutablePointer<Float>,
                              outputSize: Int) -> Bool {
        guard let model = self.model else { return false }

        do {
            // Convert C array to MLMultiArray
            let shape = [NSNumber(value: inputSize)]
            let mlArray = try MLMultiArray(shape: shape, dataType: .float32)
            for i in 0..<inputSize {
                mlArray[i] = NSNumber(value: input[i])
            }

            let features = CoreMLInputFeatures(input: mlArray)
            let prediction = try model.prediction(from: features)

            // Extract output
            // ... (model-specific output parsing)

            return true
        } catch {
            print("Prediction error: \(error)")
            return false
        }
    }
}
```

### C FFI Header

**File**: `crates/asbb-neural/swift/coreml_bridge.h`
```c
#ifndef COREML_BRIDGE_H
#define COREML_BRIDGE_H

#include <stdbool.h>

typedef void* CoreMLModelHandle;

CoreMLModelHandle coreml_load_model(const char* path);
void coreml_free_model(CoreMLModelHandle handle);

bool coreml_predict(CoreMLModelHandle handle,
                    const float* input,
                    int input_size,
                    float* output,
                    int output_size);

#endif
```

### Rust FFI Bindings

**File**: `crates/asbb-neural/src/ffi.rs`
```rust
use std::ffi::{CString, c_void};
use std::os::raw::c_char;

#[link(name = "coreml_bridge")]
extern "C" {
    fn coreml_load_model(path: *const c_char) -> *mut c_void;
    fn coreml_free_model(handle: *mut c_void);
    fn coreml_predict(
        handle: *mut c_void,
        input: *const f32,
        input_size: i32,
        output: *mut f32,
        output_size: i32,
    ) -> bool;
}

pub struct CoreMLModel {
    handle: *mut c_void,
}

impl CoreMLModel {
    pub fn load(path: &str) -> Result<Self> {
        let c_path = CString::new(path)?;
        let handle = unsafe { coreml_load_model(c_path.as_ptr()) };

        if handle.is_null() {
            anyhow::bail!("Failed to load Core ML model: {}", path);
        }

        Ok(Self { handle })
    }

    pub fn predict(&self, input: &[f32]) -> Result<Vec<f32>> {
        let output_size = 1; // Model-specific
        let mut output = vec![0.0f32; output_size];

        let success = unsafe {
            coreml_predict(
                self.handle,
                input.as_ptr(),
                input.len() as i32,
                output.as_mut_ptr(),
                output_size as i32,
            )
        };

        if !success {
            anyhow::bail!("Prediction failed");
        }

        Ok(output)
    }
}

impl Drop for CoreMLModel {
    fn drop(&mut self) {
        unsafe { coreml_free_model(self.handle) };
    }
}
```

---

## Model Creation

### Stub Models (Option 1: Fastest)

**Python Script**: `experiments/phase1_neural_dimension/create_stub_models.py`
```python
import coremltools as ct
import numpy as np
from coremltools.models import neural_network as nn

def create_quality_filter_stub():
    """
    Binary classifier: 150 quality scores → pass/fail
    """
    input_features = [('quality_scores', 150)]
    output_features = [('pass', None)]

    builder = nn.NeuralNetworkBuilder(input_features, output_features)

    # Simple 2-layer MLP
    builder.add_inner_product(
        name='fc1',
        input_name='quality_scores',
        output_name='fc1_out',
        input_channels=150,
        output_channels=64
    )
    builder.add_activation(
        name='relu1',
        non_linearity='RELU',
        input_name='fc1_out',
        output_name='relu1_out'
    )
    builder.add_inner_product(
        name='fc2',
        input_name='relu1_out',
        output_name='fc2_out',
        input_channels=64,
        output_channels=2
    )
    builder.add_softmax(
        name='softmax',
        input_name='fc2_out',
        output_name='pass'
    )

    model = ct.models.MLModel(builder.spec)
    model.save("quality_filter.mlmodel")
    print("Created quality_filter.mlmodel (stub)")

def create_complexity_score_stub():
    """
    Regressor: 256 k-mer features → complexity score (0.0-1.0)
    """
    input_features = [('kmer_features', 256)]
    output_features = [('complexity', None)]

    builder = nn.NeuralNetworkBuilder(input_features, output_features)

    builder.add_inner_product(
        name='fc1',
        input_name='kmer_features',
        output_name='fc1_out',
        input_channels=256,
        output_channels=128
    )
    builder.add_activation(
        name='relu1',
        non_linearity='RELU',
        input_name='fc1_out',
        output_name='relu1_out'
    )
    builder.add_inner_product(
        name='fc2',
        input_name='relu1_out',
        output_name='complexity',
        input_channels=128,
        output_channels=1
    )

    model = ct.models.MLModel(builder.spec)
    model.save("complexity_score.mlmodel")
    print("Created complexity_score.mlmodel (stub)")

if __name__ == "__main__":
    create_quality_filter_stub()
    create_complexity_score_stub()
```

**Usage**:
```bash
cd experiments/phase1_neural_dimension
python create_stub_models.py
# Outputs: quality_filter.mlmodel, complexity_score.mlmodel
```

---

## Data Conversion

### Quality Scores → MLMultiArray

**Challenge**: FASTQ quality strings (Phred+33 ASCII) → Float tensor

**Approach**:
```rust
fn quality_to_ml_input(quality: &[u8]) -> Vec<f32> {
    // Fixed-size input: pad or truncate to 150
    let target_len = 150;
    let mut input = vec![0.0f32; target_len];

    for (i, &q) in quality.iter().take(target_len).enumerate() {
        // Phred score: ASCII - 33, normalize to [0, 1]
        let phred = (q - 33) as f32;
        input[i] = phred / 40.0; // Assuming max Phred = 40
    }

    input
}
```

### Sequence → K-mer Features

**Challenge**: Variable-length sequence → Fixed-size feature vector

**Approach 1: K-mer Frequency**:
```rust
fn sequence_to_kmer_features(sequence: &[u8], k: usize) -> Vec<f32> {
    // 4^k possible k-mers, e.g., k=4 → 256 features
    let num_features = 4_usize.pow(k as u32);
    let mut features = vec![0.0f32; num_features];

    for kmer in sequence.windows(k) {
        let index = kmer_to_index(kmer);
        features[index] += 1.0;
    }

    // Normalize by sequence length
    let total: f32 = features.iter().sum();
    for f in &mut features {
        *f /= total;
    }

    features
}
```

---

## Verification: Neural Engine Dispatch

### Check 1: Instruments Profiling

**Tool**: Xcode Instruments → Core ML instrument

**Steps**:
1. Run experiment with Neural Engine backend
2. Profile with `instruments -t CoreML`
3. Verify "Neural Engine" in execution trace

### Check 2: GPU/CPU Utilization

**Expectation**: If Neural Engine active, GPU/CPU should be low

**Monitoring**:
```bash
# During experiment execution
sudo powermetrics --samplers cpu_power,gpu_power -i 1000 -n 10
```

**Interpretation**:
- Neural Engine active: Low GPU/CPU, high ANE (Apple Neural Engine) power
- Fallback to CPU: High CPU, low GPU
- Fallback to GPU: Low CPU, high GPU

---

## Build System Integration

### Cargo + Swift Build

**File**: `crates/asbb-neural/build.rs`
```rust
use std::process::Command;

fn main() {
    // Compile Swift wrapper
    let status = Command::new("swiftc")
        .args(&[
            "-emit-library",
            "-module-name", "CoreMLBridge",
            "-o", "libcoreml_bridge.dylib",
            "swift/CoreMLWrapper.swift",
        ])
        .status()
        .expect("Failed to compile Swift wrapper");

    if !status.success() {
        panic!("Swift compilation failed");
    }

    println!("cargo:rustc-link-lib=dylib=coreml_bridge");
    println!("cargo:rustc-link-search=native={}", env!("CARGO_MANIFEST_DIR"));
}
```

---

## Alternative: Simplified Approach

### Skip Core ML, Use Metal Performance Shaders (MPS)

**Idea**: Use Metal Performance Shaders for ML inference
- MPS has built-in neural network layers
- Direct Metal integration (no Core ML overhead)
- May dispatch to Neural Engine via Metal

**Pros**:
- No Swift FFI needed
- Metal already integrated (from GPU pilot)
- Potentially lower overhead

**Cons**:
- Less clear if Neural Engine is used
- MPS is GPU-focused, not Neural Engine-focused
- More manual model building

**Verdict**: Interesting alternative, but deviates from "Neural Engine" focus

---

## Complexity Assessment

### Estimated Implementation Time

**Core ML + FFI Approach** (Option 1):
- Swift wrapper: 1-2 days
- Rust FFI bindings: 1 day
- Stub model creation: 0.5 days
- 3 operation backends: 1 day
- Experiment harness: 0.5 days
- **Total**: 4-5 days (1 week)

**Python Prototyping** (Feasibility only):
- PyO3 setup: 0.5 days
- Python wrapper: 0.5 days
- Stub models: 0.5 days
- Basic testing: 0.5 days
- **Total**: 2 days

**Comparison to Other Pilots**:
- NEON: 2 days (SIMD intrinsics)
- GPU: 3 days (Metal shaders)
- AMX: 1 day (Accelerate framework)
- Parallel: 2 days (Rayon)
- **Neural Engine**: 5 days (FFI + models) ← Most complex

---

## Recommendation

### Proceed with Neural Engine Pilot?

**YES, if**:
- Core ML integration is valuable for future work
- Willing to invest 1 week for comprehensive pilot
- Interested in ML-based sequence analysis architectures
- Want complete coverage of Apple Silicon features

**NO (defer), if**:
- Prefer faster iteration (2-3 day pilots)
- Want to maintain momentum (5/9 → 9/9 quickly)
- Likely negative finding not worth 1-week investment
- Can defer to "advanced" pilots after basics

### Alternative: Hardware Compression Pilot

**Hardware Compression** (7/9):
- **Simpler**: AppleArchive framework, native Swift/Objective-C
- **Faster**: 2-3 days implementation
- **Broadly applicable**: All I/O operations
- **Likely positive**: Hardware compression well-proven

**Advantages**:
- Maintain pilot velocity
- Build toward 9/9 completion faster
- Neural Engine as "bonus" pilot after completing 9/9

---

## Conclusion

Neural Engine pilot is **feasible but complex** (1 week vs 2-3 days for other pilots).

**Recommended Path**:
1. **Defer Neural Engine** (for now)
2. Complete **Hardware Compression** (7/9) → 2-3 days
3. Complete **GCD/QoS** (8/9) → 2-3 days
4. Complete **M5 GPU Neural Accelerators** (9/9) → 2-3 days (if M5 available)
5. **Then return to Neural Engine** (6/9) with full context

**Total time to 9/9**: ~7-9 days (3 pilots × 2-3 days each)
vs
**Neural Engine first**: 5 days + 6 days (remaining 3 pilots) = 11 days

**Time saved**: 2-4 days by deferring Neural Engine

---

**Created**: November 2, 2025
**Status**: Implementation feasibility confirmed, deferral recommended
**Next**: Await user decision on proceed vs defer
