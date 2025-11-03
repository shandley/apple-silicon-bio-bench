#!/usr/bin/env python3
"""
Parse powermetrics log and correlate with power pilot CSV.

This script:
1. Parses powermetrics output (CPU package power samples)
2. Loads power pilot CSV (experiment timestamps)
3. Correlates: For each experiment, what was the average CPU power?
4. Calculates energy consumed per experiment
5. Outputs enriched CSV with power/energy metrics

Usage:
    python analysis/parse_powermetrics.py <powermetrics_log> <pilot_csv>

Example:
    python analysis/parse_powermetrics.py \
        results/phase1_power_consumption/powermetrics_20251102_143000.txt \
        results/phase1_power_consumption/power_pilot_raw_20251102_143000.csv
"""

import re
import sys
from datetime import datetime
from pathlib import Path
import csv

def parse_powermetrics_log(log_path):
    """
    Parse powermetrics log file and extract CPU power samples.

    Returns:
        list of dicts: [{'timestamp': datetime, 'cpu_power_mw': float}, ...]
    """
    samples = []

    with open(log_path, 'r') as f:
        current_timestamp = None

        for line in f:
            # Match timestamp line: *** Sampled system activity (Sat Nov  2 14:30:05 2025 -0700) ***
            timestamp_match = re.search(r'\*\*\* Sampled system activity \(([^)]+)\)', line)
            if timestamp_match:
                timestamp_str = timestamp_match.group(1)
                # Parse: "Sat Nov  2 14:30:05 2025 -0700"
                # Note: Day may have single or double digit
                try:
                    current_timestamp = datetime.strptime(timestamp_str.strip(), "%a %b %d %H:%M:%S %Y %z")
                except ValueError:
                    # Try with single-digit day
                    try:
                        # Handle extra spaces in day field
                        normalized = re.sub(r'\s+', ' ', timestamp_str.strip())
                        current_timestamp = datetime.strptime(normalized, "%a %b %d %H:%M:%S %Y %z")
                    except ValueError as e:
                        print(f"Warning: Could not parse timestamp: {timestamp_str}: {e}", file=sys.stderr)
                        continue

            # Match CPU Power line: CPU Power: 12450 mW
            power_match = re.search(r'CPU Power:\s+(\d+)\s+mW', line)
            if power_match and current_timestamp:
                cpu_power_mw = float(power_match.group(1))
                samples.append({
                    'timestamp': current_timestamp,
                    'cpu_power_mw': cpu_power_mw
                })

    print(f"Parsed {len(samples)} power samples from powermetrics log", file=sys.stderr)
    return samples

def load_pilot_csv(csv_path):
    """
    Load power pilot CSV with experiment results.

    Returns:
        list of dicts: experiment data with timestamp
    """
    experiments = []

    with open(csv_path, 'r') as f:
        reader = csv.DictReader(f)
        for row in reader:
            # Parse timestamp: "2025-11-02T14:30:00"
            timestamp = datetime.fromisoformat(row['timestamp'])

            row['timestamp'] = timestamp
            row['loop_duration_s'] = float(row['loop_duration_s'])
            row['num_sequences'] = int(row['num_sequences'])
            row['iterations'] = int(row['iterations'])
            row['sequences_processed'] = int(row['sequences_processed'])
            row['throughput_seqs_per_sec'] = float(row['throughput_seqs_per_sec'])

            experiments.append(row)

    print(f"Loaded {len(experiments)} experiments from pilot CSV", file=sys.stderr)
    return experiments

def correlate_power_with_experiments(power_samples, experiments):
    """
    For each experiment, find power samples during its execution window.
    Calculate average power and energy consumed.

    Args:
        power_samples: list of {'timestamp': datetime, 'cpu_power_mw': float}
        experiments: list of experiment dicts with 'timestamp' and 'loop_duration_s'

    Returns:
        list of experiments enriched with power/energy metrics
    """
    enriched = []

    for exp in experiments:
        exp_start = exp['timestamp']
        exp_end = exp_start.timestamp() + exp['loop_duration_s']
        exp_start_ts = exp_start.timestamp()

        # Find power samples during experiment window
        exp_power_samples = [
            s['cpu_power_mw'] for s in power_samples
            if exp_start_ts <= s['timestamp'].timestamp() <= exp_end
        ]

        if not exp_power_samples:
            print(f"Warning: No power samples for experiment at {exp_start}", file=sys.stderr)
            avg_power_mw = 0.0
        else:
            avg_power_mw = sum(exp_power_samples) / len(exp_power_samples)

        # Calculate energy
        avg_power_w = avg_power_mw / 1000.0
        energy_wh = avg_power_w * (exp['loop_duration_s'] / 3600.0)
        energy_per_seq_uwh = (energy_wh * 1e6) / exp['sequences_processed'] if exp['sequences_processed'] > 0 else 0.0

        # Add metrics to experiment
        exp_copy = exp.copy()
        exp_copy['cpu_power_mw'] = avg_power_mw
        exp_copy['cpu_power_w'] = avg_power_w
        exp_copy['energy_wh'] = energy_wh
        exp_copy['energy_per_seq_uwh'] = energy_per_seq_uwh
        exp_copy['power_samples_count'] = len(exp_power_samples)

        enriched.append(exp_copy)

    return enriched

def calculate_energy_efficiency(enriched_experiments):
    """
    Calculate energy efficiency metrics relative to naive baseline.

    Energy efficiency = (time_naive / time_optimized) / (energy_naive / energy_optimized)

    Efficiency = 1.0: Energy scales with time (ideal)
    Efficiency > 1.0: Better energy savings than time savings
    Efficiency < 1.0: Worse energy savings (power-hungry optimization)
    """
    # Group by operation and scale
    groups = {}
    for exp in enriched_experiments:
        key = (exp['operation'], exp['scale'])
        if key not in groups:
            groups[key] = []
        groups[key].append(exp)

    # For each group, calculate efficiency vs naive
    result = []
    for (operation, scale), exps in groups.items():
        # Find naive baseline
        naive = next((e for e in exps if e['config'] == 'naive'), None)
        if not naive:
            print(f"Warning: No naive baseline for {operation} {scale}", file=sys.stderr)
            # Add experiments without efficiency metric
            result.extend(exps)
            continue

        naive_time = naive['loop_duration_s'] / naive['iterations']  # Time per iteration
        naive_energy = naive['energy_per_seq_uwh']

        for exp in exps:
            exp_time = exp['loop_duration_s'] / exp['iterations']
            exp_energy = exp['energy_per_seq_uwh']

            if exp['config'] == 'naive':
                time_speedup = 1.0
                energy_speedup = 1.0
                energy_efficiency = 1.0
            else:
                time_speedup = naive_time / exp_time if exp_time > 0 else 0.0
                energy_speedup = naive_energy / exp_energy if exp_energy > 0 else 0.0
                energy_efficiency = time_speedup / energy_speedup if energy_speedup > 0 else 0.0

            exp['time_speedup_vs_naive'] = time_speedup
            exp['energy_speedup_vs_naive'] = energy_speedup
            exp['energy_efficiency'] = energy_efficiency

            result.append(exp)

    return result

def save_enriched_csv(experiments, output_path):
    """Save enriched experiments to CSV."""
    if not experiments:
        print("Error: No experiments to save", file=sys.stderr)
        return

    # Define field order
    fieldnames = [
        'operation', 'config', 'scale', 'num_sequences', 'loop_duration_s', 'iterations',
        'sequences_processed', 'throughput_seqs_per_sec',
        'cpu_power_mw', 'cpu_power_w', 'energy_wh', 'energy_per_seq_uwh',
        'time_speedup_vs_naive', 'energy_speedup_vs_naive', 'energy_efficiency',
        'power_samples_count', 'timestamp'
    ]

    with open(output_path, 'w', newline='') as f:
        writer = csv.DictWriter(f, fieldnames=fieldnames)
        writer.writeheader()

        for exp in experiments:
            # Convert timestamp back to string
            row = exp.copy()
            row['timestamp'] = exp['timestamp'].isoformat()
            writer.writerow(row)

    print(f"Saved enriched CSV to: {output_path}", file=sys.stderr)

def main():
    if len(sys.argv) != 3:
        print("Usage: python parse_powermetrics.py <powermetrics_log> <pilot_csv>")
        sys.exit(1)

    powermetrics_log = Path(sys.argv[1])
    pilot_csv = Path(sys.argv[2])

    if not powermetrics_log.exists():
        print(f"Error: powermetrics log not found: {powermetrics_log}")
        sys.exit(1)

    if not pilot_csv.exists():
        print(f"Error: pilot CSV not found: {pilot_csv}")
        sys.exit(1)

    print("Parsing powermetrics log...", file=sys.stderr)
    power_samples = parse_powermetrics_log(powermetrics_log)

    print("Loading pilot CSV...", file=sys.stderr)
    experiments = load_pilot_csv(pilot_csv)

    print("Correlating power samples with experiments...", file=sys.stderr)
    enriched = correlate_power_with_experiments(power_samples, experiments)

    print("Calculating energy efficiency metrics...", file=sys.stderr)
    with_efficiency = calculate_energy_efficiency(enriched)

    # Save enriched CSV
    output_path = pilot_csv.parent / f"power_enriched_{pilot_csv.stem.replace('power_pilot_raw_', '')}.csv"
    save_enriched_csv(with_efficiency, output_path)

    print("", file=sys.stderr)
    print("✅ Analysis complete!", file=sys.stderr)
    print(f"   Enriched CSV: {output_path}", file=sys.stderr)
    print("", file=sys.stderr)
    print("Next: python analysis/generate_power_findings.py", file=sys.stderr)

if __name__ == '__main__':
    main()
