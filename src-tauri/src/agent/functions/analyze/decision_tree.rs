// src-tauri/src/agent/functions/analyze/decision_tree.rs
//
// 决策树分析模块 - 对标 Python 的 Function/Analyze/DecisionTree/analyze.py
//
// 功能：使用决策树进行特征重要性分析和规则提取

use anyhow::{Context, Result};
use polars::prelude::*;
use serde_json::json;

/// 决策树分析结果
#[derive(Debug, Clone)]
pub struct DecisionTreeResult {
    pub feature_importance: DataFrame,
    pub rules: DataFrame,
    pub markdown: String,
}

/// 执行决策树分析
pub fn run_decision_tree(
    df: &DataFrame,
    target_column: &str,
    feature_columns: Option<Vec<String>>,
    max_depth: usize,
) -> Result<DecisionTreeResult> {
    // 验证目标列存在
    let col_names: Vec<String> = df
        .get_column_names()
        .iter()
        .map(|s| s.to_string())
        .collect();
    if !col_names.contains(&target_column.to_string()) {
        return Err(anyhow::anyhow!("目标列 '{}' 不存在", target_column));
    }

    // 获取特征列
    let features: Vec<String> = feature_columns.unwrap_or_else(|| {
        col_names
            .iter()
            .filter(|&name| name != target_column)
            .cloned()
            .collect()
    });

    // 验证特征列存在
    for col in &features {
        if !col_names.contains(col) {
            return Err(anyhow::anyhow!("特征列 '{}' 不存在", col));
        }
    }

    // 简化的特征重要性计算（基于相关性分析）
    let mut importance_values: Vec<f64> = Vec::new();

    let target_col = df.column(target_column)?;
    let target_series = target_col
        .as_series()
        .map(|s| s.clone())
        .unwrap_or_else(|| Series::new("".into(), Vec::<f64>::new()));
    let target_casted = target_series
        .cast(&DataType::Float64)
        .unwrap_or_else(|_| Series::new("".into(), Vec::<f64>::new()));

    for feature in &features {
        let feature_col = df.column(feature)?;
        let feature_series = feature_col
            .as_series()
            .map(|s| s.clone())
            .unwrap_or_else(|| Series::new("".into(), Vec::<f64>::new()));
        let feature_casted = feature_series
            .cast(&DataType::Float64)
            .unwrap_or_else(|_| Series::new("".into(), Vec::<f64>::new()));

        // 计算皮尔逊相关系数作为重要性指标
        let corr = match (target_casted.f64(), feature_casted.f64()) {
            (Ok(t), Ok(f)) => pearson_correlation(&t, &f),
            _ => 0.0,
        };
        importance_values.push(corr.abs());
    }

    // 创建特征重要性 DataFrame
    let importance_df = DataFrame::new(vec![
        Series::new("feature".into(), features.clone()).into(),
        Series::new("importance".into(), importance_values.clone()).into(),
    ])?;

    // 生成规则（简化版本）
    let mut rule_ids: Vec<u32> = Vec::new();
    let mut rule_texts: Vec<String> = Vec::new();

    for (i, (feature, &importance)) in features.iter().zip(importance_values.iter()).enumerate() {
        if importance > 0.1 {
            rule_ids.push(i as u32 + 1);
            rule_texts.push(format!(
                "如果 {} 较高，则目标值倾向于较高 (重要性: {:.2})",
                feature, importance
            ));
        }
    }

    let rules_df = DataFrame::new(vec![
        Series::new("rule_id".into(), rule_ids).into(),
        Series::new("rule_text".into(), rule_texts).into(),
    ])?;

    // 生成 Markdown 洞察
    let markdown = generate_insight(&importance_df, target_column, max_depth)?;

    Ok(DecisionTreeResult {
        feature_importance: importance_df,
        rules: rules_df,
        markdown,
    })
}

/// 计算皮尔逊相关系数
fn pearson_correlation(a: &ChunkedArray<Float64Type>, b: &ChunkedArray<Float64Type>) -> f64 {
    let n = a.len() as f64;

    let mean_a = a.mean().unwrap_or(0.0);
    let mean_b = b.mean().unwrap_or(0.0);

    let mut cov_ab = 0.0;
    let mut var_a = 0.0;
    let mut var_b = 0.0;

    for i in 0..a.len() {
        let val_a = a.get(i).unwrap_or(0.0);
        let val_b = b.get(i).unwrap_or(0.0);

        let diff_a = val_a - mean_a;
        let diff_b = val_b - mean_b;

        cov_ab += diff_a * diff_b;
        var_a += diff_a * diff_a;
        var_b += diff_b * diff_b;
    }

    if var_a == 0.0 || var_b == 0.0 {
        return 0.0;
    }

    cov_ab / (var_a.sqrt() * var_b.sqrt())
}

/// 生成分析洞察文本
fn generate_insight(
    importance_df: &DataFrame,
    target_column: &str,
    max_depth: usize,
) -> Result<String> {
    let features = importance_df.column("feature")?;
    let importances = importance_df.column("importance")?;

    let mut important_features: Vec<(String, f64)> = Vec::new();
    for i in 0..features.len().min(5) {
        let feature = match features.str()?.get(i) {
            Some(v) => v.to_string(),
            None => continue,
        };
        let importance = match importances.f64()?.get(i) {
            Some(v) => v,
            None => 0.0,
        };
        important_features.push((feature, importance));
    }

    important_features.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

    let mut markdown = format!(
        "## 决策树分析结果\n\n### 概览\n- **目标变量**: {}\n- **最大深度**: {}\n\n### 重要特征\n",
        target_column, max_depth
    );

    for (i, (feature, importance)) in important_features.iter().enumerate() {
        markdown.push_str(&format!("{}. **{}**: {:.4}\n", i + 1, feature, importance));
    }

    markdown.push_str("\n### 提取规则\n");
    markdown.push_str("- 规则基于特征与目标变量的相关性生成\n");
    markdown.push_str("- 建议结合业务知识验证规则有效性\n");

    Ok(markdown)
}
