// src-tauri/src/agent/functions/charts/mod.rs
//
// 图表模块 - 对标 Python Data-Analysis-Agent 的 Function/Charts_generation/
//
// 核心模块：
pub mod base;
pub mod chart_generate;
pub mod color_schemes;
pub mod registry;

// 具体图表类型：
pub mod area_chart;
pub mod bar_chart;
pub mod box_plot;
pub mod bubble_plot;
pub mod grouped_bar_chart;
pub mod heatmap;
pub mod histogram;
pub mod line_chart;
pub mod nightingale_chart;
pub mod pie_chart;
pub mod radar_chart;
pub mod sankey_chart;
pub mod scatter_plot;
pub mod stacked_area_chart;
pub mod stacked_bar_chart;
pub mod violin_chart;
pub mod waterfall;

// 导出核心类型和函数
pub use base::*;
pub use chart_generate::*;
pub use color_schemes::*;
pub use registry::*;
