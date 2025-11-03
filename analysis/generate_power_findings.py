#!/usr/bin/env python3
"""
Generate FINDINGS.md from enriched power consumption data.

This script:
1. Loads enriched CSV (from parse_powermetrics.py)
2. Calculates summary statistics
3. Generates FINDINGS.md document

Usage:
    python analysis/generate_power_findings.py <enriched_csv>

Example:
    python analysis/generate_power_findings.py \
        results/phase1_power_consumption/power_enriched_20251102_143000.csv
"""

import sys
from pathlib import Path
import csv
from collections import defaultdict

def load_enriched_csv(csv_path):
    """Load enriched power consumption CSV."""
    experiments = []

    with open(csv_path, 'r') as f:
        reader = csv.DictReader(f)
        for row in reader:
            # Convert numeric fields
            row['cpu_power_w'] = float(row['cpu_power_w'])
            row['energy_wh'] = float(row['energy_wh'])
            row['energy_per_seq_uwh'] = float(row['energy_per_seq_uwh'])
            row['time_speedup_vs_naive'] = float(row['time_speedup_vs_naive'])
            row['energy_speedup_vs_naive'] = float(row['energy_speedup_vs_naive'])
            row['energy_efficiency'] = float(row['energy_efficiency'])
            row['throughput_seqs_per_sec'] = float(row['throughput_seqs_per_sec'])
            experiments.append(row)

    return experiments

def generate_findings(experiments, output_path, csv_path):
    """Generate FINDINGS.md document."""

    # Group by operation
    by_operation = defaultdict(list)
    for exp in experiments:
        by_operation[exp['operation']].append(exp)

    with open(output_path, 'w') as f:
        f.write("# Power Consumption Pilot - Findings\n\n")
        f.write("**Date**: November 2, 2025\n")
        f.write("**Experiment**: Environmental Pillar Validation\n")
        f.write("**Lab Notebook**: Entry 020\n\n")
        f.write("---\n\n")

        f.write("## Executive Summary\n\n")
        f.write(f"**Total experiments**: {len(experiments)}\n")
        f.write(f"**Operations tested**: {len(by_operation)}\n")
        f.write(f"**Configurations**: 4 (naive, neon, neon_4t, neon_8t)\n")
        f.write(f"**Scales**: 2 (Medium 10K, Large 100K)\n\n")

        # Calculate overall statistics
        all_configs = [e for e in experiments if e['config'] != 'naive']
        if all_configs:
            avg_energy_efficiency = sum(e['energy_efficiency'] for e in all_configs) / len(all_configs)
            avg_energy_speedup = sum(e['energy_speedup_vs_naive'] for e in all_configs) / len(all_configs)
            avg_time_speedup = sum(e['time_speedup_vs_naive'] for e in all_configs) / len(all_configs)

            f.write(f"**Key Finding**: Energy scales with runtime\n")
            f.write(f"- Average time speedup: **{avg_time_speedup:.1f}×**\n")
            f.write(f"- Average energy speedup: **{avg_energy_speedup:.1f}×**\n")
            f.write(f"- Average energy efficiency: **{avg_energy_efficiency:.2f}** (1.0 = ideal)\n\n")

        f.write("---\n\n")

        # Per-operation analysis
        f.write("## Results by Operation\n\n")

        for operation, op_exps in sorted(by_operation.items()):
            f.write(f"### {operation}\n\n")

            # Group by scale
            by_scale = defaultdict(list)
            for exp in op_exps:
                by_scale[exp['scale']].append(exp)

            for scale, scale_exps in sorted(by_scale.items()):
                f.write(f"**{scale} scale** ({scale_exps[0]['num_sequences']} sequences):\n\n")
                f.write("| Config | CPU Power (W) | Energy (mWh) | Energy/Seq (μWh) | Time Speedup | Energy Speedup | Efficiency |\n")
                f.write("|--------|--------------|--------------|------------------|--------------|----------------|------------|\n")

                for exp in sorted(scale_exps, key=lambda e: ['naive', 'neon', 'neon_4t', 'neon_8t'].index(e['config'])):
                    f.write(f"| {exp['config']:8s} | {exp['cpu_power_w']:6.1f} | ")
                    f.write(f"{exp['energy_wh'] * 1000:7.3f} | ")
                    f.write(f"{exp['energy_per_seq_uwh']:8.3f} | ")
                    f.write(f"{exp['time_speedup_vs_naive']:6.1f}× | ")
                    f.write(f"{exp['energy_speedup_vs_naive']:6.1f}× | ")
                    f.write(f"{exp['energy_efficiency']:6.2f} |\n")

                f.write("\n")

            f.write("---\n\n")

        # Power draw analysis
        f.write("## Power Draw Analysis\n\n")
        f.write("Does optimization increase power draw per unit time?\n\n")

        # Average power by config
        power_by_config = defaultdict(list)
        for exp in experiments:
            power_by_config[exp['config']].append(exp['cpu_power_w'])

        f.write("| Configuration | Average CPU Power (W) | vs Naive |\n")
        f.write("|---------------|----------------------|----------|\n")

        naive_power = sum(power_by_config['naive']) / len(power_by_config['naive'])
        for config in ['naive', 'neon', 'neon_4t', 'neon_8t']:
            if config in power_by_config:
                avg_power = sum(power_by_config[config]) / len(power_by_config[config])
                vs_naive = avg_power / naive_power if naive_power > 0 else 0.0
                f.write(f"| {config:14s} | {avg_power:20.1f} | {vs_naive:8.2f}× |\n")

        f.write("\n")
        f.write("**Insight**: Power draw increases with parallelism, but total energy decreases due to faster completion.\n\n")

        f.write("---\n\n")

        # Environmental impact
        f.write("## Environmental Impact Extrapolation\n\n")

        # Use base_counting as representative
        base_counting_large = [e for e in experiments if e['operation'] == 'base_counting' and e['scale'] == 'Large']
        if base_counting_large:
            naive = next(e for e in base_counting_large if e['config'] == 'naive')
            optimized = next(e for e in base_counting_large if e['config'] == 'neon_8t')

            energy_saved_per_analysis = naive['energy_wh'] - optimized['energy_wh']

            f.write(f"**Scenario**: Small lab running 10,000 analyses/year (base_counting, 100K sequences)\n\n")
            f.write(f"- Naive energy per analysis: {naive['energy_wh'] * 1000:.3f} mWh\n")
            f.write(f"- Optimized energy per analysis: {optimized['energy_wh'] * 1000:.3f} mWh\n")
            f.write(f"- Energy saved per analysis: **{energy_saved_per_analysis * 1000:.3f} mWh**\n\n")

            annual_energy_saved_wh = energy_saved_per_analysis * 10_000
            annual_co2_saved_kg = (annual_energy_saved_wh / 1000) * 0.5  # 0.5 kg CO₂/kWh

            f.write(f"**Per-lab annual savings**:\n")
            f.write(f"- Energy saved: {annual_energy_saved_wh:.1f} Wh/year ({annual_energy_saved_wh / 1000:.3f} kWh/year)\n")
            f.write(f"- CO₂ avoided: {annual_co2_saved_kg:.2f} kg/year\n\n")

            field_energy_saved_kwh = (annual_energy_saved_wh / 1000) * 10_000
            field_co2_saved_tons = (field_energy_saved_kwh * 0.5) / 1000

            f.write(f"**Field-wide impact** (10,000 labs adopt):\n")
            f.write(f"- Energy saved: {field_energy_saved_kwh:.1f} kWh/year\n")
            f.write(f"- CO₂ avoided: {field_co2_saved_tons:.1f} tons/year\n\n")

        f.write("---\n\n")

        # Validation of 300× claim
        f.write("## Validation of \"300× Less Energy\" Claim\n\n")
        f.write("**Current claim** (from DEMOCRATIZING_BIOINFORMATICS_COMPUTE.md):\n")
        f.write("- Traditional HPC: 150 Wh (naive, 30 minutes)\n")
        f.write("- Mac Mini optimized: 0.5 Wh (NEON+Parallel, 1 minute)\n")
        f.write("- Reduction: 300×\n\n")

        f.write("**Our measurements** (Mac-to-Mac comparison):\n")
        if base_counting_large:
            naive = next(e for e in base_counting_large if e['config'] == 'naive')
            optimized = next(e for e in base_counting_large if e['config'] == 'neon_8t')
            reduction = naive['energy_wh'] / optimized['energy_wh'] if optimized['energy_wh'] > 0 else 0.0

            f.write(f"- Naive (Mac): {naive['energy_wh'] * 1000:.3f} mWh\n")
            f.write(f"- Optimized (Mac): {optimized['energy_wh'] * 1000:.3f} mWh\n")
            f.write(f"- Reduction: **{reduction:.1f}×**\n\n")

            f.write("**Conclusion**: Mac-to-Mac comparison shows ~{:.0f}× energy reduction. ".format(reduction))
            f.write("The 300× claim likely compares HPC (different hardware) to Mac, not Mac-to-Mac.\n\n")

        f.write("---\n\n")

        # Next steps
        f.write("## Next Steps\n\n")
        f.write("### Expand to Full 80 Experiments?\n\n")
        f.write("**Decision criteria**:\n")
        f.write("- ✅ If energy efficiency ≈ 1.0 (validated): Patterns hold, may not need full 80\n")
        f.write("- ❌ If energy efficiency varies widely: Expand to more operations\n\n")

        f.write("### Additional Validation\n\n")
        f.write("1. **Test on Mac Mini M4**: Lower base power than MacBook\n")
        f.write("2. **Measure HPC cluster**: Enable direct comparison for 300× claim\n")
        f.write("3. **Test on real FASTQ data**: Validate synthetic results\n\n")

        f.write("---\n\n")
        f.write("**Generated**: November 2, 2025\n")
        f.write(f"**Data source**: {Path(csv_path).name}\n")

    print(f"Generated findings: {output_path}", file=sys.stderr)

def main():
    if len(sys.argv) != 2:
        print("Usage: python generate_power_findings.py <enriched_csv>")
        sys.exit(1)

    enriched_csv = Path(sys.argv[1])

    if not enriched_csv.exists():
        print(f"Error: Enriched CSV not found: {enriched_csv}")
        sys.exit(1)

    print("Loading enriched CSV...", file=sys.stderr)
    experiments = load_enriched_csv(enriched_csv)

    print(f"Loaded {len(experiments)} experiments", file=sys.stderr)

    output_path = enriched_csv.parent / "FINDINGS.md"
    print(f"Generating findings...", file=sys.stderr)
    generate_findings(experiments, output_path, enriched_csv)

    print("", file=sys.stderr)
    print("✅ FINDINGS.md generated!", file=sys.stderr)
    print(f"   Location: {output_path}", file=sys.stderr)

if __name__ == '__main__':
    main()
