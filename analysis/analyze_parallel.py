#!/usr/bin/env python3
"""
Analyze parallel/threading dimension pilot results.

Generates:
- Speedup matrices per operation
- Visualizations (speedup curves, efficiency plots)
- P-core vs E-core comparisons
- Decision rules
"""

import pandas as pd
import matplotlib.pyplot as plt
import seaborn as sns
import numpy as np
from pathlib import Path

# Set style
sns.set_style("whitegrid")
plt.rcParams['figure.figsize'] = (14, 8)
plt.rcParams['font.size'] = 10

# Load data
csv_path = Path("results/parallel_dimension_raw_20251031_152922.csv")
df = pd.read_csv(csv_path)

print(f"Loaded {len(df)} experiments")
print(f"Operations: {df['operation'].unique()}")
print(f"Scales: {df['scale'].unique()}")
print(f"Thread counts: {sorted(df['threads'].unique())}")
print(f"Assignments: {df['assignment'].unique()}")
print()

# Create output directory
output_dir = Path("results/parallel_analysis")
output_dir.mkdir(exist_ok=True)

# ============================================================================
# 1. SPEEDUP MATRICES PER OPERATION
# ============================================================================

print("=" * 70)
print("SPEEDUP MATRICES BY OPERATION")
print("=" * 70)
print()

with open(output_dir / "speedup_matrices.txt", "w") as f:
    for operation in sorted(df['operation'].unique()):
        op_data = df[df['operation'] == operation]
        complexity = op_data['complexity'].iloc[0]

        print(f"\n{'='*70}")
        print(f"Operation: {operation} (complexity {complexity:.2f})")
        print(f"{'='*70}\n")

        f.write(f"\n{'='*70}\n")
        f.write(f"Operation: {operation} (complexity {complexity:.2f})\n")
        f.write(f"{'='*70}\n\n")

        # Create pivot table: rows=config, cols=scale
        for assignment in ['default', 'p_cores', 'e_cores']:
            config_data = op_data[op_data['assignment'] == assignment]

            if len(config_data) == 0:
                continue

            pivot = config_data.pivot_table(
                values='speedup_vs_1t',
                index='threads',
                columns='scale',
                aggfunc='mean'
            )

            # Reorder columns by num_sequences
            scale_order = ['Tiny', 'Small', 'Medium', 'Large', 'VeryLarge', 'Huge']
            pivot = pivot[[col for col in scale_order if col in pivot.columns]]

            print(f"{assignment.upper():15s}  ", end="")
            f.write(f"{assignment.upper():15s}  ")
            for col in pivot.columns:
                print(f"{col:>12s}", end="")
                f.write(f"{col:>12s}")
            print()
            f.write("\n")
            print("-" * (15 + 12 * len(pivot.columns)))
            f.write("-" * (15 + 12 * len(pivot.columns)) + "\n")

            for threads in sorted(pivot.index):
                print(f"{threads}t {assignment:13s}  ", end="")
                f.write(f"{threads}t {assignment:13s}  ")
                for col in pivot.columns:
                    speedup = pivot.loc[threads, col]
                    if pd.notna(speedup):
                        print(f"{speedup:11.2f}×", end="")
                        f.write(f"{speedup:11.2f}×")
                    else:
                        print(f"{'N/A':>12s}", end="")
                        f.write(f"{'N/A':>12s}")
                print()
                f.write("\n")
            print()
            f.write("\n")

print(f"\nSpeedup matrices saved to: {output_dir / 'speedup_matrices.txt'}")

# ============================================================================
# 2. VISUALIZATIONS
# ============================================================================

print("\n" + "=" * 70)
print("GENERATING VISUALIZATIONS")
print("=" * 70)
print()

# Figure 1: Speedup curves for all operations (8 threads, default)
fig, axes = plt.subplots(2, 5, figsize=(20, 8))
axes = axes.flatten()

scale_order = ['Tiny', 'Small', 'Medium', 'Large', 'VeryLarge', 'Huge']
num_seqs_map = {
    'Tiny': 100,
    'Small': 1000,
    'Medium': 10000,
    'Large': 100000,
    'VeryLarge': 1000000,
    'Huge': 10000000
}

for idx, operation in enumerate(sorted(df['operation'].unique())):
    ax = axes[idx]
    op_data = df[(df['operation'] == operation) & (df['assignment'] == 'default')]
    complexity = op_data['complexity'].iloc[0]

    # Plot speedup vs scale for each thread count
    for threads in [1, 2, 4, 8]:
        thread_data = op_data[op_data['threads'] == threads]

        # Order by scale
        thread_data = thread_data.set_index('scale')
        thread_data = thread_data.reindex(scale_order)

        x = [num_seqs_map[s] for s in thread_data.index if s in num_seqs_map]
        y = thread_data['speedup_vs_1t'].values

        if threads == 1:
            ax.plot(x, y, 'o-', linewidth=2, markersize=6, label=f'{threads}t', alpha=0.3)
        else:
            ax.plot(x, y, 'o-', linewidth=2, markersize=6, label=f'{threads}t')

    ax.set_xscale('log')
    ax.set_xlabel('Sequences (log scale)')
    ax.set_ylabel('Speedup vs 1t')
    ax.set_title(f'{operation}\n(complexity {complexity:.2f})', fontsize=10)
    ax.legend(fontsize=8)
    ax.grid(True, alpha=0.3)
    ax.axhline(y=1, color='gray', linestyle='--', linewidth=1, alpha=0.5)

plt.tight_layout()
plt.savefig(output_dir / 'speedup_curves_all_ops.png', dpi=300, bbox_inches='tight')
print(f"Saved: {output_dir / 'speedup_curves_all_ops.png'}")
plt.close()

# Figure 2: P-core vs E-core comparison (8 threads, large scale)
fig, ax = plt.subplots(figsize=(14, 8))

operations = sorted(df['operation'].unique())
x_pos = np.arange(len(operations))
width = 0.25

large_scale = 'Huge'
data_8t = df[(df['threads'] == 8) & (df['scale'] == large_scale)]

speedups_default = []
speedups_p = []
speedups_e = []

for op in operations:
    op_data = data_8t[data_8t['operation'] == op]
    speedups_default.append(op_data[op_data['assignment'] == 'default']['speedup_vs_1t'].mean())
    speedups_p.append(op_data[op_data['assignment'] == 'p_cores']['speedup_vs_1t'].mean())
    speedups_e.append(op_data[op_data['assignment'] == 'e_cores']['speedup_vs_1t'].mean())

ax.bar(x_pos - width, speedups_default, width, label='Default', alpha=0.8)
ax.bar(x_pos, speedups_p, width, label='P-cores', alpha=0.8)
ax.bar(x_pos + width, speedups_e, width, label='E-cores', alpha=0.8)

ax.set_ylabel('Speedup vs 1t')
ax.set_title(f'Core Assignment Comparison (8 threads, {large_scale} scale - 10M sequences)', fontsize=14, fontweight='bold')
ax.set_xticks(x_pos)
ax.set_xticklabels(operations, rotation=45, ha='right')
ax.legend()
ax.grid(True, alpha=0.3, axis='y')
ax.axhline(y=1, color='gray', linestyle='--', linewidth=1, alpha=0.5)

plt.tight_layout()
plt.savefig(output_dir / 'core_assignment_comparison.png', dpi=300, bbox_inches='tight')
print(f"Saved: {output_dir / 'core_assignment_comparison.png'}")
plt.close()

# Figure 3: Efficiency (speedup/threads) heatmap
fig, ax = plt.subplots(figsize=(14, 10))

# Calculate average efficiency across all scales for 8 threads, default
efficiency_data = []
for op in sorted(df['operation'].unique()):
    op_data = df[(df['operation'] == op) & (df['threads'] == 8) & (df['assignment'] == 'default')]
    for scale in scale_order:
        scale_data = op_data[op_data['scale'] == scale]
        if len(scale_data) > 0:
            efficiency = scale_data['efficiency'].mean()
            efficiency_data.append({
                'operation': op,
                'scale': scale,
                'efficiency': efficiency
            })

eff_df = pd.DataFrame(efficiency_data)
pivot_eff = eff_df.pivot(index='operation', columns='scale', values='efficiency')
pivot_eff = pivot_eff[scale_order]

sns.heatmap(pivot_eff, annot=True, fmt='.2f', cmap='RdYlGn', center=0.5,
            vmin=0, vmax=1.0, ax=ax, cbar_kws={'label': 'Efficiency (speedup/threads)'})
ax.set_title('Parallel Efficiency (8 threads, default) - Higher is Better', fontsize=14, fontweight='bold')
ax.set_xlabel('Scale')
ax.set_ylabel('Operation')

plt.tight_layout()
plt.savefig(output_dir / 'efficiency_heatmap.png', dpi=300, bbox_inches='tight')
print(f"Saved: {output_dir / 'efficiency_heatmap.png'}")
plt.close()

# Figure 4: Complexity vs Max Speedup scatter
fig, ax = plt.subplots(figsize=(12, 8))

max_speedups = []
for op in sorted(df['operation'].unique()):
    op_data = df[df['operation'] == op]
    complexity = op_data['complexity'].iloc[0]
    max_speedup = op_data['speedup_vs_1t'].max()
    max_speedups.append({
        'operation': op,
        'complexity': complexity,
        'max_speedup': max_speedup
    })

max_df = pd.DataFrame(max_speedups)

ax.scatter(max_df['complexity'], max_df['max_speedup'], s=200, alpha=0.6, edgecolors='black', linewidth=2)

for _, row in max_df.iterrows():
    ax.annotate(row['operation'],
                (row['complexity'], row['max_speedup']),
                xytext=(5, 5), textcoords='offset points', fontsize=9)

ax.set_xlabel('Operation Complexity', fontsize=12)
ax.set_ylabel('Maximum Speedup Achieved', fontsize=12)
ax.set_title('Complexity vs Maximum Parallel Speedup', fontsize=14, fontweight='bold')
ax.grid(True, alpha=0.3)

# Add trend line
z = np.polyfit(max_df['complexity'], max_df['max_speedup'], 1)
p = np.poly1d(z)
x_line = np.linspace(max_df['complexity'].min(), max_df['complexity'].max(), 100)
ax.plot(x_line, p(x_line), "r--", alpha=0.5, linewidth=2, label=f'Trend: y={z[0]:.2f}x+{z[1]:.2f}')
ax.legend()

plt.tight_layout()
plt.savefig(output_dir / 'complexity_vs_speedup.png', dpi=300, bbox_inches='tight')
print(f"Saved: {output_dir / 'complexity_vs_speedup.png'}")
plt.close()

# Figure 5: Thread scaling comparison (selected operations)
fig, axes = plt.subplots(2, 3, figsize=(18, 10))
axes = axes.flatten()

selected_ops = ['base_counting', 'complexity_score', 'reverse_complement',
                'sequence_length', 'quality_aggregation', 'n_content']

for idx, op in enumerate(selected_ops):
    ax = axes[idx]
    op_data = df[(df['operation'] == op) & (df['assignment'] == 'default')]
    complexity = op_data['complexity'].iloc[0]

    # Plot by scale
    for scale in ['Small', 'Medium', 'Large', 'VeryLarge', 'Huge']:
        scale_data = op_data[op_data['scale'] == scale]
        if len(scale_data) > 0:
            threads = scale_data['threads'].values
            speedups = scale_data['speedup_vs_1t'].values

            if scale in ['Large', 'VeryLarge', 'Huge']:
                ax.plot(threads, speedups, 'o-', linewidth=2, markersize=8, label=scale)
            else:
                ax.plot(threads, speedups, 'o-', linewidth=1, markersize=4, label=scale, alpha=0.4)

    # Add ideal scaling line
    ax.plot([1, 2, 4, 8], [1, 2, 4, 8], 'k--', alpha=0.3, linewidth=2, label='Ideal')

    ax.set_xlabel('Threads')
    ax.set_ylabel('Speedup vs 1t')
    ax.set_title(f'{op} (complexity {complexity:.2f})', fontsize=11, fontweight='bold')
    ax.legend(fontsize=8, loc='upper left')
    ax.grid(True, alpha=0.3)
    ax.set_xticks([1, 2, 4, 8])

plt.tight_layout()
plt.savefig(output_dir / 'thread_scaling_comparison.png', dpi=300, bbox_inches='tight')
print(f"Saved: {output_dir / 'thread_scaling_comparison.png'}")
plt.close()

print("\nAll visualizations generated successfully!")

# ============================================================================
# 3. SUMMARY STATISTICS
# ============================================================================

print("\n" + "=" * 70)
print("SUMMARY STATISTICS")
print("=" * 70)
print()

with open(output_dir / "summary_statistics.txt", "w") as f:
    # Overall statistics
    f.write("OVERALL PARALLEL PERFORMANCE SUMMARY\n")
    f.write("=" * 70 + "\n\n")

    # Best speedup per operation
    f.write("Best Speedup Achieved (any configuration):\n")
    f.write("-" * 70 + "\n")
    for op in sorted(df['operation'].unique()):
        op_data = df[df['operation'] == op]
        complexity = op_data['complexity'].iloc[0]
        best_row = op_data.loc[op_data['speedup_vs_1t'].idxmax()]

        f.write(f"{op:20s} (complexity {complexity:.2f}): "
                f"{best_row['speedup_vs_1t']:.2f}× "
                f"({best_row['threads']}t/{best_row['assignment']}, "
                f"{best_row['scale']} scale)\n")

    f.write("\n")

    # Average speedup at 8 threads, Huge scale
    f.write("Speedup at 8 threads, Huge scale (10M sequences):\n")
    f.write("-" * 70 + "\n")
    huge_8t = df[(df['scale'] == 'Huge') & (df['threads'] == 8)]
    for op in sorted(df['operation'].unique()):
        op_data = huge_8t[huge_8t['operation'] == op]
        if len(op_data) > 0:
            avg_speedup = op_data['speedup_vs_1t'].mean()
            best_assignment = op_data.loc[op_data['speedup_vs_1t'].idxmax(), 'assignment']
            best_speedup = op_data['speedup_vs_1t'].max()

            f.write(f"{op:20s}: Avg={avg_speedup:.2f}×, "
                    f"Best={best_speedup:.2f}× ({best_assignment})\n")

    f.write("\n")

    # P-core vs E-core comparison
    f.write("P-cores vs E-cores (8 threads, Huge scale, relative performance):\n")
    f.write("-" * 70 + "\n")
    for op in sorted(df['operation'].unique()):
        op_data = huge_8t[huge_8t['operation'] == op]
        p_speedup = op_data[op_data['assignment'] == 'p_cores']['speedup_vs_1t'].mean()
        e_speedup = op_data[op_data['assignment'] == 'e_cores']['speedup_vs_1t'].mean()

        if pd.notna(p_speedup) and pd.notna(e_speedup) and e_speedup > 0:
            ratio = p_speedup / e_speedup
            if ratio > 1.0:
                winner = "P-cores"
                margin = ((ratio - 1) * 100)
            else:
                winner = "E-cores"
                margin = ((1/ratio - 1) * 100)

            f.write(f"{op:20s}: P={p_speedup:.2f}×, E={e_speedup:.2f}×, "
                    f"Winner: {winner} (+{margin:.1f}%)\n")

print(f"Summary statistics saved to: {output_dir / 'summary_statistics.txt'}")

# ============================================================================
# 4. DECISION RULES
# ============================================================================

print("\n" + "=" * 70)
print("DERIVING DECISION RULES")
print("=" * 70)
print()

with open(output_dir / "decision_rules.txt", "w") as f:
    f.write("PARALLEL OPTIMIZATION DECISION RULES\n")
    f.write("=" * 70 + "\n\n")

    f.write("Based on 600 experiments across 10 operations × 6 scales × 10 configs\n\n")

    # Rule 1: Minimum batch size for parallel benefit
    f.write("RULE 1: Minimum Batch Size for Parallel Benefit\n")
    f.write("-" * 70 + "\n")

    for op in sorted(df['operation'].unique()):
        op_data = df[(df['operation'] == op) & (df['threads'] == 2) & (df['assignment'] == 'default')]

        # Find first scale where speedup > 1.1
        threshold_scale = None
        for scale in scale_order:
            scale_data = op_data[op_data['scale'] == scale]
            if len(scale_data) > 0 and scale_data['speedup_vs_1t'].iloc[0] > 1.1:
                threshold_scale = scale
                threshold_seqs = scale_data['num_sequences'].iloc[0]
                break

        if threshold_scale:
            f.write(f"{op:20s}: >={threshold_seqs:>8d} sequences ({threshold_scale})\n")
        else:
            f.write(f"{op:20s}: No clear benefit observed\n")

    f.write("\n")

    # Rule 2: Optimal thread count by operation and scale
    f.write("RULE 2: Optimal Thread Count (Default Assignment)\n")
    f.write("-" * 70 + "\n")

    for scale in ['Small', 'Medium', 'Large', 'VeryLarge', 'Huge']:
        f.write(f"\n{scale} scale:\n")
        scale_data = df[(df['scale'] == scale) & (df['assignment'] == 'default')]

        for op in sorted(df['operation'].unique()):
            op_data = scale_data[scale_data['operation'] == op]
            if len(op_data) > 0:
                best_row = op_data.loc[op_data['speedup_vs_1t'].idxmax()]
                f.write(f"  {op:20s}: {best_row['threads']}t "
                        f"({best_row['speedup_vs_1t']:.2f}× speedup)\n")

    f.write("\n")

    # Rule 3: When to use P-cores vs E-cores
    f.write("RULE 3: P-cores vs E-cores (8 threads, Huge scale)\n")
    f.write("-" * 70 + "\n")

    huge_8t = df[(df['scale'] == 'Huge') & (df['threads'] == 8)]
    for op in sorted(df['operation'].unique()):
        op_data = huge_8t[huge_8t['operation'] == op]

        default_speedup = op_data[op_data['assignment'] == 'default']['speedup_vs_1t'].mean()
        p_speedup = op_data[op_data['assignment'] == 'p_cores']['speedup_vs_1t'].mean()
        e_speedup = op_data[op_data['assignment'] == 'e_cores']['speedup_vs_1t'].mean()

        best_assignment = 'default'
        best_speedup = default_speedup

        if pd.notna(p_speedup) and p_speedup > best_speedup:
            best_assignment = 'p_cores'
            best_speedup = p_speedup
        if pd.notna(e_speedup) and e_speedup > best_speedup:
            best_assignment = 'e_cores'
            best_speedup = e_speedup

        improvement = ((best_speedup / default_speedup - 1) * 100) if default_speedup > 0 else 0

        if best_assignment == 'default':
            f.write(f"{op:20s}: Use default (no benefit from explicit assignment)\n")
        else:
            f.write(f"{op:20s}: Use {best_assignment} (+{improvement:.1f}% vs default)\n")

print(f"Decision rules saved to: {output_dir / 'decision_rules.txt'}")

print("\n" + "=" * 70)
print("ANALYSIS COMPLETE")
print("=" * 70)
print(f"\nAll outputs saved to: {output_dir}")
print("\nGenerated files:")
print(f"  - speedup_matrices.txt")
print(f"  - summary_statistics.txt")
print(f"  - decision_rules.txt")
print(f"  - speedup_curves_all_ops.png")
print(f"  - core_assignment_comparison.png")
print(f"  - efficiency_heatmap.png")
print(f"  - complexity_vs_speedup.png")
print(f"  - thread_scaling_comparison.png")
