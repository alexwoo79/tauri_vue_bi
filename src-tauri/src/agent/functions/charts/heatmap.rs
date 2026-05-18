// src-tauri/src/agent/functions/charts/heatmap.rs
//
// 热力图生成模块

use anyhow::{Context, Result};
use plotly::{Plot, Layout, HeatMap};
use plotly::common::Title;
use serde_json::json;
use std::collections::{HashMap, BTreeSet};

use super::base::{FieldMapping, ChartOptions, ChartResult};

pub fn generate_heatmap(
    data: Vec<HashMap<String, serde_json::Value>>,
    mapping: FieldMapping,
    options: ChartOptions,
) -> Result<ChartResult> {
    let x_col = mapping.x.context("Missing x field")?;
    let y_col = mapping.y.context("Missing y field")?;
    let value_col = mapping.value.context("Missing value field")?;

    let mut x_set: BTreeSet<String> = BTreeSet::new();
    let mut y_set: BTreeSet<String> = BTreeSet::new();
    let mut value_map: HashMap<(String, String), f64> = HashMap::new();

    for row in &data {
        if let (Some(x), Some(y), Some(value)) = (
            row.get(&x_col).and_then(|v| v.as_str()),
            row.get(&y_col).and_then(|v| v.as_str()),
            row.get(&value_col).and_then(|v| v.as_f64())
        ) {
            x_set.insert(x.to_string());
            y_set.insert(y.to_string());
            value_map.insert((x.to_string(), y.to_string()), value);
        }
    }

    if x_set.is_empty() || y_set.is_empty() {
        return Ok(ChartResult::error(vec!["No valid data".to_string()]));
    }

    let x_values: Vec<String> = x_set.into_iter().collect();
    let y_values: Vec<String> = y_set.into_iter().collect();

    let mut z: Vec<Vec<f64>> = Vec::new();
    for y in &y_values {
        let mut row: Vec<f64> = Vec::new();
        for x in &x_values {
            row.push(*value_map.get(&(x.clone(), y.clone())).unwrap_or(&0.0));
        }
        z.push(row);
    }

    let trace = HeatMap::new(x_values.clone(), y_values.clone(), z);

    let mut plot = Plot::new();
    plot.add_trace(trace);

    let layout = Layout::new()
        .title(Title::new())
        .width(options.width.unwrap_or(800) as usize)
        .height(options.height.unwrap_or(600) as usize);

    plot.set_layout(layout);

    let chart_html = plot.to_html();

    let html = format!(
        r#"<!DOCTYPE html>
<html>
<head>
    <meta charset="UTF-8">
    <title>{title}</title>
    <style>
        body {{ margin: 0; padding: 10px; font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif; }}
        .chart-container {{ width: 100%; height: calc(100vh - 80px); }}
        .chart-header {{ display: flex; justify-content: space-between; align-items: center; margin-bottom: 10px; padding: 10px; background: #f8f9fa; border-radius: 8px; }}
        .chart-title {{ font-size: 18px; font-weight: 600; color: #333; }}
        .chart-info {{ font-size: 12px; color: #666; }}
    </style>
</head>
<body>
    <div class="chart-header">
        <div class="chart-title">{title}</div>
        <div class="chart-info">ID: heatmap | Engine: plotly | Format: x列 + y列 + 数值列</div>
    </div>
    <div class="chart-container">{chart_html}</div>
</body>
</html>"#,
        title = options.title.clone().unwrap_or("热力图".to_string()),
        chart_html = chart_html
    );

    let meta = json!({
        "chart_id": "heatmap",
        "n_rows": data.len(),
        "x_dim": x_values.len(),
        "y_dim": y_values.len(),
    });

    Ok(ChartResult::success(html, meta))
}