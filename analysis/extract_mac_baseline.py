#!/usr/bin/env python3
"""
Extract Mac baseline data from power consumption pilot for cross-platform comparison.

This script:
1. Loads power pilot raw CSV
2. Filters for operations that will be tested on Graviton
3. Outputs Mac baseline CSV

Usage:
    python analysis/extract_mac_baseline.py <power_pilot_csv>

Example:
    python analysis/extract_mac_baseline.py \
        results/phase1_power_consumption/power_pilot_raw_20251102_184235.csv
"""

import sys
from pathlib import Path
import csv

def extract_mac_baseline(power_csv_path, output_path):
    """Extract Mac baseline data for Graviton comparison."""

    # Operations to include (matching Graviton pilot)
    target_operations = {
        'base_counting',
        'gc_content',
        'quality_aggregation',
    }

    # Configs to include
    target_configs = {
        'naive',
        'neon',
        'neon_4t',
    }

    # Scales: Medium (10K) and Large (100K) from power pilot
    # Will map to Graviton's Medium and Large
    target_scales = {'Medium', 'Large'}

    baseline = []

    with open(power_csv_path, 'r') as f:
        reader = csv.DictReader(f)
        for row in reader:
            if (row['operation'] in target_operations and
                row['config'] in target_configs and
                row['scale'] in target_scales):
                baseline.append(row)

    # Write baseline CSV
    if baseline:
        with open(output_path, 'w', newline='') as f:
            writer = csv.DictWriter(f, fieldnames=baseline[0].keys())
            writer.writeheader()
            writer.writerows(baseline)

        print(f"Extracted {len(baseline)} Mac baseline experiments", file=sys.stderr)
        print(f"Output: {output_path}", file=sys.stderr)
    else:
        print("No matching experiments found", file=sys.stderr)

def main():
    if len(sys.argv) != 2:
        print("Usage: python extract_mac_baseline.py <power_pilot_csv>")
        sys.exit(1)

    power_csv = Path(sys.argv[1])

    if not power_csv.exists():
        print(f"Error: File not found: {power_csv}")
        sys.exit(1)

    output_path = Path("results/cross_platform_graviton/mac_baseline.csv")
    output_path.parent.mkdir(parents=True, exist_ok=True)

    extract_mac_baseline(power_csv, output_path)

if __name__ == '__main__':
    main()
