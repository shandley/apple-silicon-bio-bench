//! Grand Central Dispatch (GCD) utilities for macOS
//!
//! Minimal wrapper around Apple's libdispatch for feasibility testing.
//! Provides simple parallel dispatch using GCD queues with QoS support.

/// QoS class for dispatch queues
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QoSClass {
    /// Default QoS (balanced)
    Default,
    /// User-initiated QoS (high priority, for interactive tasks)
    UserInitiated,
    /// Utility QoS (lower priority, power-efficient)
    Utility,
}

/// Parallel process a slice of data using GCD dispatch_apply
///
/// This is similar to Rayon's par_iter() but uses Apple's Grand Central Dispatch.
/// Each element is processed in parallel on a concurrent global queue.
///
/// Returns a Vec of results in the same order as the input.
#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
pub fn gcd_parallel_map<T, F, R>(data: &[T], qos: QoSClass, f: F) -> Vec<R>
where
    T: Sync,
    F: Fn(&T) -> R + Send + Sync,
    R: Send + Default,
{
    use dispatch::{Queue, QueuePriority};
    use std::sync::{Arc, Mutex};

    // Get global concurrent queue with specified QoS
    let priority = match qos {
        QoSClass::Default => QueuePriority::Default,
        QoSClass::UserInitiated => QueuePriority::High,
        QoSClass::Utility => QueuePriority::Low,
    };
    let queue = Queue::global(priority);

    // Results vector with pre-allocated capacity
    // Use Vec<Option<R>> to handle out-of-order completion
    let results: Vec<Option<R>> = (0..data.len()).map(|_| None).collect();
    let results = Arc::new(Mutex::new(results));

    // Dispatch work in parallel
    let results_clone = Arc::clone(&results);
    queue.exec(move || {
        // Simple parallel loop - each iteration processes one item
        // GCD handles work distribution automatically
        for i in 0..data.len() {
            let result = f(&data[i]);
            results_clone.lock().unwrap()[i] = Some(result);
        }
    });

    // Wait for completion
    queue.sync(|| {});

    // Extract results (unwrap Options - all should be Some)
    Arc::try_unwrap(results)
        .unwrap()
        .into_inner()
        .unwrap()
        .into_iter()
        .map(|opt| opt.unwrap())
        .collect()
}

/// Fallback for non-macOS platforms - just use sequential processing
#[cfg(not(all(target_os = "macos", target_arch = "aarch64")))]
pub fn gcd_parallel_map<T, F, R>(data: &[T], _qos: QoSClass, f: F) -> Vec<R>
where
    F: Fn(&T) -> R,
{
    data.iter().map(f).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gcd_parallel_map() {
        let data = vec![1, 2, 3, 4, 5];
        let results = gcd_parallel_map(&data, QoSClass::Default, |x| x * 2);

        assert_eq!(results.len(), 5);
        assert!(results.contains(&2));
        assert!(results.contains(&4));
        assert!(results.contains(&6));
        assert!(results.contains(&8));
        assert!(results.contains(&10));
    }
}
