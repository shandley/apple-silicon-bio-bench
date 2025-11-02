#!/usr/bin/env python3
"""
AMX Dimension Analysis
Analyzes Apple Matrix Coprocessor performance for matrix-amenable operations
"""

import pandas as pd
import matplotlib.pyplot as plt
import seaborn as sns
import numpy as np
from pathlib import Path

# Set style
sns.set_style("whitegrid")
plt.rcParams['figure.figsize'] = (12, 8)
plt.rcParams['font.size'] = 10

# Load data
data_file = Path("results/phase1_amx_dimension/amx_clean.csv")
print(f"Loading AMX data from: {data_file}")

# Read CSV
df = pd.read_csv(data_file)
print(f"Loaded {len(df)} experiments")
print(f"\nOperations: {sorted(df['operation'].unique())}")
print(f"Backends: {sorted(df['backend'].unique())}")
print(f"Scales: {sorted(df['scale'].unique())}")

# Create output directory
output_dir = Path("results/amx_analysis")
output_dir.mkdir(exist_ok=True)

print("\n" + "="*70)
print("AMX SPEEDUP ANALYSIS")
print("="*70 + "\n")

# Analysis by operation
for operation in sorted(df['operation'].unique()):
    op_data = df[df['operation'] == operation].copy()
    complexity = op_data['complexity'].iloc[0]

    print(f"\nOperation: {operation} (complexity {complexity:.2f})")
    print("-" * 60)

    # Create pivot tables
    pivot_naive = op_data.pivot_table(
        values='speedup_vs_naive',
        index='backend',
        columns='scale',
        aggfunc='mean'
    )

    pivot_neon = op_data.pivot_table(
        values='speedup_vs_neon',
        index='backend',
        columns='scale',
        aggfunc='mean'
    )

    print("\nSpeedup vs Naive:")
    print(pivot_naive.to_string())

    print("\nSpeedup vs NEON:")
    print(pivot_neon.to_string())

# Generate visualizations
print("\n" + "="*70)
print("GENERATING VISUALIZATIONS")
print("="*70 + "\n")

# 1. Speedup curves per operation
fig, axes = plt.subplots(1, 3, figsize=(18, 5))
for idx, operation in enumerate(sorted(df['operation'].unique())):
    op_data = df[df['operation'] == operation]
    ax = axes[idx]

    # Scale mapping
    scale_order = ['Tiny', 'Small', 'Medium', 'Large', 'VeryLarge', 'Huge']
    scale_nums = {s: i for i, s in enumerate(scale_order)}

    for backend in ['naive', 'neon', 'amx', 'parallel_amx']:
        backend_data = op_data[op_data['backend'] == backend].sort_values('num_sequences')
        x = [scale_nums[s] for s in backend_data['scale']]
        y = backend_data['speedup_vs_naive']
        ax.plot(x, y, 'o-', linewidth=2, markersize=8, label=backend)

    ax.set_xlabel('Scale', fontsize=12, fontweight='bold')
    ax.set_ylabel('Speedup vs Naive', fontsize=12, fontweight='bold')
    ax.set_title(f'{operation}\n(complexity {op_data["complexity"].iloc[0]:.2f})',
                 fontsize=14, fontweight='bold')
    ax.set_xticks(range(len(scale_order)))
    ax.set_xticklabels(scale_order, rotation=45)
    ax.legend()
    ax.grid(True, alpha=0.3)

plt.tight_layout()
fig.savefig(output_dir / "amx_speedup_curves.png", dpi=300, bbox_inches='tight')
print(f"Saved: {output_dir / 'amx_speedup_curves.png'}")

# 2. AMX vs NEON comparison
fig, ax = plt.subplots(figsize=(10, 6))
operations = sorted(df['operation'].unique())
x = np.arange(len(operations))
width = 0.35

# Get VeryLarge scale data
amx_speedups = []
neon_speedups = []
for op in operations:
    op_data = df[(df['operation'] == op) & (df['scale'] == 'VeryLarge')]
    amx_speedups.append(op_data[op_data['backend'] == 'amx']['speedup_vs_naive'].iloc[0])
    neon_speedups.append(op_data[op_data['backend'] == 'neon']['speedup_vs_naive'].iloc[0])

ax.bar(x - width/2, neon_speedups, width, label='NEON', color='#2ecc71')
ax.bar(x + width/2, amx_speedups, width, label='AMX', color='#e74c3c')

ax.set_xlabel('Operation', fontsize=12, fontweight='bold')
ax.set_ylabel('Speedup vs Naive (VeryLarge scale)', fontsize=12, fontweight='bold')
ax.set_title('AMX vs NEON Speedup Comparison', fontsize=14, fontweight='bold')
ax.set_xticks(x)
ax.set_xticklabels(operations, rotation=45, ha='right')
ax.legend()
ax.grid(True, alpha=0.3, axis='y')

plt.tight_layout()
fig.savefig(output_dir / "amx_vs_neon_comparison.png", dpi=300, bbox_inches='tight')
print(f"Saved: {output_dir / 'amx_vs_neon_comparison.png'}")

# 3. Parallel AMX effectiveness
fig, ax = plt.subplots(figsize=(10, 6))
for operation in sorted(df['operation'].unique()):
    op_data = df[(df['operation'] == operation) & (df['backend'] == 'parallel_amx')]
    op_data = op_data.sort_values('num_sequences')

    scale_nums = [scale_nums[s] for s in op_data['scale']]
    speedups = op_data['speedup_vs_naive']

    ax.plot(scale_nums, speedups, 'o-', linewidth=2, markersize=8, label=operation)

ax.set_xlabel('Scale', fontsize=12, fontweight='bold')
ax.set_ylabel('Speedup vs Naive', fontsize=12, fontweight='bold')
ax.set_title('Parallel AMX Scaling', fontsize=14, fontweight='bold')
ax.set_xticks(range(len(scale_order)))
ax.set_xticklabels(scale_order, rotation=45)
ax.legend()
ax.grid(True, alpha=0.3)

plt.tight_layout()
fig.savefig(output_dir / "parallel_amx_scaling.png", dpi=300, bbox_inches='tight')
print(f"Saved: {output_dir / 'parallel_amx_scaling.png'}")

# Generate summary statistics
print("\n" + "="*70)
print("SUMMARY STATISTICS")
print("="*70 + "\n")

summary_file = output_dir / "amx_summary.txt"
with open(summary_file, 'w') as f:
    f.write("AMX DIMENSION SUMMARY STATISTICS\n")
    f.write("="*70 + "\n\n")

    f.write("Maximum Speedups (VeryLarge scale):\n")
    f.write("-" * 60 + "\n")
    for op in sorted(df['operation'].unique()):
        op_data = df[(df['operation'] == op) & (df['scale'] == 'VeryLarge')]
        f.write(f"\n{op}:\n")
        for backend in ['neon', 'amx', 'parallel_amx']:
            speedup = op_data[op_data['backend'] == backend]['speedup_vs_naive'].iloc[0]
            f.write(f"  {backend:15s}: {speedup:6.2f}× vs naive\n")

    f.write("\n\nAMX Effectiveness (AMX / NEON speedup ratio):\n")
    f.write("-" * 60 + "\n")
    for op in sorted(df['operation'].unique()):
        op_data = df[(df['operation'] == op) & (df['scale'] == 'VeryLarge')]
        neon_speedup = op_data[op_data['backend'] == 'neon']['speedup_vs_naive'].iloc[0]
        amx_speedup = op_data[op_data['backend'] == 'amx']['speedup_vs_naive'].iloc[0]
        ratio = amx_speedup / neon_speedup
        f.write(f"{op:25s}: {ratio:5.2f}× ({amx_speedup:.2f}× AMX / {neon_speedup:.2f}× NEON)\n")

print(f"Saved: {summary_file}")

# Decision rules
print("\n" + "="*70)
print("AMX DECISION RULES")
print("="*70 + "\n")

rules_file = output_dir / "amx_decision_rules.txt"
with open(rules_file, 'w') as f:
    f.write("AMX OPTIMIZATION DECISION RULES\n")
    f.write("="*70 + "\n\n")

    f.write("RULE 1: Matrix-Native Operations\n")
    f.write("-" * 60 + "\n")
    f.write("Use AMX when:\n")
    f.write("  - Operation involves matrix computations (DP, statistics)\n")
    f.write("  - Parallel AMX shows >10× speedup\n")
    f.write("  - Data scale > 1,000 sequences (overhead amortized)\n\n")

    f.write("RULE 2: NEON vs AMX Selection\n")
    f.write("-" * 60 + "\n")
    f.write("Compare:\n")
    f.write("  - If NEON >5×: Use NEON (simpler, more portable)\n")
    f.write("  - If AMX >5× and NEON <2×: Use AMX\n")
    f.write("  - If parallel_amx >10×: Use parallel AMX\n\n")

    f.write("RULE 3: Scale Thresholds\n")
    f.write("-" * 60 + "\n")
    f.write("  - <1,000 sequences: AMX overhead dominates, use NEON\n")
    f.write("  - >10,000 sequences: Parallel AMX shows benefit\n")
    f.write("  - >100,000 sequences: Maximum parallel AMX effectiveness\n")

print(f"Saved: {rules_file}")

print("\n" + "="*70)
print("ANALYSIS COMPLETE")
print("="*70 + "\n")
print(f"All outputs saved to: {output_dir}")
