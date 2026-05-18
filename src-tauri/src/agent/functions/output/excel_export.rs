// src-tauri/src/agent/functions/output/excel_export.rs
//
// Excel 导出模块 - 对标 Python 的 Function/Output/excel_export.py
//
// 功能：将 DataFrame 导出为 Excel 文件

use anyhow::{Context, Result};
use polars::prelude::*;
use rust_xlsxwriter::{Format, Workbook, Worksheet, FormatAlign};
use std::path::Path;

/// 导出 DataFrame 到 Excel 文件
pub fn export_to_excel(df: &DataFrame, path: &Path) -> Result<()> {
    let mut workbook = Workbook::new();
    
    let header_format = Format::new()
        .set_bold()
        .set_background_color("#4472C4")
        .set_font_color("#FFFFFF")
        .set_align(FormatAlign::Center);
    
    let mut worksheet = workbook.add_worksheet();
    
    // 写入表头
    let headers: Vec<String> = df.get_column_names().iter().map(|s| s.to_string()).collect();
    worksheet.write_row(0, 0, headers.as_slice())?;
    worksheet.set_row_format(0, &header_format)?;
    
    // 写入数据
    let column_names = df.get_column_names();
    for row_idx in 0..df.height() {
        let mut row_data: Vec<String> = Vec::new();
        for col_name in &column_names {
            if let Ok(col) = df.column(col_name.as_str()) {
                match col.get(row_idx) {
                    Ok(v) => row_data.push(format!("{}", v)),
                    Err(_) => row_data.push("".to_string()),
                }
            } else {
                row_data.push("".to_string());
            }
        }
        
        worksheet.write_row(row_idx as u32 + 1, 0, row_data.as_slice())?;
    }
    
    // 自动调整列宽
    for (col_idx, _) in headers.iter().enumerate() {
        worksheet.set_column_width(col_idx as u16, 15.0)?;
    }
    
    workbook.save(path).context("Failed to save Excel file")
}

/// 导出多个 DataFrame 到 Excel 文件（每个 DataFrame 一个 Sheet）
pub fn export_multiple_to_excel(dataframes: &[(&str, &DataFrame)], path: &Path) -> Result<()> {
    let mut workbook = Workbook::new();
    
    let header_format = Format::new()
        .set_bold()
        .set_background_color("#4472C4")
        .set_font_color("#FFFFFF")
        .set_align(FormatAlign::Center);
    
    for (name, df) in dataframes {
        let mut worksheet = workbook.add_worksheet();
        worksheet.set_name(name.to_string())?;
        
        // 写入表头
        let headers: Vec<String> = df.get_column_names().iter().map(|s| s.to_string()).collect();
        worksheet.write_row(0, 0, headers.as_slice())?;
        worksheet.set_row_format(0, &header_format)?;
        
        // 写入数据
        let column_names = df.get_column_names();
        for row_idx in 0..df.height() {
            let mut row_data: Vec<String> = Vec::new();
            for col_name in &column_names {
                if let Ok(col) = df.column(col_name.as_str()) {
                    match col.get(row_idx) {
                        Ok(v) => row_data.push(format!("{}", v)),
                        Err(_) => row_data.push("".to_string()),
                    }
                } else {
                    row_data.push("".to_string());
                }
            }
            
            worksheet.write_row(row_idx as u32 + 1, 0, row_data.as_slice())?;
        }
        
        // 自动调整列宽
        for (col_idx, _) in headers.iter().enumerate() {
            worksheet.set_column_width(col_idx as u16, 15.0)?;
        }
    }
    
    workbook.save(path).context("Failed to save Excel file")
}
