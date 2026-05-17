// src-tauri/src/agent/tools/plotly_tools.rs
//
// Plotly.rs 图表生成工具 - 原型测试
//
// 提供以下功能：
// - 使用 Plotly.rs 生成高级统计图表
// - 补充 ECharts 难以实现的图表类型
// - 导出 HTML 或静态图片

use anyhow::{Context, Result};
use plotly::{Plot, Scatter, Bar, Layout};
use plotly::common::{Mode, Title};
use plotly::layout::{Axis, Legend};
use serde::{Deserialize, Serialize};

/// Plotly 图表生成结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlotlyChartResult {
    pub html: String,           // HTML 字符串
    pub chart_type: String,     // 图表类型
    pub width: u32,             // 宽度
    pub height: u32,            // 高度
}

/// 生成小提琴图（Violin Chart）原型
pub fn generate_violin_chart_prototype() -> Result<PlotlyChartResult> {
    // 示例数据
    let data_a = vec![1.2, 1.5, 1.8, 2.1, 2.4, 2.7, 3.0, 3.3, 3.6, 3.9];
    let data_b = vec![2.0, 2.3, 2.6, 2.9, 3.2, 3.5, 3.8, 4.1, 4.4, 4.7];
    
    // 创建小提琴图轨迹（注意：Plotly.rs 目前可能不直接支持 violin，这里用 box + scatter 模拟）
    // 实际应该使用 Box plot 作为替代
    
    let mut plot = Plot::new();
    
    // 添加 Box Plot 作为小提琴图的近似
    let trace_a = plotly::box_plot::Box::new()
        .y(data_a.clone())
        .name("Group A")
        .box_points(plotly::box_plot::BoxPoints::All);
    
    let trace_b = plotly::box_plot::Box::new()
        .y(data_b)
        .name("Group B")
        .box_points(plotly::box_plot::BoxPoints::All);
    
    plot.add_trace(trace_a);
    plot.add_trace(trace_b);
    
    // 设置布局
    let layout = Layout::new()
        .title(Title::new("Violin Chart Prototype (using Box Plot)"))
        .x_axis(Axis::new().title(Title::new("Groups")))
        .y_axis(Axis::new().title(Title::new("Values")))
        .legend(Legend::new());
    
    plot.set_layout(layout);
    
    // 导出为 HTML
    let html = plot.to_html();
    
    Ok(PlotlyChartResult {
        html,
        chart_type: "violin".to_string(),
        width: 800,
        height: 600,
    })
}

/// 生成桑基图（Sankey Chart）原型
pub fn generate_sankey_chart_prototype() -> Result<PlotlyChartResult> {
    use plotly::sankey::{Sankey, Link, Node};
    
    // 示例数据：简单的流程关系
    let source = vec![0, 0, 1, 1, 2, 2];
    let target = vec![2, 3, 4, 5, 6, 7];
    let value = vec![8, 4, 2, 6, 3, 5];
    
    let labels = vec![
        "Source A", "Source B", 
        "Intermediate 1", "Intermediate 2",
        "Target 1", "Target 2", "Target 3", "Target 4"
    ];
    
    let node = Node::new()
        .label(labels.iter().map(|s| s.as_str()).collect::<Vec<&str>>());
    
    let link = Link::new()
        .source(source)
        .target(target)
        .value(value);
    
    let sankey = Sankey::new()
        .node(node)
        .link(link);
    
    let mut plot = Plot::new();
    plot.add_trace(sankey);
    
    let layout = Layout::new()
        .title(Title::new())
        .width(1000)
        .height(600);
    
    plot.set_layout(layout);
    
    let html = plot.to_html();
    
    Ok(PlotlyChartResult {
        html,
        chart_type: "sankey".to_string(),
        width: 1000,
        height: 600,
    })
}

/// 生成气泡图（Bubble Chart）原型
pub fn generate_bubble_chart_prototype() -> Result<PlotlyChartResult> {
    // 示例数据
    let x = vec![1, 2, 3, 4, 5, 6, 7, 8];
    let y = vec![10, 11, 12, 13, 14, 15, 16, 17];
    let size = vec![40, 60, 80, 100, 120, 140, 160, 180];
    let color = vec!["A", "B", "A", "B", "A", "B", "A", "B"];
    
    let trace = Scatter::new(x, y)
        .mode(Mode::Markers)
        .marker(
            plotly::common::Marker::new()
                .size(size)
                .color(color)
        )
        .name("Bubbles");
    
    let mut plot = Plot::new();
    plot.add_trace(trace);
    
    let layout = Layout::new()
        .title(Title::new())
        .x_axis(Axis::new().title(Title::new()))
        .y_axis(Axis::new().title(Title::new()));
    
    plot.set_layout(layout);
    
    let html = plot.to_html();
    
    Ok(PlotlyChartResult {
        html,
        chart_type: "bubble".to_string(),
        width: 800,
        height: 600,
    })
}

/// 生成热力图（Heatmap）原型 - 用于对比 ECharts 实现
pub fn generate_heatmap_prototype() -> Result<PlotlyChartResult> {
    // 示例数据：5x5 矩阵
    let z = vec![
        vec![1, 20, 30, 40, 50],
        vec![20, 1, 60, 30, 40],
        vec![30, 60, 1, 20, 50],
        vec![40, 30, 20, 1, 60],
        vec![50, 40, 50, 60, 1],
    ];
    
    let x: Vec<usize> = (0..5).collect();
    let y: Vec<usize> = (0..5).collect();
    
    let trace = plotly::heatmap::Heatmap::new(x, y, z)
        .colorscale(plotly::common::ColorScale::Palette(plotly::common::Palette::Viridis));
    
    let mut plot = Plot::new();
    plot.add_trace(trace);
    
    let layout = Layout::new()
        .title(Title::new())
        .x_axis(Axis::new().title(Title::new()))
        .y_axis(Axis::new().title(Title::new()));
    
    plot.set_layout(layout);
    
    let html = plot.to_html();
    
    Ok(PlotlyChartResult {
        html,
        chart_type: "heatmap".to_string(),
        width: 800,
        height: 600,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_violin_chart() {
        let result = generate_violin_chart_prototype();
        assert!(result.is_ok());
        let chart = result.unwrap();
        assert!(!chart.html.is_empty());
        assert_eq!(chart.chart_type, "violin");
        
        // 保存 HTML 文件供手动检查
        std::fs::write("test_violin.html", &chart.html).unwrap();
        println!("Violin chart saved to test_violin.html");
    }
    
    #[test]
    fn test_sankey_chart() {
        let result = generate_sankey_chart_prototype();
        assert!(result.is_ok());
        let chart = result.unwrap();
        assert!(!chart.html.is_empty());
        assert_eq!(chart.chart_type, "sankey");
        
        // 保存 HTML 文件供手动检查
        std::fs::write("test_sankey.html", &chart.html).unwrap();
        println!("Sankey chart saved to test_sankey.html");
    }
    
    #[test]
    fn test_bubble_chart() {
        let result = generate_bubble_chart_prototype();
        assert!(result.is_ok());
        let chart = result.unwrap();
        assert!(!chart.html.is_empty());
        assert_eq!(chart.chart_type, "bubble");
        
        // 保存 HTML 文件供手动检查
        std::fs::write("test_bubble.html", &chart.html).unwrap();
        println!("Bubble chart saved to test_bubble.html");
    }
    
    #[test]
    fn test_heatmap_chart() {
        let result = generate_heatmap_prototype();
        assert!(result.is_ok());
        let chart = result.unwrap();
        assert!(!chart.html.is_empty());
        assert_eq!(chart.chart_type, "heatmap");
        
        // 保存 HTML 文件供手动检查
        std::fs::write("test_heatmap.html", &chart.html).unwrap();
        println!("Heatmap chart saved to test_heatmap.html");
    }
}
