#!/usr/bin/env python3
"""
Complexity-Speedup Regression Analysis

Analyzes N=5 experimental data to build predictive models for NEON/parallel speedup
based on operation complexity and data scale.

Usage:
    python3 analysis/complexity_regression.py
"""

import pandas as pd
import numpy as np
import matplotlib.pyplot as plt
import seaborn as sns
from sklearn.model_selection import cross_val_score, LeaveOneOut
from sklearn.linear_model import LinearRegression, Ridge, Lasso
from sklearn.preprocessing import PolynomialFeatures, StandardScaler
from sklearn.ensemble import RandomForestRegressor, GradientBoostingRegressor
from sklearn.metrics import r2_score, mean_absolute_error, mean_squared_error
import warnings
warnings.filterwarnings('ignore')

# Set plotting style
sns.set_theme(style='whitegrid')
plt.rcParams['figure.figsize'] = (12, 8)

def load_data():
    """Load N=5 experimental data with complexity scores."""
    df = pd.read_csv('analysis/n5_complexity_data.csv')

    # Encode scale numerically (log scale: 100, 1K, 10K, 100K, 1M, 10M)
    scale_map = {
        'tiny': 2,      # 10^2
        'small': 3,     # 10^3
        'medium': 4,    # 10^4
        'large': 5,     # 10^5
        'vlarge': 6,    # 10^6
        'huge': 7       # 10^7
    }
    df['scale_log10'] = df['scale'].map(scale_map)

    # Filter out encoding-limited operation (reverse complement is outlier)
    df_filtered = df[df['operation'] != 'reverse_complement'].copy()

    print(f"Loaded {len(df)} data points ({len(df['operation'].unique())} operations)")
    print(f"Filtered to {len(df_filtered)} points (excluding reverse_complement)")
    print(f"\nOperations: {df['operation'].unique()}")

    return df, df_filtered


def exploratory_analysis(df, df_filtered):
    """Exploratory data analysis and visualization."""
    print("\n" + "="*80)
    print("EXPLORATORY DATA ANALYSIS")
    print("="*80)

    # Summary statistics
    print("\nComplexity scores:")
    complexity_summary = df.groupby('operation')['complexity_score'].first().sort_values()
    for op, score in complexity_summary.items():
        print(f"  {op:25s}: {score:.3f}")

    print("\nNEON speedup by operation (mean across scales):")
    neon_summary = df.groupby('operation')['neon_speedup'].mean().sort_values(ascending=False)
    for op, speedup in neon_summary.items():
        print(f"  {op:25s}: {speedup:.2f}×")

    # Correlation matrix
    print("\nCorrelation matrix (filtered data):")
    corr_cols = ['complexity_score', 'scale_log10', 'neon_speedup', 'parallel_speedup']
    corr = df_filtered[corr_cols].corr()
    print(corr)

    # Visualizations
    fig, axes = plt.subplots(2, 2, figsize=(14, 10))

    # 1. Complexity vs NEON speedup (all scales)
    ax = axes[0, 0]
    for operation in df_filtered['operation'].unique():
        op_data = df_filtered[df_filtered['operation'] == operation]
        ax.scatter(op_data['complexity_score'], op_data['neon_speedup'],
                   label=operation, alpha=0.7, s=100)
    ax.set_xlabel('Complexity Score')
    ax.set_ylabel('NEON Speedup (×)')
    ax.set_title('Complexity vs NEON Speedup (All Scales)')
    ax.legend()
    ax.grid(True, alpha=0.3)

    # 2. Scale vs NEON speedup (by operation)
    ax = axes[0, 1]
    for operation in df_filtered['operation'].unique():
        op_data = df_filtered[df_filtered['operation'] == operation]
        ax.plot(op_data['scale_log10'], op_data['neon_speedup'],
                marker='o', label=operation, linewidth=2)
    ax.set_xlabel('Scale (log10 sequences)')
    ax.set_ylabel('NEON Speedup (×)')
    ax.set_title('Scale-Dependent NEON Speedup')
    ax.legend()
    ax.grid(True, alpha=0.3)

    # 3. Complexity vs Parallel speedup at large scale
    ax = axes[1, 0]
    large_data = df_filtered[df_filtered['scale'] == 'large']
    for _, row in large_data.iterrows():
        ax.scatter(row['complexity_score'], row['parallel_speedup'],
                   label=row['operation'], alpha=0.7, s=150)
    ax.set_xlabel('Complexity Score')
    ax.set_ylabel('Parallel Speedup (×) at 100K')
    ax.set_title('Complexity vs Parallel Speedup (Large Scale)')
    ax.legend()
    ax.grid(True, alpha=0.3)

    # 4. Heatmap: Operation × Scale → NEON speedup
    ax = axes[1, 1]
    pivot = df_filtered.pivot_table(values='neon_speedup',
                                     index='operation',
                                     columns='scale',
                                     aggfunc='first')
    sns.heatmap(pivot, annot=True, fmt='.1f', cmap='YlOrRd', ax=ax, cbar_kws={'label': 'NEON Speedup (×)'})
    ax.set_title('NEON Speedup Heatmap: Operation × Scale')
    ax.set_xlabel('Scale')
    ax.set_ylabel('Operation')

    plt.tight_layout()
    plt.savefig('analysis/complexity_exploratory.png', dpi=300, bbox_inches='tight')
    print("\n✅ Saved exploratory plots to: analysis/complexity_exploratory.png")


def build_models(df_filtered):
    """Build and evaluate regression models."""
    print("\n" + "="*80)
    print("REGRESSION MODEL BUILDING")
    print("="*80)

    # Prepare data
    X = df_filtered[['complexity_score', 'scale_log10']].values
    y_neon = df_filtered['neon_speedup'].values
    y_parallel = df_filtered['parallel_speedup'].values

    # Standardize features
    scaler = StandardScaler()
    X_scaled = scaler.fit_transform(X)

    models = {}

    # 1. Linear Regression
    print("\n1. Linear Regression (NEON speedup)")
    lr = LinearRegression()
    lr.fit(X_scaled, y_neon)
    y_pred_lr = lr.predict(X_scaled)
    r2_lr = r2_score(y_neon, y_pred_lr)
    mae_lr = mean_absolute_error(y_neon, y_pred_lr)

    # Cross-validation
    cv_scores_lr = cross_val_score(lr, X_scaled, y_neon, cv=5, scoring='r2')

    print(f"   R² score: {r2_lr:.3f}")
    print(f"   MAE: {mae_lr:.2f}×")
    print(f"   Cross-val R² (mean): {cv_scores_lr.mean():.3f} ± {cv_scores_lr.std():.3f}")
    print(f"   Coefficients: complexity={lr.coef_[0]:.2f}, scale={lr.coef_[1]:.2f}")
    print(f"   Intercept: {lr.intercept_:.2f}")

    models['linear'] = (lr, scaler, r2_lr, mae_lr)

    # 2. Polynomial Regression (degree 2)
    print("\n2. Polynomial Regression (degree=2, NEON speedup)")
    poly = PolynomialFeatures(degree=2, include_bias=False)
    X_poly = poly.fit_transform(X_scaled)

    lr_poly = LinearRegression()
    lr_poly.fit(X_poly, y_neon)
    y_pred_poly = lr_poly.predict(X_poly)
    r2_poly = r2_score(y_neon, y_pred_poly)
    mae_poly = mean_absolute_error(y_neon, y_pred_poly)

    cv_scores_poly = cross_val_score(lr_poly, X_poly, y_neon, cv=5, scoring='r2')

    print(f"   R² score: {r2_poly:.3f}")
    print(f"   MAE: {mae_poly:.2f}×")
    print(f"   Cross-val R² (mean): {cv_scores_poly.mean():.3f} ± {cv_scores_poly.std():.3f}")
    print(f"   Features: {poly.get_feature_names_out(['complexity', 'scale'])}")

    models['polynomial'] = (lr_poly, (scaler, poly), r2_poly, mae_poly)

    # 3. Random Forest
    print("\n3. Random Forest Regressor (NEON speedup)")
    rf = RandomForestRegressor(n_estimators=100, max_depth=5, random_state=42)
    rf.fit(X, y_neon)  # RF doesn't need scaling
    y_pred_rf = rf.predict(X)
    r2_rf = r2_score(y_neon, y_pred_rf)
    mae_rf = mean_absolute_error(y_neon, y_pred_rf)

    cv_scores_rf = cross_val_score(rf, X, y_neon, cv=5, scoring='r2')

    print(f"   R² score: {r2_rf:.3f}")
    print(f"   MAE: {mae_rf:.2f}×")
    print(f"   Cross-val R² (mean): {cv_scores_rf.mean():.3f} ± {cv_scores_rf.std():.3f}")
    print(f"   Feature importances: complexity={rf.feature_importances_[0]:.3f}, scale={rf.feature_importances_[1]:.3f}")

    models['random_forest'] = (rf, None, r2_rf, mae_rf)

    # 4. Gradient Boosting
    print("\n4. Gradient Boosting Regressor (NEON speedup)")
    gb = GradientBoostingRegressor(n_estimators=100, max_depth=3, random_state=42)
    gb.fit(X, y_neon)
    y_pred_gb = gb.predict(X)
    r2_gb = r2_score(y_neon, y_pred_gb)
    mae_gb = mean_absolute_error(y_neon, y_pred_gb)

    cv_scores_gb = cross_val_score(gb, X, y_neon, cv=5, scoring='r2')

    print(f"   R² score: {r2_gb:.3f}")
    print(f"   MAE: {mae_gb:.2f}×")
    print(f"   Cross-val R² (mean): {cv_scores_gb.mean():.3f} ± {cv_scores_gb.std():.3f}")
    print(f"   Feature importances: complexity={gb.feature_importances_[0]:.3f}, scale={gb.feature_importances_[1]:.3f}")

    models['gradient_boosting'] = (gb, None, r2_gb, mae_gb)

    # Model comparison
    print("\n" + "="*80)
    print("MODEL COMPARISON")
    print("="*80)
    print(f"\n{'Model':<20} {'R² Score':<12} {'MAE (×)':<12} {'Cross-Val R²'}")
    print("-" * 60)
    print(f"{'Linear':<20} {r2_lr:<12.3f} {mae_lr:<12.2f} {cv_scores_lr.mean():.3f} ± {cv_scores_lr.std():.3f}")
    print(f"{'Polynomial (deg=2)':<20} {r2_poly:<12.3f} {mae_poly:<12.2f} {cv_scores_poly.mean():.3f} ± {cv_scores_poly.std():.3f}")
    print(f"{'Random Forest':<20} {r2_rf:<12.3f} {mae_rf:<12.2f} {cv_scores_rf.mean():.3f} ± {cv_scores_rf.std():.3f}")
    print(f"{'Gradient Boosting':<20} {r2_gb:<12.3f} {mae_gb:<12.2f} {cv_scores_gb.mean():.3f} ± {cv_scores_gb.std():.3f}")

    # Choose best model
    best_model_name = max(models, key=lambda k: models[k][2])
    print(f"\n✅ Best model: {best_model_name} (R² = {models[best_model_name][2]:.3f})")

    return models


def prediction_analysis(df_filtered, models):
    """Analyze predictions vs actual values."""
    print("\n" + "="*80)
    print("PREDICTION ANALYSIS")
    print("="*80)

    # Use best model (gradient boosting)
    gb, _, _, _ = models['gradient_boosting']

    X = df_filtered[['complexity_score', 'scale_log10']].values
    y_actual = df_filtered['neon_speedup'].values
    y_pred = gb.predict(X)

    df_filtered['neon_predicted'] = y_pred
    df_filtered['neon_error'] = y_actual - y_pred
    df_filtered['neon_error_pct'] = (df_filtered['neon_error'] / y_actual) * 100

    print("\nPrediction Errors by Operation:")
    print("-" * 80)
    print(f"{'Operation':<25} {'Scale':<10} {'Actual':<10} {'Predicted':<12} {'Error (%)'}")
    print("-" * 80)

    for _, row in df_filtered.iterrows():
        print(f"{row['operation']:<25} {row['scale']:<10} {row['neon_speedup']:>8.2f}× "
              f"{row['neon_predicted']:>10.2f}× {row['neon_error_pct']:>10.1f}%")

    # Prediction accuracy
    within_20pct = (df_filtered['neon_error_pct'].abs() <= 20).sum()
    within_50pct = (df_filtered['neon_error_pct'].abs() <= 50).sum()
    total = len(df_filtered)

    print("\nPrediction Accuracy:")
    print(f"  Within 20%: {within_20pct}/{total} ({within_20pct/total*100:.1f}%)")
    print(f"  Within 50%: {within_50pct}/{total} ({within_50pct/total*100:.1f}%)")

    # Visualization
    fig, axes = plt.subplots(1, 2, figsize=(14, 6))

    # 1. Predicted vs Actual
    ax = axes[0]
    for operation in df_filtered['operation'].unique():
        op_data = df_filtered[df_filtered['operation'] == operation]
        ax.scatter(op_data['neon_speedup'], op_data['neon_predicted'],
                   label=operation, alpha=0.7, s=100)

    # Perfect prediction line
    min_val = min(y_actual.min(), y_pred.min())
    max_val = max(y_actual.max(), y_pred.max())
    ax.plot([min_val, max_val], [min_val, max_val], 'k--', alpha=0.5, label='Perfect prediction')

    ax.set_xlabel('Actual NEON Speedup (×)')
    ax.set_ylabel('Predicted NEON Speedup (×)')
    ax.set_title('Predicted vs Actual NEON Speedup')
    ax.legend()
    ax.grid(True, alpha=0.3)

    # 2. Residuals
    ax = axes[1]
    ax.scatter(y_pred, df_filtered['neon_error'], alpha=0.7, s=100)
    ax.axhline(y=0, color='k', linestyle='--', alpha=0.5)
    ax.set_xlabel('Predicted NEON Speedup (×)')
    ax.set_ylabel('Residual (Actual - Predicted)')
    ax.set_title('Residual Plot')
    ax.grid(True, alpha=0.3)

    plt.tight_layout()
    plt.savefig('analysis/complexity_predictions.png', dpi=300, bbox_inches='tight')
    print("\n✅ Saved prediction plots to: analysis/complexity_predictions.png")


def predict_new_operations(models):
    """Predict speedup for hypothetical operations."""
    print("\n" + "="*80)
    print("PREDICTIONS FOR HYPOTHETICAL OPERATIONS")
    print("="*80)

    gb, _, _, _ = models['gradient_boosting']

    # Define hypothetical operations
    hypothetical = [
        ("Very simple counting (e.g., count A only)", 0.25, "tiny"),
        ("Very simple counting", 0.25, "large"),
        ("Medium-simple (e.g., AT count)", 0.35, "tiny"),
        ("Medium-simple", 0.35, "large"),
        ("High complexity (e.g., translate)", 0.75, "tiny"),
        ("High complexity", 0.75, "large"),
    ]

    scale_map = {'tiny': 2, 'small': 3, 'medium': 4, 'large': 5, 'vlarge': 6, 'huge': 7}

    print(f"\n{'Operation':<45} {'Complexity':<12} {'Scale':<10} {'Predicted NEON Speedup'}")
    print("-" * 90)

    for op_name, complexity, scale in hypothetical:
        scale_val = scale_map[scale]
        X_new = np.array([[complexity, scale_val]])
        y_pred = gb.predict(X_new)[0]
        print(f"{op_name:<45} {complexity:<12.2f} {scale:<10} {y_pred:>8.2f}×")


def main():
    """Main analysis pipeline."""
    print("\n" + "="*80)
    print("COMPLEXITY-SPEEDUP REGRESSION ANALYSIS")
    print("N=5 Operations: base_counting, gc_content, n_content, quality_aggregation")
    print("Excluded: reverse_complement (encoding-limited outlier)")
    print("="*80)

    # Load data
    df, df_filtered = load_data()

    # Exploratory analysis
    exploratory_analysis(df, df_filtered)

    # Build models
    models = build_models(df_filtered)

    # Prediction analysis
    prediction_analysis(df_filtered, models)

    # Predict hypothetical operations
    predict_new_operations(models)

    print("\n" + "="*80)
    print("ANALYSIS COMPLETE")
    print("="*80)
    print("\nGenerated files:")
    print("  - analysis/complexity_exploratory.png (exploratory plots)")
    print("  - analysis/complexity_predictions.png (prediction analysis)")
    print("\nNext steps:")
    print("  1. Review model performance (R² > 0.6 target)")
    print("  2. Identify outliers and anomalies")
    print("  3. Test predictions on new operations")
    print("  4. Refine complexity metric if needed")


if __name__ == '__main__':
    main()
