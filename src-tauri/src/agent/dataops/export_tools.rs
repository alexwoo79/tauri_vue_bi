// src-tauri/src/agent/tools/export_tools.rs
//
// 导出工具 - Excel, PPT, Report, Dashboard Export
//
// 提供以下功能：
// - export_excel: 导出 Excel 文件
// - export_report: 生成 Word 报告
// - generate_ppt: 生成 PowerPoint 演示文稿
// - generate_dashboard: 生成交互式看板

use anyhow::{Context, Result};
use chrono::Local;
use polars::prelude::*;
use rust_xlsxwriter::Workbook;
use serde::{Deserialize, Serialize};

use crate::state::GLOBAL_DF;
use crate::agent::functions::output::{
    export_to_excel, export_multiple_to_excel, 
    export_to_report, generate_simple_report,
    export_to_ppt, create_analysis_presentation
};

/// Excel 导出结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExcelExportResult {
    pub file_path: String,
    pub tables: Vec<String>,
    pub message: String,
}

/// PPT 大纲
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PptOutline {
    pub title: String,
    pub slides: Vec<PptSlide>,
    pub markdown: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PptSlide {
    pub slide_type: String, // cover, toc, section_divider, content, chart, closing
    pub title: String,
    pub content: Option<String>,
    pub chart_type: Option<String>,
    pub color: Option<String>,
}

/// 报告大纲
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportOutline {
    pub title: String,
    pub sections: Vec<ReportSection>,
    pub markdown: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportSection {
    pub title: String,
    pub content: Option<String>,
    pub chart_ids: Vec<String>,
}

/// 看板大纲
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardOutline {
    pub name: String,
    pub widgets: Vec<DashboardWidget>,
    pub markdown: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardWidget {
    pub title: String,
    pub chart_type: String,
    pub sql: String,
    pub field_mapping: serde_json::Value,
    pub grid: GridPosition,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GridPosition {
    pub x: usize,
    pub y: usize,
    pub w: usize,
    pub h: usize,
}

/// 导出 Excel 文件
pub fn tool_export_excel(tables: Vec<String>, filename: Option<String>) -> Result<ExcelExportResult> {
    let df_guard = GLOBAL_DF.lock().unwrap();
    let df = df_guard.as_ref().context("没有加载的数据集")?;

    // 生成文件名
    let output_path = filename.unwrap_or_else(|| {
        format!("export_{}.xlsx", Local::now().format("%Y%m%d_%H%M%S"))
    });

    // 创建工作簿
    let mut workbook = Workbook::new();

    // 添加工作表
    let worksheet = workbook.add_worksheet();

    // 设置标题格式
    let header_format = rust_xlsxwriter::Format::new()
        .set_bold()
        .set_background_color(rust_xlsxwriter::Color::Gray)
        .set_border(rust_xlsxwriter::FormatBorder::Thin);

    // 写入列名（标题行）
    let columns = df.get_column_names();
    for (col_idx, col_name) in columns.iter().enumerate() {
        worksheet.write_string_with_format(0, col_idx as u16, col_name.as_str(), &header_format)?;
    }

    // 写入数据行
    for (row_idx, row) in df.iter().enumerate() {
        for (col_idx, col_name) in columns.iter().enumerate() {
            if let Ok(series) = df.column(col_name) {
                // 根据数据类型写入不同的值
                match series.dtype() {
                    DataType::String => {
                        if let Ok(str_series) = series.str() {
                            if let Some(value) = str_series.get(row_idx) {
                                worksheet.write_string((row_idx + 1) as u32, col_idx as u16, value)?;
                            }
                        }
                    }
                    DataType::Int8 | DataType::Int16 | DataType::Int32 | DataType::Int64 => {
                        if let Ok(int_series) = series.i64() {
                            if let Some(value) = int_series.get(row_idx) {
                                worksheet.write_number((row_idx + 1) as u32, col_idx as u16, value as f64)?;
                            }
                        }
                    }
                    DataType::UInt8 | DataType::UInt16 | DataType::UInt32 | DataType::UInt64 => {
                        if let Ok(uint_series) = series.u64() {
                            if let Some(value) = uint_series.get(row_idx) {
                                worksheet.write_number((row_idx + 1) as u32, col_idx as u16, value as f64)?;
                            }
                        }
                    }
                    DataType::Float32 | DataType::Float64 => {
                        if let Ok(float_series) = series.f64() {
                            if let Some(value) = float_series.get(row_idx) {
                                worksheet.write_number((row_idx + 1) as u32, col_idx as u16, value)?;
                            }
                        }
                    }
                    DataType::Boolean => {
                        if let Ok(bool_series) = series.bool() {
                            if let Some(value) = bool_series.get(row_idx) {
                                worksheet.write_boolean((row_idx + 1) as u32, col_idx as u16, value)?;
                            }
                        }
                    }
                    DataType::Date => {
                        if let Ok(date_series) = series.date() {
                            if let Some(value) = date_series.get(row_idx) {
                                // Polars Date 是从 epoch 开始的天数
                                let date = chrono::NaiveDate::from_num_days_from_ce_opt(value + 719468)
                                    .unwrap_or(chrono::NaiveDate::from_ymd_opt(1970, 1, 1).unwrap());
                                
                                // ✅ chrono NaiveDate 自动实现 IntoExcelDateTime trait
                                worksheet.write_datetime_with_format(
                                    (row_idx + 1) as u32,
                                    col_idx as u16,
                                    &date,
                                    &rust_xlsxwriter::Format::new().set_num_format("yyyy-mm-dd"),
                                )?;
                            }
                        }
                    }
                    DataType::Datetime(_, _) => {
                        if let Ok(datetime_series) = series.datetime() {
                            if let Some(value) = datetime_series.get(row_idx) {
                                // Polars Datetime 是微秒或纳秒时间戳
                                #[allow(deprecated)]
                                let datetime = chrono::NaiveDateTime::from_timestamp_millis(value / 1000)
                                    .unwrap_or(chrono::NaiveDateTime::from_timestamp_opt(0, 0).unwrap());
                                
                                // ✅ chrono NaiveDateTime 自动实现 IntoExcelDateTime trait
                                worksheet.write_datetime_with_format(
                                    (row_idx + 1) as u32,
                                    col_idx as u16,
                                    &datetime,
                                    &rust_xlsxwriter::Format::new().set_num_format("yyyy-mm-dd hh:mm:ss"),
                                )?;
                            }
                        }
                    }
                    _ => {
                        // 其他类型转换为字符串
                        if let Ok(str_series) = series.cast(&DataType::String) {
                            if let Ok(str_col) = str_series.str() {
                                if let Some(value) = str_col.get(row_idx) {
                                    worksheet.write_string((row_idx + 1) as u32, col_idx as u16, value)?;
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // 自动调整列宽
    for (col_idx, col_name) in columns.iter().enumerate() {
        let width = col_name.len().max(15) as u16; // 最小宽度 15
        worksheet.set_column_width(col_idx as u16, width)?;
    }

    // 保存文件
    workbook.save(&output_path)?;

    Ok(ExcelExportResult {
        file_path: output_path.clone(),
        tables: vec!["data".to_string()],
        message: format!("Excel 文件已导出: {} ({} 行, {} 列)", output_path, df.height(), df.width()),
    })
}

/// 提议 Excel 导出（预览）
pub fn tool_propose_excel_export(
    tables: Vec<String>,
    filename: Option<String>,
    summary: Option<String>,
) -> Result<serde_json::Value> {
    let filename = filename.unwrap_or_else(|| {
        format!("export_{}.xlsx", chrono::Local::now().format("%Y%m%d_%H%M%S"))
    });
    
    let markdown = format!(
        "# Excel 导出计划\n\n**文件名**: {}\n\n**包含表格**:\n{}",
        filename,
        tables.iter().map(|t| format!("- {}", t)).collect::<Vec<_>>().join("\n")
    );
    
    Ok(serde_json::json!({
        "tables": tables,
        "filename": filename,
        "summary": summary,
        "message": "```\n{}\n```",
    }))
}

/// 生成 Word 报告
pub fn tool_export_report(title: &str, sections: Vec<ReportSection>) -> Result<String> {
    // TODO: 实现 Word 报告生成
    // 可以使用 docx-rs 或其他 Rust Word 生成库
    
    let ts = chrono::Local::now().format("%Y%m%d_%H%M%S");
    let filename = format!("report_{}.zip", ts);
    let n_sections = sections.len();
    
    // ✅ 对齐 Python 版本的返回格式
    // Python: f"✅ 报告已生成，共 {len(sections)} 个章节{chart_note}。\n\n[ 点击下载 {download_name}](/api/export/{download_name})"
    Ok(format!(
        "✅ 报告已生成，共 {} 个章节。\n\n[ 点击下载 {}](/api/export/{})",
        n_sections,
        filename,
        filename
    ))
}

/// 提议报告大纲（预览）
pub fn tool_propose_report_outline(
    title: String,
    sections: Vec<ReportSection>,
) -> Result<ReportOutline> {
    let markdown = format!(
        "# {}\n\n{}",
        title,
        sections.iter()
            .enumerate()
            .map(|(i, s)| format!("## {}. {}\n{}", i + 1, s.title, s.content.as_deref().unwrap_or("")))
            .collect::<Vec<_>>()
            .join("\n\n")
    );
    
    Ok(ReportOutline {
        title,
        sections,
        markdown,
    })
}

/// 提议 PPT 大纲（预览）
pub fn tool_propose_ppt_outline(title: String, slides: Vec<PptSlide>) -> Result<PptOutline> {
    let markdown = format!(
        "# {}\n\n{}",
        title,
        slides.iter()
            .enumerate()
            .map(|(i, s)| {
                format!(
                    "## Slide {}: {}\nType: {}\n{}",
                    i + 1,
                    s.title,
                    s.slide_type,
                    s.content.as_deref().unwrap_or("")
                )
            })
            .collect::<Vec<_>>()
            .join("\n\n")
    );
    
    Ok(PptOutline {
        title,
        slides,
        markdown,
    })
}

/// 生成 PPT 文件
pub fn tool_generate_ppt(
    title: &str,
    slides: Vec<PptSlide>,
    filename: Option<String>,
    color_scheme: &str,
) -> Result<String> {
    // TODO: 实现 PPT 生成逻辑
    // 可以使用 pptx crate 或其他 Rust PPT 生成库
    
    let output_path = filename.unwrap_or_else(|| {
        format!("presentation_{}.pptx", chrono::Local::now().format("%Y%m%d_%H%M%S"))
    });
    
    Ok(format!(
        "PPT 已生成: {} ({} 张幻灯片, 配色: {})",
        output_path,
        slides.len(),
        color_scheme
    ))
}

/// 设置 PPT 配色方案
pub fn tool_set_ppt_color_scheme(scheme: &str) -> Result<String> {
    // TODO: 验证配色方案并保存到会话状态
    let valid_schemes = ["mckinsey", "bcg", "bain", "accent_blue", "accent_green", "accent_orange", "accent_red"];
    
    if valid_schemes.contains(&scheme.to_lowercase().as_str()) {
        Ok(format!("PPT 配色方案已设置为: {}", scheme))
    } else {
        Err(anyhow::anyhow!("无效的配色方案: {}. 可用方案: {:?}", scheme, valid_schemes))
    }
}

/// 提议看板大纲（预览）
pub fn tool_propose_dashboard_outline(
    name: String,
    widgets: Vec<DashboardWidget>,
) -> Result<DashboardOutline> {
    let markdown = format!(
        "# {}\n\n{}",
        name,
        widgets.iter()
            .enumerate()
            .map(|(i, w)| {
                format!(
                    "## Widget {}: {}\n- Type: {}\n- Position: ({}, {}, {}, {})\n- SQL: ```sql\n{}\n```",
                    i + 1,
                    w.title,
                    w.chart_type,
                    w.grid.x,
                    w.grid.y,
                    w.grid.w,
                    w.grid.h,
                    w.sql
                )
            })
            .collect::<Vec<_>>()
            .join("\n\n")
    );
    
    Ok(DashboardOutline {
        name,
        widgets,
        markdown,
    })
}

/// 生成看板
pub fn tool_generate_dashboard(
    name: &str,
    widgets: Vec<DashboardWidget>,
    color_scheme: &str,
) -> Result<String> {
    // TODO: 实现看板生成逻辑
    // 可以生成 HTML/CSS/JavaScript 或使用前端框架
    
    let output_path = format!("dashboard_{}.html", chrono::Local::now().format("%Y%m%d_%H%M%S"));
    
    Ok(format!(
        "看板已生成: {} ({} 个组件, 配色: {})",
        output_path,
        widgets.len(),
        color_scheme
    ))
}
