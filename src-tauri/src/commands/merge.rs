// src-tauri/src/commands/merge.rs
//
// 数据表合并命令（Merge / Join / Concat Commands）
//
// 支持两种合并模式：
//   1. join_datasets   — 横向 JOIN（inner / left / right / outer）
//   2. concat_datasets — 纵向 CONCAT（上下堆叠，支持宽松模式）

use anyhow::{bail, Result};
use polars::prelude::*;
use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::commands::loader::load_file_impl;
use crate::df_util::{df_to_payload, CHART_LIMIT};
use crate::state::{register_dataset, DATASET_REGISTRY, GLOBAL_DF};
use crate::types::{ApiResult, ChartPayload};

// ─────────────────────────────────────────────────────────────────────────────
// join_datasets
// ─────────────────────────────────────────────────────────────────────────────

/// 将 GLOBAL_DF（左表）与指定数据集（右表）按键列进行 JOIN。
///
/// 参数：
///   right_dataset_id  — 右表的数据集 ID（来自 DATASET_REGISTRY）
///   left_on           — 左表的连接键列名列表
///   right_on          — 右表的连接键列名列表（与 left_on 一一对应）
///   how               — 连接类型："inner" | "left" | "right" | "outer"
///   save_as_dataset   — 是否将结果保存到数据集列表
///   dataset_name      — 保存时使用的名称（可选）
#[tauri::command]
pub async fn join_datasets(
    right_dataset_id: String,
    left_on: Vec<String>,
    right_on: Vec<String>,
    how: String,
    save_as_dataset: Option<bool>,
    dataset_name: Option<String>,
) -> ApiResult<ChartPayload> {
    if left_on.is_empty() || left_on.len() != right_on.len() {
        return ApiResult::failure("连接键不能为空，且左右键列数量必须相等");
    }

    let left_df = {
        let guard = GLOBAL_DF.lock().unwrap();
        match guard.as_ref() {
            None => return ApiResult::failure("当前没有加载数据，请先在数据加载页面选择数据集"),
            Some(df) => df.clone(),
        }
    };

    let right_df = {
        let guard = DATASET_REGISTRY.lock().unwrap();
        match guard.iter().find(|r| r.meta.id == right_dataset_id) {
            None => return ApiResult::failure("找不到指定的右表数据集"),
            Some(rec) => rec.df.clone(),
        }
    };

    match join_impl(&left_df, &right_df, &left_on, &right_on, &how) {
        Ok(result) => {
            if save_as_dataset.unwrap_or(false) {
                let name = dataset_name
                    .filter(|n| !n.trim().is_empty())
                    .unwrap_or_else(|| {
                        let ts = SystemTime::now()
                            .duration_since(UNIX_EPOCH)
                            .map(|d| d.as_secs())
                            .unwrap_or(0);
                        format!("JOIN结果_{ts}")
                    });
                register_dataset(&result, name, "join_datasets".to_string());
            }
            match df_to_payload(&result, Some(CHART_LIMIT)) {
                Ok(p) => ApiResult::success(p),
                Err(e) => ApiResult::failure(e.to_string()),
            }
        }
        Err(e) => ApiResult::failure(e.to_string()),
    }
}

fn join_impl(
    left: &DataFrame,
    right: &DataFrame,
    left_on: &[String],
    right_on: &[String],
    how: &str,
) -> Result<DataFrame> {
    let join_type = match how {
        "inner" => JoinType::Inner,
        "left" => JoinType::Left,
        "right" => JoinType::Right,
        "outer" | "full" => JoinType::Full,
        other => bail!("不支持的 JOIN 类型: {other}，请使用 inner / left / right / outer"),
    };

    let left_exprs: Vec<Expr> = left_on.iter().map(|s| col(s.as_str())).collect();
    let right_exprs: Vec<Expr> = right_on.iter().map(|s| col(s.as_str())).collect();

    let result = left
        .clone()
        .lazy()
        .join(
            right.clone().lazy(),
            left_exprs,
            right_exprs,
            JoinArgs::new(join_type),
        )
        .collect()?;
    Ok(result)
}

// ─────────────────────────────────────────────────────────────────────────────
// concat_datasets
// ─────────────────────────────────────────────────────────────────────────────

/// 纵向拼接多个数据集（CONCAT / UNION）。
///
/// 参数：
///   dataset_ids       — 要拼接的数据集 ID 列表（顺序即堆叠顺序）
///   include_current   — 是否将 GLOBAL_DF（当前活跃数据）作为第一张表
///   diagonal          — true = 宽松模式（允许列名不完全匹配，缺失列填 null）
///                       false = 严格模式（要求 schema 完全一致）
///   save_as_dataset   — 是否保存结果
///   dataset_name      — 保存名称（可选）
#[tauri::command]
pub async fn concat_datasets(
    dataset_ids: Vec<String>,
    include_current: Option<bool>,
    diagonal: Option<bool>,
    save_as_dataset: Option<bool>,
    dataset_name: Option<String>,
) -> ApiResult<ChartPayload> {
    let use_diagonal = diagonal.unwrap_or(false);

    // 收集待拼接的 DataFrame 列表
    let mut frames: Vec<DataFrame> = Vec::new();

    if include_current.unwrap_or(false) {
        let guard = GLOBAL_DF.lock().unwrap();
        match guard.as_ref() {
            None => return ApiResult::failure("当前没有加载数据，请先在数据加载页面选择数据集"),
            Some(df) => frames.push(df.clone()),
        }
    }

    {
        let registry = DATASET_REGISTRY.lock().unwrap();
        for id in &dataset_ids {
            match registry.iter().find(|r| &r.meta.id == id) {
                None => return ApiResult::failure(format!("找不到数据集 ID: {id}")),
                Some(rec) => frames.push(rec.df.clone()),
            }
        }
    }

    if frames.len() < 2 {
        return ApiResult::failure("请至少选择两个数据集进行拼接");
    }

    match concat_impl(frames, use_diagonal) {
        Ok(result) => {
            if save_as_dataset.unwrap_or(false) {
                let name = dataset_name
                    .filter(|n| !n.trim().is_empty())
                    .unwrap_or_else(|| {
                        let ts = SystemTime::now()
                            .duration_since(UNIX_EPOCH)
                            .map(|d| d.as_secs())
                            .unwrap_or(0);
                        format!("CONCAT结果_{ts}")
                    });
                register_dataset(&result, name, "concat_datasets".to_string());
            }
            match df_to_payload(&result, Some(CHART_LIMIT)) {
                Ok(p) => ApiResult::success(p),
                Err(e) => ApiResult::failure(e.to_string()),
            }
        }
        Err(e) => ApiResult::failure(e.to_string()),
    }
}

fn concat_impl(frames: Vec<DataFrame>, diagonal: bool) -> Result<DataFrame> {
    let lazy_frames: Vec<LazyFrame> = frames.into_iter().map(|df| df.lazy()).collect();

    let args = if diagonal {
        UnionArgs {
            diagonal: true,
            ..Default::default()
        }
    } else {
        UnionArgs::default()
    };

    let result = concat(lazy_frames, args)?.collect()?;
    Ok(result)
}

// ─────────────────────────────────────────────────────────────────────────────
// concat_paths
// ─────────────────────────────────────────────────────────────────────────────

/// 从文件/文件夹路径列表加载数据后执行纵向拼接（CONCAT / UNION）。
///
/// 支持：
///   1. 直接传入多个文件路径
///   2. 传入一个或多个文件夹路径（递归扫描支持的文件类型）
///
/// 当前支持扩展名：csv / xlsx / xls / xlsm / ods
#[tauri::command]
pub async fn concat_paths(
    paths: Vec<String>,
    diagonal: Option<bool>,
    save_as_dataset: Option<bool>,
    dataset_name: Option<String>,
) -> ApiResult<ChartPayload> {
    if paths.is_empty() {
        return ApiResult::failure("请至少提供一个文件或文件夹路径");
    }

    let use_diagonal = diagonal.unwrap_or(false);

    let files = match collect_supported_files(&paths) {
        Ok(list) => list,
        Err(e) => return ApiResult::failure(e.to_string()),
    };

    if files.len() < 2 {
        return ApiResult::failure(
            "可用于拼接的文件少于 2 个，请拖拽多个文件或包含多个文件的数据目录",
        );
    }

    let mut frames: Vec<DataFrame> = Vec::with_capacity(files.len());
    for f in &files {
        let p = f.to_string_lossy().to_string();
        match load_file_impl(&p, 0, 0, -1, false) {
            Ok(output) => frames.push(output.df),
            Err(e) => {
                return ApiResult::failure(format!("读取文件失败: {}，原因: {}", f.display(), e));
            }
        }
    }

    match concat_impl(frames, use_diagonal) {
        Ok(result) => {
            if save_as_dataset.unwrap_or(false) {
                let name = dataset_name
                    .filter(|n| !n.trim().is_empty())
                    .unwrap_or_else(|| {
                        let ts = SystemTime::now()
                            .duration_since(UNIX_EPOCH)
                            .map(|d| d.as_secs())
                            .unwrap_or(0);
                        format!("多文件汇总_{ts}")
                    });
                register_dataset(&result, name, "concat_paths".to_string());
            }
            match df_to_payload(&result, Some(CHART_LIMIT)) {
                Ok(p) => ApiResult::success(p),
                Err(e) => ApiResult::failure(e.to_string()),
            }
        }
        Err(e) => ApiResult::failure(e.to_string()),
    }
}

fn collect_supported_files(paths: &[String]) -> Result<Vec<PathBuf>> {
    let mut out: BTreeSet<PathBuf> = BTreeSet::new();

    for raw in paths {
        if raw.trim().is_empty() {
            continue;
        }
        let p = Path::new(raw);
        if !p.exists() {
            bail!("路径不存在: {raw}");
        }

        if p.is_file() {
            if is_supported_file(p) {
                out.insert(p.to_path_buf());
            }
            continue;
        }

        if p.is_dir() {
            collect_dir_files(p, &mut out)?;
            continue;
        }
    }

    Ok(out.into_iter().collect())
}

fn collect_dir_files(dir: &Path, out: &mut BTreeSet<PathBuf>) -> Result<()> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            collect_dir_files(&path, out)?;
        } else if path.is_file() && is_supported_file(&path) {
            out.insert(path);
        }
    }
    Ok(())
}

fn is_supported_file(path: &Path) -> bool {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    matches!(ext.as_str(), "csv" | "xlsx" | "xls" | "xlsm" | "ods")
}
