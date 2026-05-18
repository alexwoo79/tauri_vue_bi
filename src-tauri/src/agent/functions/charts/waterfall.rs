// src-tauri/src/agent/functions/charts/waterfall.rs
//
// 瀑布图生成模块

use anyhow::{Context, Result};
use plotly::{Plot, Layout, Bar};
use plotly::common::Title;
use serde_json::json;
use std::collections::HashMap;

use super::base::{FieldMapping, ChartOptions, ChartResult};
use super::color_schemes::get_color_scheme;

pub fn generate_waterfall(
    data: Vec<HashMap<String, serde_json::Value>>,
    mapping: FieldMapping,
    options: ChartOptions,
) -> Result<ChartResult> {
    let x_col = mapping.x.context("Missing x field")?;
    let y_col = mapping.y.context("Missing y field")?;

    let color_scheme_name = options.color_scheme.as_deref().unwrap_or("mckinsey");
    let color_scheme = get_color_scheme(color_scheme_name);

    let mut x_values: Vec<String> = Vec::new();
    let mut y_values: Vec<f64> = Vec::new();

    for row in &data {
        if let (Some(x), Some(y)) = (
            row.get(&x_col).and_then(|v| v.as_str()),
            row.get(&y_col).and_then(|v| v.as_f64())
        ) {
            x_values.push(x.to_string());
            y_values.push(y);
        }
    }

    if x_values.is_empty() {
        return Ok(ChartResult::error(vec!["No valid data".to_string()]));
    }

    // 计算累积值
    let mut cumulative: Vec<f64> = Vec::new();
    let mut current = 0.0;
    for &y in &y_values {
        current += y;
        cumulative.push(current);
    }

    // 生成颜色
    let mut colors: Vec<String> = Vec::new();
    for &y in &y_values {
        if y >= 0.0 {
            colors.push(color_scheme.colors[0].clone());
        } else {
            colors.push(color_scheme.colors[5].clone());
        }
    }

    let trace_bar = Bar::new(x_values.clone(), y_values)
        .name("变化")
        .marker(plotly::common::Marker::new().colors(colors));

    let trace_line = plotly::Scatter::new(x_values, cumulative.clone())
        .mode(plotly::common::Mode::LinesMarkers)
        .name("累计")
        .marker(plotly::common::Marker::new().color(color_scheme.primary.clone()));

    let mut plot = Plot::new();
    plot.add_trace(trace_bar);
    plot.add_trace(trace_line);

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
        <div class="chart-info">ID: waterfall | Engine: plotly | Format: x列(阶段) + y列(数值)</div>
    </div>
    <div class="chart-container">{chart_html}</div>
</body>
</html>"#,
        title = options.title.clone().unwrap_or("瀑布图".to_string()),
        chart_html = chart_html
    );

    let meta = json!({
        "chart_id": "waterfall",
        "n_rows": data.len(),
        "total": cumulative.last().unwrap_or(&0.0),
    });

    Ok(ChartResult::success(html, meta))
}