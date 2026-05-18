// src-tauri/src/agent/functions/charts/radar_chart.rs
//
// 雷达图生成模块

use anyhow::{Context, Result};
use plotly::{Plot, Layout, ScatterPolar};
use plotly::common::Title;
use serde_json::json;
use std::collections::HashMap;

use super::base::{FieldMapping, ChartOptions, ChartResult};

pub fn generate_radar_chart(
    data: Vec<HashMap<String, serde_json::Value>>,
    mapping: FieldMapping,
    options: ChartOptions,
) -> Result<ChartResult> {
    let dimensions = mapping.dimensions.context("Missing dimensions")?;

    let mut labels: Vec<String> = Vec::new();
    let mut values: Vec<Vec<f64>> = Vec::new();

    for row in &data {
        let mut row_values: Vec<f64> = Vec::new();
        for dim in &dimensions {
            if let Some(value) = row.get(dim).and_then(|v| v.as_f64()) {
                row_values.push(value);
            } else {
                row_values.push(0.0);
            }
        }
        if !row_values.is_empty() {
            values.push(row_values);
            if let Some(name) = row.get("name").and_then(|v| v.as_str()) {
                labels.push(name.to_string());
            } else {
                labels.push(format!("Row {}", labels.len() + 1));
            }
        }
    }

    if values.is_empty() {
        return Ok(ChartResult::error(vec!["No valid data".to_string()]));
    }

    let mut plot = Plot::new();

    for (_i, (label, row_values)) in labels.iter().zip(values.iter()).enumerate() {
        let trace = ScatterPolar::new(dimensions.clone(), row_values.clone())
            .name(label)
            .fill(plotly::common::Fill::ToNext);
        plot.add_trace(trace);
    }

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
        <div class="chart-info">ID: radar_chart | Engine: plotly | Format: dimensions列 + name列</div>
    </div>
    <div class="chart-container">{chart_html}</div>
</body>
</html>"#,
        title = options.title.clone().unwrap_or("雷达图".to_string()),
        chart_html = chart_html
    );

    let meta = json!({
        "chart_id": "radar_chart",
        "n_rows": data.len(),
        "n_dimensions": dimensions.len(),
    });

    Ok(ChartResult::success(html, meta))
}