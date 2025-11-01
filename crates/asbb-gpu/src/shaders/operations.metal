//! Metal Compute Shaders for Bioinformatics Operations
//!
//! These kernels leverage Apple Silicon's unified memory architecture
//! for zero-copy data access and massive parallelism.
//!
//! Architecture:
//! - Each thread processes one sequence (embarrassingly parallel)
//! - Unified memory: no data transfer overhead
//! - Optimized for batch sizes >50K (amortize dispatch overhead)

#include <metal_stdlib>
using namespace metal;

/// Base counting kernel - counts A, C, G, T in each sequence
///
/// Each thread processes one complete sequence and writes results.
///
/// @param sequences Flattened sequence data (all sequences concatenated)
/// @param seq_offsets Start offset for each sequence in the sequences buffer
/// @param seq_lengths Length of each sequence
/// @param counts Output buffer [num_sequences * 4] for [A, C, G, T] counts per sequence
/// @param gid Thread ID (one thread per sequence)
kernel void count_bases(
    device const uchar* sequences [[buffer(0)]],
    device const uint* seq_offsets [[buffer(1)]],
    device const uint* seq_lengths [[buffer(2)]],
    device uint* counts [[buffer(3)]],
    uint gid [[thread_position_in_grid]]
) {
    // Get sequence boundaries for this thread
    uint offset = seq_offsets[gid];
    uint length = seq_lengths[gid];

    // Initialize counters
    uint count_a = 0;
    uint count_c = 0;
    uint count_g = 0;
    uint count_t = 0;

    // Count bases in this sequence
    for (uint i = 0; i < length; i++) {
        uchar base = sequences[offset + i];

        // Count bases (case-insensitive)
        if (base == 'A' || base == 'a') {
            count_a++;
        } else if (base == 'C' || base == 'c') {
            count_c++;
        } else if (base == 'G' || base == 'g') {
            count_g++;
        } else if (base == 'T' || base == 't') {
            count_t++;
        }
        // Ignore other characters (N, etc.)
    }

    // Write results (4 counts per sequence)
    uint base_idx = gid * 4;
    counts[base_idx + 0] = count_a;
    counts[base_idx + 1] = count_c;
    counts[base_idx + 2] = count_g;
    counts[base_idx + 3] = count_t;
}

/// GC content kernel - counts G and C bases
///
/// Optimized variant that only counts GC bases (simpler than full counting).
///
/// @param sequences Flattened sequence data
/// @param seq_offsets Start offset for each sequence
/// @param seq_lengths Length of each sequence
/// @param gc_counts Output buffer [num_sequences] for GC count per sequence
/// @param gid Thread ID
kernel void count_gc(
    device const uchar* sequences [[buffer(0)]],
    device const uint* seq_offsets [[buffer(1)]],
    device const uint* seq_lengths [[buffer(2)]],
    device uint* gc_counts [[buffer(3)]],
    uint gid [[thread_position_in_grid]]
) {
    uint offset = seq_offsets[gid];
    uint length = seq_lengths[gid];

    uint gc_count = 0;

    for (uint i = 0; i < length; i++) {
        uchar base = sequences[offset + i];

        if (base == 'G' || base == 'g' || base == 'C' || base == 'c') {
            gc_count++;
        }
    }

    gc_counts[gid] = gc_count;
}

/// AT content kernel - counts A and T bases
kernel void count_at(
    device const uchar* sequences [[buffer(0)]],
    device const uint* seq_offsets [[buffer(1)]],
    device const uint* seq_lengths [[buffer(2)]],
    device uint* at_counts [[buffer(3)]],
    uint gid [[thread_position_in_grid]]
) {
    uint offset = seq_offsets[gid];
    uint length = seq_lengths[gid];

    uint at_count = 0;

    for (uint i = 0; i < length; i++) {
        uchar base = sequences[offset + i];

        if (base == 'A' || base == 'a' || base == 'T' || base == 't') {
            at_count++;
        }
    }

    at_counts[gid] = at_count;
}

/// Reverse complement kernel - transforms each sequence
///
/// Each thread reads a sequence, reverse complements it, and writes output.
///
/// @param sequences Input sequences
/// @param seq_offsets Start offset for each input sequence
/// @param seq_lengths Length of each sequence
/// @param output Output buffer for reversed complemented sequences
/// @param out_offsets Start offset for each output sequence
/// @param gid Thread ID
kernel void reverse_complement(
    device const uchar* sequences [[buffer(0)]],
    device const uint* seq_offsets [[buffer(1)]],
    device const uint* seq_lengths [[buffer(2)]],
    device uchar* output [[buffer(3)]],
    device const uint* out_offsets [[buffer(4)]],
    uint gid [[thread_position_in_grid]]
) {
    uint offset = seq_offsets[gid];
    uint length = seq_lengths[gid];
    uint out_offset = out_offsets[gid];

    // Reverse complement: read forward, write backward with complement
    for (uint i = 0; i < length; i++) {
        uchar base = sequences[offset + i];
        uchar complement;

        // Complement mapping (case-preserving)
        if (base == 'A') complement = 'T';
        else if (base == 'a') complement = 't';
        else if (base == 'C') complement = 'G';
        else if (base == 'c') complement = 'g';
        else if (base == 'G') complement = 'C';
        else if (base == 'g') complement = 'c';
        else if (base == 'T') complement = 'A';
        else if (base == 't') complement = 'a';
        else complement = base;  // N or other -> unchanged

        // Write in reverse order
        output[out_offset + (length - 1 - i)] = complement;
    }
}

/// Sequence length kernel - just returns lengths
///
/// Trivial operation for testing overhead measurement.
///
/// @param seq_lengths Input sequence lengths
/// @param output Output lengths (copy of input)
/// @param gid Thread ID
kernel void sequence_length(
    device const uint* seq_lengths [[buffer(0)]],
    device uint* output [[buffer(1)]],
    uint gid [[thread_position_in_grid]]
) {
    output[gid] = seq_lengths[gid];
}

/// Quality filter kernel - filters sequences by mean quality
///
/// Each thread checks one sequence and marks it for inclusion/exclusion.
///
/// @param quality_scores Quality scores (flattened)
/// @param seq_offsets Start offset for each sequence
/// @param seq_lengths Length of each sequence
/// @param min_quality Minimum mean quality threshold
/// @param pass_filter Output buffer [num_sequences] - 1 if pass, 0 if fail
/// @param gid Thread ID
kernel void quality_filter(
    device const uchar* quality_scores [[buffer(0)]],
    device const uint* seq_offsets [[buffer(1)]],
    device const uint* seq_lengths [[buffer(2)]],
    constant float& min_quality [[buffer(3)]],
    device uint* pass_filter [[buffer(4)]],
    uint gid [[thread_position_in_grid]]
) {
    uint offset = seq_offsets[gid];
    uint length = seq_lengths[gid];

    // Calculate mean quality
    uint sum = 0;
    for (uint i = 0; i < length; i++) {
        sum += quality_scores[offset + i];
    }

    float mean_quality = float(sum) / float(length);

    // Mark pass/fail
    pass_filter[gid] = (mean_quality >= min_quality) ? 1 : 0;
}

/// Length filter kernel - filters sequences by length
kernel void length_filter(
    device const uint* seq_lengths [[buffer(0)]],
    constant uint& min_length [[buffer(1)]],
    constant uint& max_length [[buffer(2)]],
    device uint* pass_filter [[buffer(3)]],
    uint gid [[thread_position_in_grid]]
) {
    uint length = seq_lengths[gid];
    pass_filter[gid] = (length >= min_length && length <= max_length) ? 1 : 0;
}

/// Quality aggregation kernel - computes min/max/sum for each sequence
///
/// Each thread processes one sequence's quality scores and outputs aggregated stats.
///
/// @param quality_scores Quality scores (flattened, all sequences concatenated)
/// @param seq_offsets Start offset for each sequence in quality_scores buffer
/// @param seq_lengths Length of each sequence
/// @param stats Output buffer [num_sequences * 4] for [min, max, sum_low32, sum_high32] per sequence
/// @param gid Thread ID
kernel void aggregate_quality(
    device const uchar* quality_scores [[buffer(0)]],
    device const uint* seq_offsets [[buffer(1)]],
    device const uint* seq_lengths [[buffer(2)]],
    device uint* stats [[buffer(3)]],
    uint gid [[thread_position_in_grid]]
) {
    uint offset = seq_offsets[gid];
    uint length = seq_lengths[gid];

    // Initialize min/max/sum
    uchar min_q = 255;
    uchar max_q = 0;
    ulong sum_q = 0;  // Use 64-bit to avoid overflow

    // Process all quality scores in this sequence
    for (uint i = 0; i < length; i++) {
        uchar q = quality_scores[offset + i];

        if (q < min_q) min_q = q;
        if (q > max_q) max_q = q;
        sum_q += q;
    }

    // Write results (4 values per sequence)
    // Split 64-bit sum into two 32-bit values for compatibility
    uint base_idx = gid * 4;
    stats[base_idx + 0] = min_q;
    stats[base_idx + 1] = max_q;
    stats[base_idx + 2] = uint(sum_q & 0xFFFFFFFF);        // Low 32 bits
    stats[base_idx + 3] = uint((sum_q >> 32) & 0xFFFFFFFF); // High 32 bits
}

/// Complexity score kernel - calculates sequence complexity (character diversity)
///
/// Each thread processes one sequence and computes its complexity score.
/// Complexity = unique_characters / max_unique (normalized diversity)
///
/// @param sequences Flattened sequence data
/// @param seq_offsets Start offset for each sequence
/// @param seq_lengths Length of each sequence
/// @param complexity Output buffer [num_sequences] for complexity scores (as uint, scaled by 1000)
/// @param gid Thread ID
kernel void calculate_complexity(
    device const uchar* sequences [[buffer(0)]],
    device const uint* seq_offsets [[buffer(1)]],
    device const uint* seq_lengths [[buffer(2)]],
    device uint* complexity [[buffer(3)]],
    uint gid [[thread_position_in_grid]]
) {
    uint offset = seq_offsets[gid];
    uint length = seq_lengths[gid];

    if (length == 0) {
        complexity[gid] = 0;
        return;
    }

    // Count character occurrences (use local array for counts)
    // We only care about unique count, not actual counts
    // Track which characters we've seen using a bitset approach
    // For 256 possible bytes, use 8 uint32s (256 bits)
    uint seen[8] = {0, 0, 0, 0, 0, 0, 0, 0};

    for (uint i = 0; i < length; i++) {
        uchar c = sequences[offset + i];
        uint idx = c / 32;  // Which uint32 (0-7)
        uint bit = c % 32;  // Which bit (0-31)
        seen[idx] |= (1u << bit);  // Set the bit
    }

    // Count unique characters (count set bits)
    uint unique = 0;
    for (uint i = 0; i < 8; i++) {
        unique += popcount(seen[i]);
    }

    // Calculate complexity: unique / max_unique (max 4 for ACGT)
    uint max_unique = min(length, 4u);
    float score = float(unique) / float(max_unique);

    // Store as scaled integer (multiply by 1000 for precision)
    complexity[gid] = uint(score * 1000.0);
}
