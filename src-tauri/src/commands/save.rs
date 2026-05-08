// src-tauri/src/commands/save.rs
//
// 文件保存命令（File Save Command）

use anyhow::{anyhow, bail, Context, Result};
use polars::prelude::*;
use rust_xlsxwriter::{Format, Workbook};
use std::path::Path;

use crate::state::GLOBAL_DF;
use crate::types::ApiResult;

// ─────────────────────────────────────────────────────────────────────────────
// Tauri command
// ─────────────────────────────────────────────────────────────────────────────

#[tauri::command]
pub async fn save_file(path: String) -> ApiResult<String> {
    let df = {
        let guard = GLOBAL_DF.lock().unwrap();
        match guard.as_ref() {
            None => {
                return ApiResult::failure(
                    "No data loaded. Please select a file and click Load.",
                )
            }
            Some(df) => df.clone(),
        }
    };
    match save_file_impl(&df, &path) {
        Ok(()) => ApiResult::success(path),
        Err(e) => ApiResult::failure(e.to_string()),
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Implementation
// ─────────────────────────────────────────────────────────────────────────────

pub fn save_file_impl(df: &DataFrame, path: &str) -> Result<()> {
    let p = Path::new(path);
    let ext = p
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();

    match ext.as_str() {
        "csv" => save_csv(df, p),
        "xlsx" => save_xlsx(df, p),
        other => bail!("Unsupported save format: .{other} (use .csv or .xlsx)"),
    }
}

fn save_csv(df: &DataFrame, path: &Path) -> Result<()> {
    let mut file = std::fs::File::create(path)
        .with_context(|| format!("Cannot create file: {}", path.display()))?;
    CsvWriter::new(&mut file)
        .finish(&mut df.clone())
        .map_err(|e| anyhow!("CSV write error: {e}"))
}

fn save_xlsx(df: &DataFrame, path: &Path) -> Result<()> {
    let mut workbook = Workbook::new();
    let sheet = workbook.add_worksheet();

    let header_fmt = Format::new().set_bold();

    let columns = df.get_columns();
    for (c, col) in columns.iter().enumerate() {
        sheet
            .write_string_with_format(0, c as u16, col.name().as_str(), &header_fmt)
            .map_err(|e| anyhow!("xlsx header write error: {e}"))?;
    }

    for r in 0..df.height() {
        for (c, col) in columns.iter().enumerate() {
            let row = (r + 1) as u32;
            let col_idx = c as u16;
            let series = col.as_materialized_series();

            match series.get(r).unwrap_or(AnyValue::Null) {
                AnyValue::Null => {}
                AnyValue::Boolean(v) => {
                    sheet
                        .write_boolean(row, col_idx, v)
                        .map_err(|e| anyhow!("{e}"))?;
                }
                AnyValue::Int8(v) => {
                    sheet
                        .write_number(row, col_idx, v as f64)
                        .map_err(|e| anyhow!("{e}"))?;
                }
                AnyValue::Int16(v) => {
                    sheet
                        .write_number(row, col_idx, v as f64)
                        .map_err(|e| anyhow!("{e}"))?;
                }
                AnyValue::Int32(v) => {
                    sheet
                        .write_number(row, col_idx, v as f64)
                        .map_err(|e| anyhow!("{e}"))?;
                }
                AnyValue::Int64(v) => {
                    sheet
                        .write_number(row, col_idx, v as f64)
                        .map_err(|e| anyhow!("{e}"))?;
                }
                AnyValue::UInt8(v) => {
                    sheet
                        .write_number(row, col_idx, v as f64)
                        .map_err(|e| anyhow!("{e}"))?;
                }
                AnyValue::UInt16(v) => {
                    sheet
                        .write_number(row, col_idx, v as f64)
                        .map_err(|e| anyhow!("{e}"))?;
                }
                AnyValue::UInt32(v) => {
                    sheet
                        .write_number(row, col_idx, v as f64)
                        .map_err(|e| anyhow!("{e}"))?;
                }
                AnyValue::UInt64(v) => {
                    sheet
                        .write_number(row, col_idx, v as f64)
                        .map_err(|e| anyhow!("{e}"))?;
                }
                AnyValue::Float32(v) => {
                    sheet
                        .write_number(row, col_idx, v as f64)
                        .map_err(|e| anyhow!("{e}"))?;
                }
                AnyValue::Float64(v) => {
                    sheet
                        .write_number(row, col_idx, v)
                        .map_err(|e| anyhow!("{e}"))?;
                }
                other => {
                    sheet
                        .write_string(row, col_idx, &format!("{other}"))
                        .map_err(|e| anyhow!("{e}"))?;
                }
            }
        }
    }

    workbook
        .save(path)
        .map_err(|e| anyhow!("xlsx save error: {e}"))
}
