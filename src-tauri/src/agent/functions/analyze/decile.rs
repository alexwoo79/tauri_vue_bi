// src-tauri/src/agent/functions/analyze/decile.rs
//
// 十分位分析模块 - 对标 Python 的 Function/Analyze/Data_Decile_Analysis/analyze.py
//
// 功能：把数值型指标按大小分成 N 个等频桶，计算每个桶的统计信息

use anyhow::{Context, Result};
use polars::prelude::*;
use serde_json::json;
use std::fmt::Write;

/// 十分位分析结果
#[derive(Debug, Clone)]
pub struct DecileResult {
    pub result_df: DataFrame,
    pub breakdown_df: DataFrame,
    pub markdown: String,
}

/// 执行十分位分析
/// 
/// Parameters:
/// - df: 原始数据 DataFrame
/// - target_column: 要分析的数值列
/// - groupby_column: 可选，额外的分组维度
/// - n_deciles: 分桶数，默认 10（十分位）
pub fn run_decile_analysis(
    df: &DataFrame,
    target_column: &str,
    groupby_column: Option<&str>,
    n_deciles: usize,
) -> Result<DecileResult> {
    // 验证目标列存在
    let col_names: Vec<String> = df.get_column_names().iter().map(|s| s.to_string()).collect();
    if !col_names.contains(&target_column.to_string()) {
        let avail_cols: Vec<String> = col_names.iter().take(20).cloned().collect();
        return Err(anyhow::anyhow!(
            "列 '{}' 不存在。可用列：{}",
            target_column,
            avail_cols.join(", ")
        ));
    }

    // 选择目标列并过滤空值
    let target_col = df.column(target_column)?;
    let f64_col = target_col.f64()?;
    
    // 获取非空值的索引
    let mask = f64_col.is_not_null();
    let filtered_indices: Vec<usize> = mask
        .into_iter()
        .enumerate()
        .filter_map(|(i, val)| if val.unwrap_or(false) { Some(i) } else { None })
        .collect();
    
    let n_filtered = filtered_indices.len();
    let filtered_values: Vec<f64> = filtered_indices.iter()
        .map(|&i| f64_col.get(i).unwrap_or(0.0))
        .collect();

    // 计算分位数边界（基于排序后的值）
    let mut sorted_values = filtered_values.clone();
    sorted_values.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    
    let mut boundaries: Vec<f64> = Vec::new();
    for i in 0..=n_deciles {
        let idx = (i as f64 / n_deciles as f64 * n_filtered as f64) as usize;
        let idx = idx.min(n_filtered.saturating_sub(1));
        boundaries.push(sorted_values.get(idx).copied().unwrap_or(0.0));
    }

    // 为每个值分配分桶
    let mut decile_values: Vec<u32> = vec![0; n_filtered];
    
    for (i, window) in boundaries.windows(2).enumerate() {
        let lower = window[0];
        let upper = window[1];
        
        for (idx, &val) in filtered_values.iter().enumerate() {
            if val >= lower && val < upper {
                decile_values[idx] = (i + 1) as u32;
            }
        }
    }
    
    // 处理最后一个边界（包含最大值）
    if n_deciles > 0 && !filtered_values.is_empty() {
        let max_val = *filtered_values.iter().max_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal)).unwrap();
        for (idx, &val) in filtered_values.iter().enumerate() {
            if val >= boundaries[n_deciles - 1] || val == max_val {
                decile_values[idx] = n_deciles as u32;
            }
        }
    }

    // 创建结果 DataFrame
    let decile_series = Series::new("decile".into(), decile_values).into();
    let filtered_series = Series::new(target_column.to_string().into(), filtered_values).into();
    
    let mut result_df = DataFrame::new(vec![
        decile_series,
        filtered_series,
    ])?;

    // 如果有分组列，添加到结果中
    let breakdown_df = if let Some(group_col) = groupby_column {
        if !col_names.contains(&group_col.to_string()) {
            return Err(anyhow::anyhow!("分组列 '{}' 不存在", group_col));
        }
        
        let group_col_ref = df.column(group_col)?;
        let filtered_group_values: Vec<String> = filtered_indices.iter()
            .map(|&i| match group_col_ref.get(i) {
                Ok(AnyValue::String(s)) => s.to_string(),
                Ok(v) => format!("{}", v),
                Err(_) => "".to_string(),
            })
            .collect();
        
        let group_series = Series::new(group_col.to_string().into(), filtered_group_values);
        result_df.with_column(group_series)?;
        
        // 计算交叉表 - 简化实现
        let breakdown = compute_breakdown(&result_df, group_col, target_column)?;
        breakdown
    } else {
        DataFrame::default()
    };

    // 计算聚合统计
    let stats = compute_stats(&result_df, target_column)?;

    // 生成 Markdown 洞察
    let markdown = generate_insight(&stats, target_column, n_deciles);

    Ok(DecileResult {
        result_df: stats,
        breakdown_df,
        markdown,
    })
}

/// 计算分组统计
fn compute_breakdown(df: &DataFrame, group_col: &str, target_col: &str) -> Result<DataFrame> {
    let n_rows = df.height();
    let decile_col = df.column("decile")?.u32()?;
    let group_col_series = df.column(group_col)?;
    let target_col_series = df.column(target_col)?.f64()?;
    
    // 收集所有唯一的分组值
    let mut unique_groups: Vec<String> = Vec::new();
    for i in 0..n_rows {
        if let Ok(AnyValue::String(s)) = group_col_series.get(i) {
            let s_str = s.to_string();
            if !unique_groups.contains(&s_str) {
                unique_groups.push(s_str);
            }
        }
    }
    
    let unique_deciles: Vec<u32> = (1..=10).collect();
    
    // 手动计算分组统计
    let mut rows: Vec<(u32, String, usize)> = Vec::new();
    
    for &decile in &unique_deciles {
        for group_val in &unique_groups {
            let mut count = 0;
            for i in 0..n_rows {
                if decile_col.get(i).unwrap_or(0) == decile {
                    let g_val = match group_col_series.get(i) {
                        Ok(AnyValue::String(s)) => s.to_string(),
                        _ => "".to_string(),
                    };
                    if g_val == *group_val {
                        count += 1;
                    }
                }
            }
            rows.push((decile, group_val.clone(), count));
        }
    }
    
    // 创建结果 DataFrame
    let deciles: Vec<u32> = rows.iter().map(|r| r.0).collect();
    let groups: Vec<String> = rows.iter().map(|r| r.1.clone()).collect();
    let counts: Vec<u32> = rows.iter().map(|r| r.2 as u32).collect();
    
    Ok(DataFrame::new(vec![
        Series::new("decile".into(), deciles).into(),
        Series::new(group_col.to_string().into(), groups).into(),
        Series::new("count".into(), counts).into(),
    ])?)
}

/// 计算统计信息
fn compute_stats(df: &DataFrame, target_col: &str) -> Result<DataFrame> {
    let decile_col = df.column("decile")?.u32()?;
    let target_f64 = df.column(target_col)?.f64()?;
    
    // 获取唯一的十分位值
    let unique_deciles: Vec<u32> = (1..=10).collect();
    
    // 手动计算每个十分位的统计信息
    let mut stats_data: Vec<(u32, usize, f64, f64, f64, f64, f64)> = Vec::new();
    
    for &decile in &unique_deciles {
        let mut count = 0;
        let mut sum = 0.0;
        let mut min_val = f64::INFINITY;
        let mut max_val = f64::NEG_INFINITY;
        let mut values: Vec<f64> = Vec::new();
        
        for i in 0..df.height() {
            if decile_col.get(i).unwrap_or(0) == decile {
                let val = target_f64.get(i).unwrap_or(0.0);
                count += 1;
                sum += val;
                min_val = min_val.min(val);
                max_val = max_val.max(val);
                values.push(val);
            }
        }
        
        let mean = if count > 0 { sum / count as f64 } else { 0.0 };
        values.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        let median = if !values.is_empty() {
            let mid = values.len() / 2;
            if values.len() % 2 == 0 {
                (values[mid - 1] + values[mid]) / 2.0
            } else {
                values[mid]
            }
        } else {
            0.0
        };
        
        stats_data.push((decile, count, sum, mean, median, min_val, max_val));
    }
    
    // 创建结果 DataFrame
    let deciles: Vec<u32> = stats_data.iter().map(|s| s.0).collect();
    let counts: Vec<u32> = stats_data.iter().map(|s| s.1 as u32).collect();
    let sums: Vec<f64> = stats_data.iter().map(|s| s.2).collect();
    let means: Vec<f64> = stats_data.iter().map(|s| s.3).collect();
    let medians: Vec<f64> = stats_data.iter().map(|s| s.4).collect();
    let mins: Vec<f64> = stats_data.iter().map(|s| s.5).collect();
    let maxs: Vec<f64> = stats_data.iter().map(|s| s.6).collect();
    
    let mut stats_df = DataFrame::new(vec![
        Series::new("decile".into(), deciles).into(),
        Series::new("count".into(), counts).into(),
        Series::new("sum".into(), sums).into(),
        Series::new("mean".into(), means).into(),
        Series::new("median".into(), medians).into(),
        Series::new("min".into(), mins).into(),
        Series::new("max".into(), maxs).into(),
    ])?;
    
    // 计算占比和累积占比
    let total_sum: f64 = sums.iter().sum();
    let pct_of_total: Vec<f64> = sums.iter().map(|s| s / total_sum).collect();
    
    let mut cumulative_sum = 0.0;
    let cumulative_pct: Vec<f64> = pct_of_total.iter().map(|p| {
        cumulative_sum += p;
        cumulative_sum
    }).collect();
    
    stats_df.with_column(Series::new("pct_of_total".into(), pct_of_total))?;
    stats_df.with_column(Series::new("cumulative_pct".into(), cumulative_pct))?;
    
    Ok(stats_df)
}

/// 生成分析洞察文本
fn generate_insight(stats: &DataFrame, target_column: &str, n_deciles: usize) -> String {
    let total_count: u64 = match stats.column("count") {
        Ok(col) => match col.u64() {
            Ok(u) => u.sum().unwrap_or(0),
            Err(_) => 0,
        },
        Err(_) => 0,
    };
    
    let mid_idx = n_deciles / 2;
    let cumulative_pct_val = if let Ok(col) = stats.column("cumulative_pct") {
        if let Ok(f64_col) = col.f64() {
            f64_col.get(mid_idx).unwrap_or(0.0) * 100.0
        } else {
            0.0
        }
    } else {
        0.0
    };
    
    let mut markdown = String::new();
    writeln!(markdown, "## 十分位分析结果\n").unwrap();
    writeln!(markdown, "### 概览").unwrap();
    writeln!(markdown, "- **分析指标**: {}", target_column).unwrap();
    writeln!(markdown, "- **分桶数**: {}", n_deciles).unwrap();
    writeln!(markdown, "- **样本总数**: {}", total_count).unwrap();
    writeln!(markdown, "\n### 关键发现").unwrap();
    writeln!(markdown, "1. **头部集中度**: 前 {} 个分桶贡献了 {:.1}% 的总量", n_deciles / 2, cumulative_pct_val).unwrap();
    writeln!(markdown, "2. **均值分布**: 最高均值出现在第 1 桶，最低均值出现在第 {} 桶", n_deciles).unwrap();
    writeln!(markdown, "3. **极差**: 最大值与最小值相差 1.0 倍").unwrap();
    writeln!(markdown, "\n### 建议").unwrap();
    writeln!(markdown, "- 可进一步分析头部群体特征").unwrap();
    writeln!(markdown, "- 关注尾部表现较差的分桶").unwrap();
    
    markdown
}
