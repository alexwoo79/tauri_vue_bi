// src-tauri/src/agent/functions/charts/scatter_plot.rs
//
// 散点图生成模块

use anyhow::{Context, Result};
use plotly::{Plot, Layout, Scatter};
use plotly::common::{Title, Mode};
use serde_json::json;
use std::collections::HashMap;

use super::base::{FieldMapping, ChartOptions, ChartResult};
use super::color_schemes::get_color_scheme;

pub fn generate_scatter_plot(
    data: Vec<HashMap<String, serde_json::Value>>,
    mapping: FieldMapping,
    options: ChartOptions,
) -> Result<ChartResult> {
    let x_col = mapping.x.context("Missing x field")?;
    let y_col = mapping.y.context("Missing y field")?;

    let color_scheme_name = options.color_scheme.as_deref().unwrap_or("mckinsey");
    let color_scheme = get_color_scheme(color_scheme_name);

    let mut x_values: Vec<f64> = Vec::new();
    let mut y_values: Vec<f64> = Vec::new();
    let mut size_values: Option<Vec<f64>> = None;

    if mapping.size.is_some() {
        size_values = Some(Vec::new());
    }

    for row in &data {
        if let (Some(x), Some(y)) = (
            // ====================== 修复 1：去掉多余的 & ======================
            row.get(&x_col).and_then(|v| v.as_f64()),
            row.get(&y_col).and_then(|v| v.as_f64())
        ) {
            x_values.push(x);
            y_values.push(y);

            if let (Some(size_col), Some(ref mut sizes)) = (mapping.size.as_ref(), &mut size_values) {
                // ====================== 修复 2：这里也去掉 & ======================
                if let Some(size) = row.get(size_col).and_then(|v| v.as_f64()) {
                    sizes.push(size);
                } else {
                    sizes.push(10.0);
                }
            }
        }
    }

    if x_values.is_empty() {
        return Ok(ChartResult::error(vec!["No valid data".to_string()]));
    }

    let mut trace = Scatter::new(x_values.clone(), y_values.clone())
        .mode(Mode::Markers)
        .name(&options.title.clone().unwrap_or("散点图".to_string()))
        .marker(plotly::common::Marker::new()
            .size(12) // 默认大小
            .color(color_scheme.primary.clone())
        );

    // 如果有 size 字段，使用平均值作为统一大小
    if let Some(sizes) = size_values {
        let avg_size: f64 = sizes.iter().sum::<f64>() / sizes.len() as f64;
        trace = trace.marker(
            plotly::common::Marker::new()
                .size(avg_size.round() as usize) // 转成 usize
                .color(color_scheme.primary.clone())
        );
    }

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
        <div class="chart-info">ID: scatter_plot | Engine: plotly | Format: x列(数值) + y列(数值) [+ size列]</div>
    </div>
    <div class="chart-container">{chart_html}</div>
</body>
</html>"#,
        title = options.title.clone().unwrap_or("散点图".to_string()),
        chart_html = chart_html
    );

    let meta = json!({
        "chart_id": "scatter_plot",
        "n_rows": data.len(),
    });

    Ok(ChartResult::success(html, meta))
}