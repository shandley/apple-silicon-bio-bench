#!/usr/bin/env python3
"""
Compare Mac M4 vs AWS Graviton 3 performance (cross-platform validation).

This script:
1. Loads Mac baseline data (from power pilot)
2. Loads Graviton experiment data
3. Joins on operation/config/scale
4. Calculates portability metrics
5. Outputs comparison CSV

Usage:
    python analysis/compare_mac_graviton.py <mac_baseline_csv> <graviton_raw_csv>

Example:
    python analysis/compare_mac_graviton.py \
        results/cross_platform_graviton/mac_baseline.csv \
        results/cross_platform_graviton/graviton_raw_20251102_140000.csv
"""

import sys
from pathlib import Path
import csv
from collections import defaultdict

def load_csv(csv_path):
    """Load CSV into list of dicts."""
    experiments = []
    with open(csv_path, 'r') as f:
        reader = csv.DictReader(f)
        for row in reader:
            experiments.append(row)
    return experiments

def calculate_speedups(experiments):
    """Calculate speedup vs naive for each platform."""

    # Group by operation and scale to find naive baseline
    naive_throughput = {}

    for exp in experiments:
        if exp['config'] == 'naive':
            key = (exp['operation'], exp['scale'])
            naive_throughput[key] = float(exp['throughput_seqs_per_sec'])

    # Calculate speedups
    for exp in experiments:
        key = (exp['operation'], exp['scale'])
        if key in naive_throughput:
            naive_tput = naive_throughput[key]
            current_tput = float(exp['throughput_seqs_per_sec'])
            exp['speedup_vs_naive'] = current_tput / naive_tput if naive_tput > 0 else 0.0
        else:
            exp['speedup_vs_naive'] = 1.0  # No baseline found

    return experiments

def compare_platforms(mac_data, graviton_data):
    """Compare Mac vs Graviton performance."""

    # Calculate speedups for each platform
    mac_data = calculate_speedups(mac_data)
    graviton_data = calculate_speedups(graviton_data)

    # Create lookup for Mac data
    mac_lookup = {}
    for exp in mac_data:
        key = (exp['operation'], exp['config'], exp['scale'])
        mac_lookup[key] = exp

    # Join Graviton data with Mac data
    comparison = []

    for g_exp in graviton_data:
        key = (g_exp['operation'], g_exp['config'], g_exp['scale'])

        if key in mac_lookup:
            m_exp = mac_lookup[key]

            # Calculate portability metrics
            mac_speedup = float(m_exp['speedup_vs_naive'])
            graviton_speedup = float(g_exp['speedup_vs_naive'])

            portability_ratio = graviton_speedup / mac_speedup if mac_speedup > 0 else 0.0
            speedup_variance_pct = ((graviton_speedup - mac_speedup) / mac_speedup * 100) if mac_speedup > 0 else 0.0

            comparison.append({
                'operation': g_exp['operation'],
                'config': g_exp['config'],
                'scale': g_exp['scale'],
                'num_sequences': g_exp['num_sequences'],

                # Mac metrics
                'mac_throughput': float(m_exp['throughput_seqs_per_sec']),
                'mac_speedup': mac_speedup,

                # Graviton metrics
                'graviton_throughput': float(g_exp['throughput_seqs_per_sec']),
                'graviton_speedup': graviton_speedup,

                # Cross-platform metrics
                'portability_ratio': portability_ratio,
                'speedup_variance_pct': speedup_variance_pct,

                # Absolute throughput ratio
                'graviton_vs_mac_throughput': float(g_exp['throughput_seqs_per_sec']) / float(m_exp['throughput_seqs_per_sec'])
                    if float(m_exp['throughput_seqs_per_sec']) > 0 else 0.0,
            })

    return comparison

def write_comparison(comparison, output_path):
    """Write comparison CSV."""

    if not comparison:
        print("No comparison data to write", file=sys.stderr)
        return

    with open(output_path, 'w', newline='') as f:
        fieldnames = comparison[0].keys()
        writer = csv.DictWriter(f, fieldnames=fieldnames)
        writer.writeheader()
        writer.writerows(comparison)

    print(f"\n✅ Comparison complete: {len(comparison)} experiments", file=sys.stderr)
    print(f"Output: {output_path}", file=sys.stderr)

def print_summary(comparison):
    """Print summary statistics."""

    if not comparison:
        return

    # Filter for NEON single-threaded (most important for portability)
    neon_single = [c for c in comparison if c['config'] == 'neon']

    if neon_single:
        ratios = [c['portability_ratio'] for c in neon_single]
        avg_ratio = sum(ratios) / len(ratios)
        min_ratio = min(ratios)
        max_ratio = max(ratios)

        print("\n=== Portability Summary (NEON single-threaded) ===", file=sys.stderr)
        print(f"Experiments: {len(neon_single)}", file=sys.stderr)
        print(f"Portability ratio: {avg_ratio:.2f} (range: {min_ratio:.2f} - {max_ratio:.2f})", file=sys.stderr)
        print(f"Expected: 0.8 - 1.2 (within ±20%)", file=sys.stderr)

        if 0.8 <= avg_ratio <= 1.2:
            print("✅ Portability VALIDATED", file=sys.stderr)
        else:
            print("⚠️  Portability outside expected range", file=sys.stderr)

def main():
    if len(sys.argv) != 3:
        print("Usage: python compare_mac_graviton.py <mac_baseline_csv> <graviton_raw_csv>")
        sys.exit(1)

    mac_csv = Path(sys.argv[1])
    graviton_csv = Path(sys.argv[2])

    if not mac_csv.exists():
        print(f"Error: Mac baseline not found: {mac_csv}")
        sys.exit(1)

    if not graviton_csv.exists():
        print(f"Error: Graviton data not found: {graviton_csv}")
        sys.exit(1)

    print("Loading data...", file=sys.stderr)
    mac_data = load_csv(mac_csv)
    graviton_data = load_csv(graviton_csv)

    print(f"Mac experiments: {len(mac_data)}", file=sys.stderr)
    print(f"Graviton experiments: {len(graviton_data)}", file=sys.stderr)

    print("\nComparing platforms...", file=sys.stderr)
    comparison = compare_platforms(mac_data, graviton_data)

    output_path = Path("results/cross_platform_graviton/mac_vs_graviton_comparison.csv")
    write_comparison(comparison, output_path)

    print_summary(comparison)

if __name__ == '__main__':
    main()
