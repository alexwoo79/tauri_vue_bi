// src-tauri/src/commands/dataset.rs
//
// 数据集管理命令（Dataset Management Commands）

use std::collections::HashSet;
use polars::prelude::*;

use crate::df_util::{df_to_payload, PREVIEW_LIMIT};
use crate::state::{
    persist_dataset_registry, register_dataset, ACTIVE_DATASET_ID, CLEAN_HISTORY,
    DATASET_REGISTRY, GLOBAL_DF, ORIGINAL_DF,
};
use crate::types::{ApiResult, ChartPayload, DatasetMeta};

#[tauri::command]
pub async fn list_datasets() -> ApiResult<Vec<DatasetMeta>> {
    let mut metas: Vec<DatasetMeta> = DATASET_REGISTRY
        .lock()
        .unwrap()
        .iter()
        .map(|r| r.meta.clone())
        .collect();
    metas.sort_by(|a, b| b.created_at_ms.cmp(&a.created_at_ms));
    ApiResult::success(metas)
}

#[tauri::command]
pub async fn switch_dataset(dataset_id: String) -> ApiResult<ChartPayload> {
    let found = DATASET_REGISTRY
        .lock()
        .unwrap()
        .iter()
        .find(|r| r.meta.id == dataset_id)
        .cloned();

    let Some(rec) = found else {
        return ApiResult::failure("Dataset not found");
    };

    *GLOBAL_DF.lock().unwrap() = Some(rec.df.clone());
    *ORIGINAL_DF.lock().unwrap() = Some(rec.df.clone());
    CLEAN_HISTORY.lock().unwrap().clear();
    *ACTIVE_DATASET_ID.lock().unwrap() = Some(rec.meta.id);
    if let Err(e) = persist_dataset_registry() {
        eprintln!("persist dataset registry failed: {e}");
    }

    match df_to_payload(&rec.df, Some(PREVIEW_LIMIT)) {
        Ok(p) => ApiResult::success(p),
        Err(e) => ApiResult::failure(e.to_string()),
    }
}

#[tauri::command]
pub async fn save_current_dataset(name: String, source: Option<String>) -> ApiResult<DatasetMeta> {
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
    let meta = register_dataset(
        &df,
        if name.trim().is_empty() {
            "子数据集".to_string()
        } else {
            name.trim().to_string()
        },
        source.unwrap_or_else(|| "manual_save".to_string()),
    );
    ApiResult::success(meta)
}

#[tauri::command]
pub async fn delete_datasets(dataset_ids: Vec<String>) -> ApiResult<Vec<DatasetMeta>> {
    let ids_set: HashSet<String> = dataset_ids.into_iter().collect();

    {
        let mut registry = DATASET_REGISTRY.lock().unwrap();
        registry.retain(|r| !ids_set.contains(&r.meta.id));
    }

    let active_id = ACTIVE_DATASET_ID.lock().unwrap().clone();
    if let Some(id) = &active_id {
        if ids_set.contains(id) {
            let first = DATASET_REGISTRY.lock().unwrap().first().cloned();
            if let Some(rec) = first {
                *ACTIVE_DATASET_ID.lock().unwrap() = Some(rec.meta.id.clone());
                *GLOBAL_DF.lock().unwrap() = Some(rec.df.clone());
                *ORIGINAL_DF.lock().unwrap() = Some(rec.df);
            } else {
                *ACTIVE_DATASET_ID.lock().unwrap() = None;
                *GLOBAL_DF.lock().unwrap() = None;
                *ORIGINAL_DF.lock().unwrap() = None;
            }
            CLEAN_HISTORY.lock().unwrap().clear();
        }
    }

    if let Err(e) = persist_dataset_registry() {
        eprintln!("persist dataset registry failed: {e}");
    }

    let mut metas: Vec<DatasetMeta> = DATASET_REGISTRY
        .lock()
        .unwrap()
        .iter()
        .map(|r| r.meta.clone())
        .collect();
    metas.sort_by(|a, b| b.created_at_ms.cmp(&a.created_at_ms));
    ApiResult::success(metas)
}

#[tauri::command]
pub async fn sort_and_save_dataset(
    sort_col: String,
    sort_asc: bool,
    dataset_name: Option<String>,
) -> ApiResult<DatasetMeta> {
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

    // 排序数据
    let sorted_df = match df.sort(
        [sort_col.as_str()],
        SortMultipleOptions::default().with_order_descending(!sort_asc),
    ) {
        Ok(result) => result,
        Err(e) => {
            return ApiResult::failure(format!("排序失败: {e}"))
        }
    };

    // 生成数据集名称
    let name = if let Some(n) = dataset_name {
        if n.trim().is_empty() {
            format!("排序结果_{}_{}", sort_col, if sort_asc { "升序" } else { "降序" })
        } else {
            n.trim().to_string()
        }
    } else {
        format!("排序结果_{}_{}", sort_col, if sort_asc { "升序" } else { "降序" })
    };

    // 保存为新数据集
    let meta = register_dataset(&sorted_df, name, "sort_and_save".to_string());
    ApiResult::success(meta)
}
