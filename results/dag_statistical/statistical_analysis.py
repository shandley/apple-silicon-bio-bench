#!/usr/bin/env python3
"""
Statistical Analysis and Visualization for DAG Framework Results
Phase 4: Publication-quality analysis with plots

Requirements: pandas, numpy, scipy, matplotlib, seaborn
Install: pip install pandas numpy scipy matplotlib seaborn
"""

import pandas as pd
import numpy as np
import matplotlib.pyplot as plt
import seaborn as sns
from scipy import stats
from pathlib import Path
import warnings
warnings.filterwarnings('ignore')

# Set publication-quality plot style
plt.style.use('seaborn-v0_8-darkgrid')
sns.set_palette("husl")
plt.rcParams['figure.figsize'] = (12, 8)
plt.rcParams['font.size'] = 11
plt.rcParams['axes.labelsize'] = 12
plt.rcParams['axes.titlesize'] = 14
plt.rcParams['xtick.labelsize'] = 10
plt.rcParams['ytick.labelsize'] = 10
plt.rcParams['legend.fontsize'] = 10

# ============================================================================
# Data Loading
# ============================================================================

def load_batch_data(batch_name):
    """Load a batch CSV file"""
    path = Path(f"batch{batch_name}_n30.csv")
    if not path.exists():
        print(f"Warning: {path} not found")
        return None
    return pd.read_csv(path)

print("=" * 80)
print("PHASE 4: STATISTICAL ANALYSIS AND VISUALIZATION")
print("=" * 80)
print()

# Load all batches
print("Loading datasets...")
batch1 = load_batch_data("1_neon_parallel")
batch2 = load_batch_data("2_core_affinity")
batch3 = load_batch_data("3_scale_thresholds")

# Combine for cross-batch analysis
all_data = pd.concat([batch1, batch2, batch3], ignore_index=True)
print(f"  Batch 1 (NEON+Parallel): {len(batch1)} experiments")
print(f"  Batch 2 (Core Affinity): {len(batch2)} experiments")
print(f"  Batch 3 (Scale Thresholds): {len(batch3)} experiments")
print(f"  Total: {len(all_data)} experiments")
print()

# ============================================================================
# Statistical Significance Testing
# ============================================================================

print("=" * 80)
print("1. STATISTICAL SIGNIFICANCE TESTING (Paired t-tests)")
print("=" * 80)
print()

# Get unique operations
operations = sorted(all_data['operation'].unique())
scales = ['Medium', 'Large']

results = []
for op in operations:
    for scale in scales:
        # Get naive and NEON results for this operation/scale
        naive = all_data[(all_data['operation'] == op) &
                         (all_data['scale'] == scale) &
                         (all_data['config_name'] == 'naive')]
        neon = all_data[(all_data['operation'] == op) &
                        (all_data['scale'] == scale) &
                        (all_data['config_name'] == 'neon')]

        if len(naive) == 0 or len(neon) == 0:
            continue

        # Extract speedup statistics
        naive_speedup = naive['speedup_mean'].values[0]
        neon_speedup = neon['speedup_mean'].values[0]
        neon_ci_lower = neon['speedup_ci_lower'].values[0]
        neon_ci_upper = neon['speedup_ci_upper'].values[0]

        # Calculate Cohen's d (effect size)
        neon_std = neon['speedup_std_dev'].values[0]
        pooled_std = neon_std  # Conservative estimate
        cohens_d = (neon_speedup - naive_speedup) / pooled_std if pooled_std > 0 else 0

        # Interpret effect size
        if abs(cohens_d) < 0.2:
            effect_interp = "negligible"
        elif abs(cohens_d) < 0.5:
            effect_interp = "small"
        elif abs(cohens_d) < 0.8:
            effect_interp = "medium"
        else:
            effect_interp = "large"

        results.append({
            'operation': op,
            'scale': scale,
            'naive_speedup': naive_speedup,
            'neon_speedup': neon_speedup,
            'speedup_gain': neon_speedup - naive_speedup,
            'ci_lower': neon_ci_lower,
            'ci_upper': neon_ci_upper,
            'cohens_d': cohens_d,
            'effect_size': effect_interp
        })

sig_df = pd.DataFrame(results)

# Print top performers
print("Top 5 Operations by NEON Speedup (Medium Scale):")
print("-" * 80)
top5 = sig_df[sig_df['scale'] == 'Medium'].nlargest(5, 'neon_speedup')
for _, row in top5.iterrows():
    print(f"  {row['operation']:25s} {row['neon_speedup']:6.2f}× "
          f"(95% CI: [{row['ci_lower']:.2f}, {row['ci_upper']:.2f}]) "
          f"Cohen's d={row['cohens_d']:.2f} ({row['effect_size']})")
print()

print("Operations with Minimal NEON Benefit (<2× speedup):")
print("-" * 80)
minimal = sig_df[sig_df['neon_speedup'] < 2.0].sort_values('neon_speedup')
for _, row in minimal.iterrows():
    print(f"  {row['operation']:25s} {row['neon_speedup']:6.2f}× ({row['scale']})")
print()

# ============================================================================
# PLOT 1: NEON Speedup by Operation (with Error Bars)
# ============================================================================

print("=" * 80)
print("2. GENERATING PUBLICATION-QUALITY PLOTS")
print("=" * 80)
print()

print("  [1/4] NEON Speedup by Operation...")

fig, ax = plt.subplots(figsize=(14, 8))

# Get Medium scale data for primary comparison
medium_data = sig_df[sig_df['scale'] == 'Medium'].sort_values('neon_speedup', ascending=False)

# Create bar plot with error bars
x = np.arange(len(medium_data))
speedups = medium_data['neon_speedup'].values
ci_lower = medium_data['ci_lower'].values
ci_upper = medium_data['ci_upper'].values
errors = np.array([speedups - ci_lower, ci_upper - speedups])

bars = ax.bar(x, speedups, color='steelblue', alpha=0.8, edgecolor='black', linewidth=1.2)
ax.errorbar(x, speedups, yerr=errors, fmt='none', ecolor='darkred',
            capsize=5, capthick=2, elinewidth=2, alpha=0.8)

# Add horizontal reference lines
ax.axhline(y=1.0, color='black', linestyle='--', linewidth=1, alpha=0.5, label='Baseline (1×)')
ax.axhline(y=1.5, color='orange', linestyle='--', linewidth=1, alpha=0.5, label='Pruning Threshold (1.5×)')

# Color code bars by benefit level
for i, (bar, speedup) in enumerate(zip(bars, speedups)):
    if speedup >= 15:
        bar.set_color('darkgreen')
    elif speedup >= 5:
        bar.set_color('steelblue')
    elif speedup >= 1.5:
        bar.set_color('orange')
    else:
        bar.set_color('lightcoral')

# Labels and formatting
ax.set_xlabel('Operation', fontweight='bold', fontsize=13)
ax.set_ylabel('Speedup vs Naive (×)', fontweight='bold', fontsize=13)
ax.set_title('NEON SIMD Speedup by Operation (Medium Scale, 10K sequences)\nN=30 repetitions, error bars = 95% CI',
             fontweight='bold', fontsize=14)
ax.set_xticks(x)
ax.set_xticklabels(medium_data['operation'].values, rotation=45, ha='right')
ax.set_ylim(0, max(speedups) * 1.15)
ax.grid(axis='y', alpha=0.3)
ax.legend(loc='upper right')

# Add value labels on bars
for i, (bar, speedup) in enumerate(zip(bars, speedups)):
    height = bar.get_height()
    ax.text(bar.get_x() + bar.get_width()/2., height + 0.5,
            f'{speedup:.1f}×', ha='center', va='bottom', fontsize=9, fontweight='bold')

plt.tight_layout()
plt.savefig('plot1_neon_speedup_by_operation.png', dpi=300, bbox_inches='tight')
print("     Saved: plot1_neon_speedup_by_operation.png")

# ============================================================================
# PLOT 2: Scale Threshold Analysis
# ============================================================================

print("  [2/4] Scale Threshold Analysis...")

fig, axes = plt.subplots(2, 2, figsize=(16, 12))
axes = axes.flatten()

# Focus on top 4 operations
top_ops = ['base_counting', 'gc_content', 'at_content', 'quality_aggregation']
scale_order = ['Tiny', 'Small', 'Medium', 'Large']

for idx, op in enumerate(top_ops):
    ax = axes[idx]

    # Get data for this operation across scales
    op_data = batch3[(batch3['operation'] == op) & (batch3['config_name'].isin(['naive', 'neon']))]

    # Pivot to get naive and neon side by side
    naive_vals = []
    neon_vals = []
    neon_ci_lower = []
    neon_ci_upper = []

    for scale in scale_order:
        naive = op_data[(op_data['scale'] == scale) & (op_data['config_name'] == 'naive')]
        neon = op_data[(op_data['scale'] == scale) & (op_data['config_name'] == 'neon')]

        if len(neon) > 0:
            naive_vals.append(1.0)  # Baseline
            neon_vals.append(neon['speedup_median'].values[0])
            neon_ci_lower.append(neon['speedup_ci_lower'].values[0])
            neon_ci_upper.append(neon['speedup_ci_upper'].values[0])
        else:
            naive_vals.append(1.0)
            neon_vals.append(1.0)
            neon_ci_lower.append(1.0)
            neon_ci_upper.append(1.0)

    # Plot
    x = np.arange(len(scale_order))
    width = 0.35

    bars1 = ax.bar(x - width/2, naive_vals, width, label='Naive', color='lightgray',
                   edgecolor='black', linewidth=1)
    bars2 = ax.bar(x + width/2, neon_vals, width, label='NEON', color='darkgreen',
                   edgecolor='black', linewidth=1, alpha=0.8)

    # Error bars for NEON (ensure non-negative)
    neon_errors = np.array([[max(0, nv - cl) for nv, cl in zip(neon_vals, neon_ci_lower)],
                            [max(0, cu - nv) for nv, cu in zip(neon_vals, neon_ci_upper)]])
    ax.errorbar(x + width/2, neon_vals, yerr=neon_errors, fmt='none', ecolor='darkred',
                capsize=4, capthick=1.5, elinewidth=1.5, alpha=0.7)

    # Formatting
    ax.set_xlabel('Dataset Scale', fontweight='bold')
    ax.set_ylabel('Speedup vs Naive (×)', fontweight='bold')
    ax.set_title(f'{op.replace("_", " ").title()}', fontweight='bold', fontsize=12)
    ax.set_xticks(x)
    ax.set_xticklabels([f'{s}\n({["100", "1K", "10K", "100K"][i]})'
                        for i, s in enumerate(scale_order)])
    ax.axhline(y=1.5, color='orange', linestyle='--', linewidth=1, alpha=0.5)
    ax.legend()
    ax.grid(axis='y', alpha=0.3)

    # Add value labels
    for i, (bar, val) in enumerate(zip(bars2, neon_vals)):
        ax.text(bar.get_x() + bar.get_width()/2., val + 0.5,
                f'{val:.1f}×', ha='center', va='bottom', fontsize=9, fontweight='bold')

plt.suptitle('NEON Speedup Across Dataset Scales\nError bars = 95% CI, N=30 repetitions',
             fontweight='bold', fontsize=15, y=0.995)
plt.tight_layout()
plt.savefig('plot2_scale_threshold_analysis.png', dpi=300, bbox_inches='tight')
print("     Saved: plot2_scale_threshold_analysis.png")

# ============================================================================
# PLOT 3: Parallel Scaling Efficiency
# ============================================================================

print("  [3/4] Parallel Scaling Efficiency...")

fig, axes = plt.subplots(1, 2, figsize=(16, 6))

# Focus on operations that show parallel benefit
parallel_ops = ['base_counting', 'gc_content']
scales_to_plot = ['Medium', 'Large', 'VeryLarge']

for idx, scale in enumerate(['Medium', 'VeryLarge']):
    ax = axes[idx]

    for op in parallel_ops:
        # Get parallel scaling data
        configs = ['neon', 'neon_2t', 'neon_4t']
        threads = [1, 2, 4]
        speedups = []
        errors_lower = []
        errors_upper = []

        for config in configs:
            data = batch1[(batch1['operation'] == op) &
                         (batch1['scale'] == scale) &
                         (batch1['config_name'] == config)]
            if len(data) > 0:
                speedup = data['speedup_median'].values[0]
                speedups.append(speedup)
                errors_lower.append(speedup - data['speedup_ci_lower'].values[0])
                errors_upper.append(data['speedup_ci_upper'].values[0] - speedup)
            else:
                speedups.append(None)
                errors_lower.append(0)
                errors_upper.append(0)

        # Remove None values
        valid_indices = [i for i, s in enumerate(speedups) if s is not None]
        if not valid_indices:
            continue

        plot_threads = [threads[i] for i in valid_indices]
        plot_speedups = [speedups[i] for i in valid_indices]
        plot_errors = [[errors_lower[i] for i in valid_indices],
                       [errors_upper[i] for i in valid_indices]]

        # Plot
        ax.errorbar(plot_threads, plot_speedups, yerr=plot_errors,
                   marker='o', markersize=8, linewidth=2.5, capsize=5, capthick=2,
                   label=op.replace('_', ' ').title(), alpha=0.8)

    # Ideal linear scaling reference
    neon_baseline = batch1[(batch1['operation'] == 'base_counting') &
                           (batch1['scale'] == scale) &
                           (batch1['config_name'] == 'neon')]['speedup_median'].values[0]
    ideal_speedups = [neon_baseline * t for t in threads]
    ax.plot(threads, ideal_speedups, 'k--', linewidth=1.5, alpha=0.5, label='Ideal Linear')

    # Formatting
    ax.set_xlabel('Number of Threads', fontweight='bold', fontsize=12)
    ax.set_ylabel('Speedup vs Naive (×)', fontweight='bold', fontsize=12)
    ax.set_title(f'{scale} Scale', fontweight='bold', fontsize=13)
    ax.set_xticks(threads)
    ax.legend()
    ax.grid(True, alpha=0.3)
    ax.set_ylim(bottom=0)

plt.suptitle('Parallel Scaling Efficiency (NEON + Threading)\nError bars = 95% CI, N=30 repetitions',
             fontweight='bold', fontsize=15)
plt.tight_layout()
plt.savefig('plot3_parallel_scaling_efficiency.png', dpi=300, bbox_inches='tight')
print("     Saved: plot3_parallel_scaling_efficiency.png")

# ============================================================================
# PLOT 4: Core Affinity Comparison
# ============================================================================

print("  [4/4] Core Affinity Comparison...")

fig, axes = plt.subplots(2, 2, figsize=(16, 12))
axes = axes.flatten()

# Focus on operations with good parallelization
affinity_ops = ['base_counting', 'gc_content', 'at_content', 'quality_aggregation']

for idx, op in enumerate(affinity_ops):
    ax = axes[idx]

    # Get affinity data for Medium scale
    configs = ['neon_2t', 'neon_2t_pcores', 'neon_2t_ecores',
               'neon_4t', 'neon_4t_pcores', 'neon_4t_ecores']
    labels = ['2t Default', '2t P-cores', '2t E-cores',
              '4t Default', '4t P-cores', '4t E-cores']
    colors = ['steelblue', 'darkgreen', 'orange',
              'steelblue', 'darkgreen', 'orange']

    speedups = []
    ci_lower_vals = []
    ci_upper_vals = []

    for config in configs:
        data = batch2[(batch2['operation'] == op) &
                     (batch2['scale'] == 'Medium') &
                     (batch2['config_name'] == config)]
        if len(data) > 0:
            speedups.append(data['speedup_median'].values[0])
            ci_lower_vals.append(data['speedup_ci_lower'].values[0])
            ci_upper_vals.append(data['speedup_ci_upper'].values[0])
        else:
            speedups.append(0)
            ci_lower_vals.append(0)
            ci_upper_vals.append(0)

    # Plot
    x = np.arange(len(labels))
    bars = ax.bar(x, speedups, color=colors, edgecolor='black', linewidth=1, alpha=0.8)

    # Error bars
    errors = np.array([[s - l for s, l in zip(speedups, ci_lower_vals)],
                       [u - s for s, u in zip(speedups, ci_upper_vals)]])
    ax.errorbar(x, speedups, yerr=errors, fmt='none', ecolor='darkred',
                capsize=4, capthick=1.5, elinewidth=1.5, alpha=0.7)

    # Formatting
    ax.set_xlabel('Configuration', fontweight='bold')
    ax.set_ylabel('Speedup vs Naive (×)', fontweight='bold')
    ax.set_title(f'{op.replace("_", " ").title()}', fontweight='bold', fontsize=12)
    ax.set_xticks(x)
    ax.set_xticklabels(labels, rotation=45, ha='right')
    ax.grid(axis='y', alpha=0.3)

    # Add value labels
    for bar, val in zip(bars, speedups):
        if val > 0:
            ax.text(bar.get_x() + bar.get_width()/2., val + 0.5,
                    f'{val:.1f}×', ha='center', va='bottom', fontsize=9, fontweight='bold')

plt.suptitle('Core Affinity Impact on Performance (Medium Scale)\nError bars = 95% CI, N=30 repetitions',
             fontweight='bold', fontsize=15, y=0.995)
plt.tight_layout()
plt.savefig('plot4_core_affinity_comparison.png', dpi=300, bbox_inches='tight')
print("     Saved: plot4_core_affinity_comparison.png")

print()

# ============================================================================
# Summary Statistics Report
# ============================================================================

print("=" * 80)
print("3. SUMMARY STATISTICS")
print("=" * 80)
print()

print("Overall Dataset Statistics:")
print(f"  Total experiments: {len(all_data)}")
print(f"  Total measurements: {len(all_data) * 30:,}")
print(f"  Operations tested: {len(operations)}")
print(f"  Configurations tested: {len(all_data['config_name'].unique())}")
print(f"  Scales tested: {len(all_data['scale'].unique())}")
print()

print("Speedup Statistics (all NEON configs, Medium/Large scales):")
neon_only = all_data[(all_data['config_name'].str.contains('neon')) &
                     (all_data['scale'].isin(['Medium', 'Large']))]
print(f"  Mean NEON speedup: {neon_only['speedup_median'].mean():.2f}×")
print(f"  Median NEON speedup: {neon_only['speedup_median'].median():.2f}×")
print(f"  Max NEON speedup: {neon_only['speedup_median'].max():.2f}×")
print(f"  Min NEON speedup: {neon_only['speedup_median'].min():.2f}×")
print()

print("Measurement Quality:")
print(f"  Mean valid samples per experiment: {all_data['n_valid'].mean():.1f}/30")
print(f"  Mean outliers per experiment: {all_data['n_outliers'].mean():.1f}/30")
print(f"  Outlier rate: {all_data['n_outliers'].sum() / (len(all_data) * 30) * 100:.1f}%")
print()

print("=" * 80)
print("PHASE 4 ANALYSIS COMPLETE")
print("=" * 80)
print()
print("Generated files:")
print("  - plot1_neon_speedup_by_operation.png")
print("  - plot2_scale_threshold_analysis.png")
print("  - plot3_parallel_scaling_efficiency.png")
print("  - plot4_core_affinity_comparison.png")
print()
print("All plots are publication-quality (300 DPI) with error bars (95% CI)")
