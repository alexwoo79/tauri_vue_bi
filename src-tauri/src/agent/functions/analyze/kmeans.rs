// src-tauri/src/agent/functions/analyze/kmeans.rs
//
// K-Means 聚类分析模块 - 对标 Python 的 Function/Analyze/Kmeans/analyze.py
//
// 功能：使用 K-Means 算法进行聚类分析

use anyhow::{Context, Result};
use polars::prelude::*;
use serde_json::json;

/// K-Means 分析结果
#[derive(Debug, Clone)]
pub struct KMeansResult {
    pub cluster_result: DataFrame,
    pub cluster_centers: DataFrame,
    pub markdown: String,
}

/// 执行 K-Means 聚类分析
pub fn run_kmeans(
    df: &DataFrame,
    feature_columns: Vec<String>,
    n_clusters: usize,
    max_iterations: usize,
) -> Result<KMeansResult> {
    // 验证特征列存在
    let col_names: Vec<String> = df.get_column_names().iter().map(|s| s.to_string()).collect();
    for col in &feature_columns {
        if !col_names.contains(col) {
            return Err(anyhow::anyhow!("特征列 '{}' 不存在", col));
        }
    }

    // 提取特征数据
    let mut feature_data: Vec<Series> = Vec::new();
    for col in &feature_columns {
        let col_ref = df.column(col)?;
        let series = col_ref.as_series().unwrap_or_else(|| Series::new("", Vec::<f64>::new()));
        let casted = series.cast(&DataType::Float64).unwrap_or_else(|_| Series::new("", Vec::<f64>::new()));
        feature_data.push(casted);
    }

    // 创建特征 DataFrame
    let feature_df = DataFrame::new(feature_data.clone().into_iter().map(|s| s.into()).collect())?;

    // 简化的 K-Means 实现（使用确定性初始化）
    let n_rows = feature_df.height();
    let mut clusters: Vec<usize> = vec![0; n_rows];
    
    // 使用均匀采样初始化聚类中心索引
    let step = n_rows / n_clusters;
    let mut center_indices: Vec<usize> = Vec::new();
    for i in 0..n_clusters {
        center_indices.push(i * step);
    }
    
    // 获取初始聚类中心
    let mut centers: Vec<Vec<f64>> = Vec::new();
    for &idx in &center_indices {
        let mut center: Vec<f64> = Vec::new();
        for col in &feature_data {
            let val = col.f64()?.get(idx).unwrap_or(0.0);
            center.push(val);
        }
        centers.push(center);
    }

    // 迭代聚类
    for _ in 0..max_iterations {
        // 分配点到最近的聚类中心
        let mut changed = false;
        for row_idx in 0..n_rows {
            let mut min_dist = f64::INFINITY;
            let mut best_cluster = 0;
            
            for (cluster_idx, center) in centers.iter().enumerate() {
                let mut dist = 0.0;
                for (col_idx, col) in feature_data.iter().enumerate() {
                    let val = col.f64()?.get(row_idx).unwrap_or(0.0);
                    dist += (val - center[col_idx]).powi(2);
                }
                
                if dist < min_dist {
                    min_dist = dist;
                    best_cluster = cluster_idx;
                }
            }
            
            if clusters[row_idx] != best_cluster {
                clusters[row_idx] = best_cluster;
                changed = true;
            }
        }
        
        if !changed {
            break;
        }
        
        // 更新聚类中心
        for cluster_idx in 0..n_clusters {
            let mut count = 0;
            let mut sums: Vec<f64> = vec![0.0; feature_columns.len()];
            
            for row_idx in 0..n_rows {
                if clusters[row_idx] == cluster_idx {
                    count += 1;
                    for (col_idx, col) in feature_data.iter().enumerate() {
                        sums[col_idx] += col.f64()?.get(row_idx).unwrap_or(0.0);
                    }
                }
            }
            
            if count > 0 {
                for sum in &mut sums {
                    *sum /= count as f64;
                }
                centers[cluster_idx] = sums;
            }
        }
    }

    // 创建聚类结果 DataFrame
    let cluster_series = Series::new("cluster".into(), clusters.iter().map(|c| (*c + 1) as u32).collect::<Vec<u32>>());
    let mut result_df = df.clone();
    result_df.with_column(cluster_series)?;

    // 创建聚类中心 DataFrame
    let mut center_data: Vec<Series> = Vec::new();
    center_data.push(Series::new("cluster".into(), (1..=n_clusters).map(|i| i as u32).collect::<Vec<u32>>()));
    
    for (col_idx, col_name) in feature_columns.iter().enumerate() {
        let col_data: Vec<f64> = centers.iter().map(|c| c[col_idx]).collect();
        center_data.push(Series::new(col_name.clone().into(), col_data));
    }
    
    let cluster_centers = DataFrame::new(center_data.into_iter().map(|s| s.into()).collect())?;

    // 生成 Markdown 洞察
    let markdown = generate_insight(&cluster_centers, &feature_columns, n_clusters);

    Ok(KMeansResult {
        cluster_result: result_df,
        cluster_centers,
        markdown,
    })
}

/// 生成分析洞察文本
fn generate_insight(cluster_centers: &DataFrame, feature_columns: &[String], n_clusters: usize) -> String {
    let mut markdown = format!("## K-Means 聚类分析结果\n\n### 概览\n- **聚类数**: {}\n- **特征数**: {}\n\n### 聚类中心\n", n_clusters, feature_columns.len());
    
    markdown.push_str("| 聚类 | ");
    for col in feature_columns {
        markdown.push_str(col);
        markdown.push_str(" | ");
    }
    markdown.push_str("\n|------|");
    for _ in feature_columns {
        markdown.push_str("--------|");
    }
    markdown.push_str("\n");
    
    for i in 0..n_clusters {
        markdown.push_str(&format!("| {} | ", i + 1));
        for col in feature_columns {
            let val = match cluster_centers.column(col) {
                Ok(c) => match c.f64() {
                    Ok(f) => format!("{:.2}", f.get(i).unwrap_or(0.0)),
                    Err(_) => "N/A".to_string(),
                },
                Err(_) => "N/A".to_string(),
            };
            markdown.push_str(&format!("{} | ", val));
        }
        markdown.push_str("\n");
    }
    
    markdown.push_str("\n### 建议\n");
    markdown.push_str("- 可根据聚类结果进行客户分群或异常检测\n");
    markdown.push_str("- 建议结合业务含义解释各聚类特征\n");
    
    markdown
}
