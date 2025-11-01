//! Minimal test - Run a single experiment to diagnose execution issues

use anyhow::Result;
use asbb_core::{HardwareConfig, SequenceRecord};
use asbb_ops::base_counting::BaseCounting;
use asbb_ops::PrimitiveOperation;

fn main() -> Result<()> {
    println!("🧪 Minimal Experiment Test");
    println!("=========================");
    println!();

    // Create minimal test data
    println!("📊 Generating test data...");
    let data = generate_test_data(100, 150);
    println!("   ✅ Generated {} sequences of length 150", data.len());
    println!();

    // Test 1: Naive baseline
    println!("🔬 Test 1: Naive baseline");
    let op = BaseCounting::new();
    let config = HardwareConfig::naive();

    println!("   Running naive...");
    let start = std::time::Instant::now();
    let output = op.execute_with_config(&data, &config)?;
    let duration = start.elapsed();
    println!("   ✅ Completed in {:?}", duration);
    println!("   Output: {:?}", output);
    println!();

    // Test 2: NEON
    #[cfg(target_arch = "aarch64")]
    {
        println!("🔬 Test 2: NEON");
        let mut neon_config = HardwareConfig::naive();
        neon_config.use_neon = true;

        println!("   Running NEON...");
        let start = std::time::Instant::now();
        let output = op.execute_with_config(&data, &neon_config)?;
        let duration = start.elapsed();
        println!("   ✅ Completed in {:?}", duration);
        println!("   Output: {:?}", output);
        println!();
    }

    // Test 3: Parallel
    println!("🔬 Test 3: Parallel (2 threads)");
    let mut parallel_config = HardwareConfig::naive();
    parallel_config.num_threads = 2;

    println!("   Running parallel...");
    let start = std::time::Instant::now();
    let output = op.execute_with_config(&data, &parallel_config)?;
    let duration = start.elapsed();
    println!("   ✅ Completed in {:?}", duration);
    println!("   Output: {:?}", output);
    println!();

    println!("✅ All tests passed!");
    println!();
    println!("💡 Diagnosis: If you see this, operations work correctly.");
    println!("   The Level 1/2 hang is likely in ExecutionEngine infrastructure.");

    Ok(())
}

fn generate_test_data(num_sequences: usize, seq_length: usize) -> Vec<SequenceRecord> {
    use rand::{Rng, SeedableRng};
    use rand_chacha::ChaCha8Rng;

    let mut rng = ChaCha8Rng::seed_from_u64(12345);

    (0..num_sequences)
        .map(|i| {
            let id = format!("seq_{}", i);
            let sequence: Vec<u8> = (0..seq_length)
                .map(|_| match rng.gen_range(0..4) {
                    0 => b'A',
                    1 => b'C',
                    2 => b'G',
                    _ => b'T',
                })
                .collect();

            let quality: Vec<u8> = (0..seq_length)
                .map(|_| rng.gen_range(33..74))
                .collect();

            SequenceRecord::fastq(id, sequence, quality)
        })
        .collect()
}
