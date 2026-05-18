// src-tauri/src/agent/functions/output/report_export.rs
//
// 报告导出模块 - 对标 Python 的 Function/Output/report_export.py
//
// 功能：生成并导出分析报告

use anyhow::{Context, Result};
use polars::prelude::*;
use std::fs::File;
use std::io::Write;
use std::path::Path;

/// 报告内容结构体
#[derive(Debug, Clone)]
pub struct ReportContent {
    pub title: String,
    pub sections: Vec<ReportSection>,
    pub charts: Vec<String>, // 图表 HTML 或路径
}

/// 报告章节
#[derive(Debug, Clone)]
pub struct ReportSection {
    pub heading: String,
    pub content: String,
    pub dataframes: Vec<DataFrame>,
}

/// 将报告导出为 Markdown 格式
pub fn export_to_report(content: &ReportContent, filepath: &Path) -> Result<()> {
    let mut file = File::create(filepath)?;

    // 写入标题
    writeln!(file, "# {}", content.title)?;
    writeln!(file)?;

    // 写入各个章节
    for section in &content.sections {
        writeln!(file, "## {}", section.heading)?;
        writeln!(file, "{}", section.content)?;
        writeln!(file)?;

        // 写入 DataFrame 表格
        for (i, df) in section.dataframes.iter().enumerate() {
            writeln!(file, "### 表格 {}", i + 1)?;
            writeln!(file, "{}", dataframe_to_markdown(df))?;
            writeln!(file)?;
        }
    }

    // 写入图表
    if !content.charts.is_empty() {
        writeln!(file, "## 图表")?;
        for chart in &content.charts {
            writeln!(file, "{}", chart)?;
            writeln!(file)?;
        }
    }

    Ok(())
}

/// 将 DataFrame 转换为 Markdown 表格
fn dataframe_to_markdown(df: &DataFrame) -> String {
    let mut result = String::new();

    // 写入表头
    let headers: Vec<String> = df
        .get_column_names()
        .iter()
        .map(|s| s.to_string())
        .collect();
    result.push_str(&format!("| {} |\n", headers.join(" | ")));

    // 写入分隔线
    let separators = headers
        .iter()
        .map(|_| "---")
        .collect::<Vec<_>>()
        .join(" | ");
    result.push_str(&format!("| {} |\n", separators));

    // 写入数据行（最多显示 10 行）
    let max_rows = std::cmp::min(df.height(), 10);
    for i in 0..max_rows {
        let mut row = Vec::new();
        for col in df.get_columns() {
            let val = match col.get(i) {
                Ok(v) => format!("{}", v),
                Err(_) => "NULL".to_string(),
            };
            row.push(val);
        }
        result.push_str(&format!("| {} |\n", row.join(" | ")));
    }

    // 如果行数超过 10，添加省略提示
    if df.height() > 10 {
        result.push_str(&format!("| ... (共 {} 行) ... |\n", df.height()));
    }

    result
}

/// 生成简单的分析报告
pub fn generate_simple_report(
    title: &str,
    summary: &str,
    dataframes: &[(&str, &DataFrame)],
) -> Result<ReportContent> {
    let mut sections = Vec::new();

    // 摘要章节
    sections.push(ReportSection {
        heading: "分析摘要".to_string(),
        content: summary.to_string(),
        dataframes: Vec::new(),
    });

    // 数据表格章节
    for (name, df) in dataframes {
        sections.push(ReportSection {
            heading: format!("{}", name),
            content: "".to_string(),
            dataframes: vec![(*df).clone()],
        });
    }

    Ok(ReportContent {
        title: title.to_string(),
        sections,
        charts: Vec::new(),
    })
}
