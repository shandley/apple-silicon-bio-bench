#!/usr/bin/env python3
"""
Generate Publication-Quality Validation Plots (Artifact 3)

Creates 5 plots for manuscript submission:
1. NEON Speedup by Operation
2. Streaming Memory Footprint
3. I/O Optimization Stack
4. Block Size Impact (Streaming Overhead)
5. mmap Threshold Effect

Output: 300 DPI PNG + vector PDF (publication-ready)
"""

import pandas as pd
import matplotlib.pyplot as plt
import matplotlib.patches as mpatches
import numpy as np
from pathlib import Path
import seaborn as sns

# Set publication-quality style
plt.rcParams.update({
    'font.size': 11,
    'font.family': 'sans-serif',
    'font.sans-serif': ['Arial', 'Helvetica'],
    'axes.labelsize': 12,
    'axes.titlesize': 14,
    'xtick.labelsize': 10,
    'ytick.labelsize': 10,
    'legend.fontsize': 10,
    'figure.titlesize': 16,
    'figure.dpi': 300,
    'savefig.dpi': 300,
    'savefig.bbox': 'tight',
    'savefig.pad_inches': 0.1,
})

# Color palette
COLORS = {
    'naive': '#E74C3C',      # Red
    'neon': '#3498DB',       # Blue
    'parallel': '#2ECC71',   # Green
    'combined': '#9B59B6',   # Purple
    'batch': '#E67E22',      # Orange
    'streaming': '#1ABC9C',  # Teal
    'sequential': '#95A5A6', # Gray
    'mmap': '#F39C12',       # Yellow-orange
}

def create_output_dir():
    """Create output directory for plots"""
    output_dir = Path('results/publication_plots')
    output_dir.mkdir(parents=True, exist_ok=True)
    return output_dir

def save_plot(fig, name, output_dir):
    """Save plot as both PNG and PDF"""
    png_path = output_dir / f"{name}.png"
    pdf_path = output_dir / f"{name}.pdf"

    fig.savefig(png_path, format='png', dpi=300, bbox_inches='tight')
    fig.savefig(pdf_path, format='pdf', bbox_inches='tight')

    print(f"✓ Saved: {name}.png and {name}.pdf")
    return png_path, pdf_path

def plot1_neon_speedup_by_operation(output_dir):
    """
    Plot 1: NEON Speedup by Operation

    Shows speedup for different operations to demonstrate
    the 16-25× range and operation-specific effects.
    """
    print("\n=== Plot 1: NEON Speedup by Operation ===")

    # Load DAG batch 1 data (NEON validation)
    df = pd.read_csv('results/dag_statistical/batch1_neon_parallel_n30.csv')

    # Filter for Medium scale, NEON config only
    df_neon = df[(df['scale'] == 'Medium') & (df['config_name'] == 'neon')]
    df_naive = df[(df['scale'] == 'Medium') & (df['config_name'] == 'naive')]

    # Merge to calculate speedup
    df_merged = df_neon.merge(
        df_naive[['operation', 'throughput_mean']],
        on='operation',
        suffixes=('_neon', '_naive')
    )
    df_merged['speedup'] = df_merged['throughput_mean_neon'] / df_merged['throughput_mean_naive']

    # Sort by speedup
    df_merged = df_merged.sort_values('speedup', ascending=True)

    # Create figure
    fig, ax = plt.subplots(figsize=(10, 6))

    # Bar plot with color gradient based on speedup
    colors = ['#2ECC71' if s >= 10 else '#3498DB' if s >= 5 else '#95A5A6'
              for s in df_merged['speedup']]

    bars = ax.barh(df_merged['operation'], df_merged['speedup'], color=colors, edgecolor='black', linewidth=0.5)

    # Add speedup labels on bars
    for i, (idx, row) in enumerate(df_merged.iterrows()):
        ax.text(row['speedup'] + 0.5, i, f"{row['speedup']:.1f}×",
                va='center', ha='left', fontsize=9)

    # Reference line at 1× (no speedup)
    ax.axvline(x=1, color='red', linestyle='--', linewidth=1, alpha=0.5, label='No speedup')

    # Formatting
    ax.set_xlabel('Speedup vs Naive (NEON SIMD)', fontweight='bold')
    ax.set_ylabel('Operation', fontweight='bold')
    ax.set_title('NEON SIMD Speedup by Operation\n(Medium scale: 10K sequences, N=30)',
                 fontweight='bold', pad=15)
    ax.set_xlim(0, max(df_merged['speedup']) * 1.15)
    ax.grid(axis='x', alpha=0.3, linestyle=':', linewidth=0.5)

    # Legend
    high_patch = mpatches.Patch(color='#2ECC71', label='High benefit (≥10×)')
    med_patch = mpatches.Patch(color='#3498DB', label='Medium benefit (5-10×)')
    low_patch = mpatches.Patch(color='#95A5A6', label='Low benefit (<5×)')
    ax.legend(handles=[high_patch, med_patch, low_patch], loc='lower right')

    plt.tight_layout()
    save_plot(fig, 'plot1_neon_speedup_by_operation', output_dir)
    plt.close()

def plot2_streaming_memory_footprint(output_dir):
    """
    Plot 2: Streaming Memory Footprint

    Shows 99.5% memory reduction from batch to streaming approach.
    """
    print("\n=== Plot 2: Streaming Memory Footprint ===")

    # Load streaming memory data
    df = pd.read_csv('results/streaming/streaming_memory_v2_n30.csv')

    # Aggregate by scale and pattern
    df_agg = df.groupby(['scale', 'pattern']).agg({
        'peak_mb': 'mean',
        'num_sequences': 'first'
    }).reset_index()

    # Pivot for easier plotting
    df_pivot = df_agg.pivot(index='scale', columns='pattern', values='peak_mb')

    # Define scale order
    scale_order = ['Medium', 'Large', 'VeryLarge']
    df_pivot = df_pivot.reindex(scale_order)

    # Create figure
    fig, ax = plt.subplots(figsize=(10, 6))

    # Bar plot
    x = np.arange(len(scale_order))
    width = 0.35

    bars1 = ax.bar(x - width/2, df_pivot['batch'], width,
                   label='Batch (load-all)', color=COLORS['batch'],
                   edgecolor='black', linewidth=0.5)
    bars2 = ax.bar(x + width/2, df_pivot['streaming'], width,
                   label='Streaming', color=COLORS['streaming'],
                   edgecolor='black', linewidth=0.5)

    # Add value labels on bars
    for bars in [bars1, bars2]:
        for bar in bars:
            height = bar.get_height()
            if height > 10:
                label = f'{height:.0f} MB'
            else:
                label = f'{height:.1f} MB'
            ax.text(bar.get_x() + bar.get_width()/2., height,
                    label, ha='center', va='bottom', fontsize=9)

    # Add reduction percentage annotations
    for i, scale in enumerate(scale_order):
        batch_mem = df_pivot.loc[scale, 'batch']
        stream_mem = df_pivot.loc[scale, 'streaming']
        reduction = (1 - stream_mem / batch_mem) * 100

        # Arrow showing reduction
        ax.annotate('', xy=(i + width/2, stream_mem + 10),
                    xytext=(i - width/2, batch_mem - 10),
                    arrowprops=dict(arrowstyle='->', lw=1.5, color='red'))

        # Reduction percentage
        mid_y = (batch_mem + stream_mem) / 2
        ax.text(i, mid_y, f'-{reduction:.1f}%',
                ha='center', va='center', fontsize=10,
                fontweight='bold', color='red',
                bbox=dict(boxstyle='round,pad=0.3', facecolor='white', edgecolor='red'))

    # Formatting
    ax.set_xlabel('Dataset Scale', fontweight='bold')
    ax.set_ylabel('Peak Memory Usage (MB)', fontweight='bold')
    ax.set_title('Streaming Memory Footprint: 99.5% Reduction\n(base_counting + gc_content, N=30)',
                 fontweight='bold', pad=15)
    ax.set_xticks(x)
    ax.set_xticklabels(['Medium\n(10K seqs)', 'Large\n(100K seqs)', 'VeryLarge\n(1M seqs)'])
    ax.legend(loc='upper left')
    ax.grid(axis='y', alpha=0.3, linestyle=':', linewidth=0.5)

    plt.tight_layout()
    save_plot(fig, 'plot2_streaming_memory_footprint', output_dir)
    plt.close()

def plot3_io_optimization_stack(output_dir):
    """
    Plot 3: I/O Optimization Stack

    Shows layered benefits: parallel bgzip (6.5×) + mmap (2.5×) = 16.3×
    """
    print("\n=== Plot 3: I/O Optimization Stack ===")

    # Data from lab notebook entries 029-032
    optimizations = ['Sequential\nBaseline', 'Parallel\nbgzip', 'Parallel +\nmmap']
    throughput_small = [646, 3541, 3541]  # Small files: mmap doesn't help
    throughput_large = [718, 4669, 15694]  # Large files: mmap adds 2.5×

    speedup_small = [1.0, 5.48, 5.48]
    speedup_large = [1.0, 6.50, 16.3]

    # Create figure with two subplots
    fig, (ax1, ax2) = plt.subplots(1, 2, figsize=(14, 6))

    # --- Left subplot: Small files ---
    x = np.arange(len(optimizations))
    width = 0.6

    bars1 = ax1.bar(x, throughput_small, width,
                    color=['#95A5A6', '#3498DB', '#3498DB'],
                    edgecolor='black', linewidth=0.5)

    # Add speedup labels
    for i, (bar, speedup) in enumerate(zip(bars1, speedup_small)):
        height = bar.get_height()
        ax1.text(bar.get_x() + bar.get_width()/2., height + 200,
                f'{speedup:.1f}×', ha='center', va='bottom',
                fontsize=11, fontweight='bold')
        ax1.text(bar.get_x() + bar.get_width()/2., height/2,
                f'{throughput_small[i]:,}\nMB/s', ha='center', va='center',
                fontsize=9, color='white', fontweight='bold')

    ax1.set_ylabel('Throughput (MB/s)', fontweight='bold')
    ax1.set_title('Small Files (<50 MB)\n0.58 MB, 51 bgzip blocks',
                  fontweight='bold', pad=10)
    ax1.set_xticks(x)
    ax1.set_xticklabels(optimizations)
    ax1.set_ylim(0, 4500)
    ax1.grid(axis='y', alpha=0.3, linestyle=':', linewidth=0.5)

    # --- Right subplot: Large files ---
    bars2 = ax2.bar(x, throughput_large, width,
                    color=['#95A5A6', '#3498DB', '#2ECC71'],
                    edgecolor='black', linewidth=0.5)

    # Add speedup labels
    for i, (bar, speedup) in enumerate(zip(bars2, speedup_large)):
        height = bar.get_height()
        ax2.text(bar.get_x() + bar.get_width()/2., height + 1000,
                f'{speedup:.1f}×', ha='center', va='bottom',
                fontsize=11, fontweight='bold')
        ax2.text(bar.get_x() + bar.get_width()/2., height/2,
                f'{throughput_large[i]:,}\nMB/s', ha='center', va='center',
                fontsize=9, color='white', fontweight='bold')

    ax2.set_ylabel('Throughput (MB/s)', fontweight='bold')
    ax2.set_title('Large Files (≥50 MB)\n5.82 MB, 485 bgzip blocks',
                  fontweight='bold', pad=10)
    ax2.set_xticks(x)
    ax2.set_xticklabels(optimizations)
    ax2.set_ylim(0, 18000)
    ax2.grid(axis='y', alpha=0.3, linestyle=':', linewidth=0.5)

    # Overall title
    fig.suptitle('I/O Optimization Stack: Layered Benefits\n(CPU parallel bgzip + smart mmap)',
                 fontsize=16, fontweight='bold', y=1.02)

    plt.tight_layout()
    save_plot(fig, 'plot3_io_optimization_stack', output_dir)
    plt.close()

def plot4_block_size_impact(output_dir):
    """
    Plot 4: Block Size Impact (Streaming Overhead)

    Shows 82-86% overhead with record-by-record, solved by block-based processing.
    """
    print("\n=== Plot 4: Block Size Impact (Streaming Overhead) ===")

    # Load streaming overhead data
    df = pd.read_csv('results/streaming/streaming_overhead_n30.csv')

    # Filter for NEON only (shows the overhead most dramatically)
    df_neon = df[df['config'] == 'neon']

    # Calculate overhead percentage
    df_pivot = df_neon.pivot_table(
        index=['operation', 'scale'],
        columns='pattern',
        values='throughput_mean'
    ).reset_index()

    df_pivot['overhead_pct'] = ((df_pivot['batch'] - df_pivot['streaming']) / df_pivot['batch']) * 100
    df_pivot['streaming_ratio'] = df_pivot['streaming'] / df_pivot['batch']

    # Filter for interesting scales
    scale_order = ['Small', 'Medium', 'Large', 'VeryLarge']
    df_plot = df_pivot[df_pivot['scale'].isin(scale_order)]

    # Create figure
    fig, ax = plt.subplots(figsize=(12, 6))

    # Group by operation
    operations = df_plot['operation'].unique()
    x = np.arange(len(scale_order))
    width = 0.25

    for i, op in enumerate(operations):
        df_op = df_plot[df_plot['operation'] == op].set_index('scale').reindex(scale_order)
        offset = (i - len(operations)/2) * width + width/2

        bars = ax.bar(x + offset, df_op['overhead_pct'], width,
                      label=op, edgecolor='black', linewidth=0.5)

    # Reference line at 0% (no overhead)
    ax.axhline(y=0, color='green', linestyle='--', linewidth=1.5, alpha=0.7, label='No overhead')

    # Formatting
    ax.set_xlabel('Dataset Scale', fontweight='bold')
    ax.set_ylabel('Streaming Overhead (%)', fontweight='bold')
    ax.set_title('Record-by-Record Streaming Overhead with NEON\n(Solved by block-based processing: 10K sequence blocks)',
                 fontweight='bold', pad=15)
    ax.set_xticks(x)
    ax.set_xticklabels(['Small\n(1K seqs)', 'Medium\n(10K seqs)', 'Large\n(100K seqs)', 'VeryLarge\n(1M seqs)'])
    ax.set_ylim(0, 100)
    ax.legend(title='Operation', loc='upper right')
    ax.grid(axis='y', alpha=0.3, linestyle=':', linewidth=0.5)

    # Add annotation about solution
    ax.text(0.5, 0.95, 'Problem: 82-86% overhead destroys NEON speedup\nSolution: Process 10K sequence blocks → <10% overhead',
            transform=ax.transAxes, ha='center', va='top',
            fontsize=10, bbox=dict(boxstyle='round,pad=0.5', facecolor='yellow', alpha=0.3))

    plt.tight_layout()
    save_plot(fig, 'plot4_block_size_impact', output_dir)
    plt.close()

def plot5_mmap_threshold_effect(output_dir):
    """
    Plot 5: mmap Threshold Effect

    Shows how mmap benefits scale with file size (threshold at 50 MB).
    """
    print("\n=== Plot 5: mmap Threshold Effect ===")

    # Data from mmap findings (Test 2)
    file_sizes_mb = [0.54, 5.4, 54, 544]
    standard_io = [8092, 7192, 6524, 6162]
    mmap_madvise = [5350, 7149, 15021, 15694]

    # Calculate speedup
    speedup = [mmap / std for mmap, std in zip(mmap_madvise, standard_io)]

    # Create figure
    fig, (ax1, ax2) = plt.subplots(1, 2, figsize=(14, 6))

    # --- Left subplot: Throughput comparison ---
    x = np.arange(len(file_sizes_mb))
    width = 0.35

    bars1 = ax1.bar(x - width/2, standard_io, width,
                    label='Standard I/O', color='#95A5A6',
                    edgecolor='black', linewidth=0.5)
    bars2 = ax1.bar(x + width/2, mmap_madvise, width,
                    label='mmap + madvise', color=COLORS['mmap'],
                    edgecolor='black', linewidth=0.5)

    # Formatting
    ax1.set_xlabel('File Size (MB)', fontweight='bold')
    ax1.set_ylabel('Throughput (MB/s)', fontweight='bold')
    ax1.set_title('Throughput by File Size', fontweight='bold', pad=10)
    ax1.set_xticks(x)
    ax1.set_xticklabels(['0.54 MB\n(10K seqs)', '5.4 MB\n(100K seqs)',
                         '54 MB\n(1M seqs)', '544 MB\n(10M seqs)'])
    ax1.legend(loc='upper right')
    ax1.grid(axis='y', alpha=0.3, linestyle=':', linewidth=0.5)
    ax1.axvline(x=1.5, color='red', linestyle='--', linewidth=2, alpha=0.5)
    ax1.text(1.5, 16000, '← 50 MB threshold →', ha='center', va='top',
             fontsize=10, color='red', fontweight='bold')

    # --- Right subplot: Speedup ratio ---
    colors_speedup = ['#E74C3C' if s < 1 else '#2ECC71' for s in speedup]
    bars3 = ax2.bar(x, speedup, width=0.6, color=colors_speedup,
                    edgecolor='black', linewidth=0.5)

    # Add speedup labels
    for i, (bar, sp) in enumerate(zip(bars3, speedup)):
        height = bar.get_height()
        label = f'{sp:.2f}×'
        if sp < 1:
            label += '\n(slower!)'
            color = 'red'
        else:
            label += '\n(faster)'
            color = 'green'
        ax2.text(bar.get_x() + bar.get_width()/2., height + 0.1,
                label, ha='center', va='bottom', fontsize=10,
                fontweight='bold', color=color)

    # Reference line at 1× (no change)
    ax2.axhline(y=1, color='black', linestyle='--', linewidth=1.5, alpha=0.7, label='No change')

    # Formatting
    ax2.set_xlabel('File Size (MB)', fontweight='bold')
    ax2.set_ylabel('Speedup (mmap / standard)', fontweight='bold')
    ax2.set_title('mmap Speedup by File Size', fontweight='bold', pad=10)
    ax2.set_xticks(x)
    ax2.set_xticklabels(['0.54 MB', '5.4 MB', '54 MB', '544 MB'])
    ax2.set_ylim(0, 3)
    ax2.grid(axis='y', alpha=0.3, linestyle=':', linewidth=0.5)
    ax2.axvline(x=1.5, color='red', linestyle='--', linewidth=2, alpha=0.5)

    # Add threshold annotation
    ax2.text(0.5, 0.95, 'Threshold-based approach:\n<50 MB: Standard I/O\n≥50 MB: mmap + madvise',
            transform=ax2.transAxes, ha='center', va='top',
            fontsize=10, fontweight='bold',
            bbox=dict(boxstyle='round,pad=0.5', facecolor='yellow', alpha=0.3))

    # Overall title
    fig.suptitle('mmap Threshold Effect: File Size Determines Benefit\n(APFS optimization with madvise hints)',
                 fontsize=16, fontweight='bold', y=1.02)

    plt.tight_layout()
    save_plot(fig, 'plot5_mmap_threshold_effect', output_dir)
    plt.close()

def main():
    """Generate all publication plots"""
    print("="*60)
    print("Generating Publication-Quality Validation Plots (Artifact 3)")
    print("="*60)

    # Create output directory
    output_dir = create_output_dir()
    print(f"\nOutput directory: {output_dir}")

    # Generate all plots
    try:
        plot1_neon_speedup_by_operation(output_dir)
        plot2_streaming_memory_footprint(output_dir)
        plot3_io_optimization_stack(output_dir)
        plot4_block_size_impact(output_dir)
        plot5_mmap_threshold_effect(output_dir)

        print("\n" + "="*60)
        print("✓ All 5 plots generated successfully!")
        print("="*60)
        print(f"\nOutput location: {output_dir.absolute()}")
        print("\nFiles generated:")
        print("  1. plot1_neon_speedup_by_operation.{png,pdf}")
        print("  2. plot2_streaming_memory_footprint.{png,pdf}")
        print("  3. plot3_io_optimization_stack.{png,pdf}")
        print("  4. plot4_block_size_impact.{png,pdf}")
        print("  5. plot5_mmap_threshold_effect.{png,pdf}")
        print("\nFormat: 300 DPI PNG + vector PDF (publication-ready)")

    except Exception as e:
        print(f"\n❌ Error: {e}")
        import traceback
        traceback.print_exc()
        return 1

    return 0

if __name__ == '__main__':
    import sys
    sys.exit(main())
