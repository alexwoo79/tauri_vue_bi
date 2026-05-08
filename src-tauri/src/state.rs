// src-tauri/src/state.rs
//
// 全局共享状态（Global Shared State）
//
// 所有跨命令共享的全局变量及其持久化逻辑集中在此文件。
// 其他模块通过 `use crate::state::*` 引入全部状态与辅助函数。

use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result};
use once_cell::sync::Lazy;
use polars::io::parquet::read::ParquetReader;
use polars::io::parquet::write::ParquetWriter;
use polars::prelude::*;

use crate::types::{DatasetMeta, DatasetRecord, PersistedDatasetState};

// ─────────────────────────────────────────────────────────────────────────────
// Global in-memory state
// ─────────────────────────────────────────────────────────────────────────────

/// The currently loaded DataFrame, shared across all Tauri commands.
pub static GLOBAL_DF: Lazy<Mutex<Option<DataFrame>>> = Lazy::new(|| Mutex::new(None));
/// Snapshot of the DataFrame right after `load_file`, used for rollback.
pub static ORIGINAL_DF: Lazy<Mutex<Option<DataFrame>>> = Lazy::new(|| Mutex::new(None));
/// History stack for step-wise clean undo.
pub static CLEAN_HISTORY: Lazy<Mutex<Vec<DataFrame>>> = Lazy::new(|| Mutex::new(Vec::new()));
/// In-memory dataset registry for switching between loaded/derived datasets.
pub static DATASET_REGISTRY: Lazy<Mutex<Vec<DatasetRecord>>> = Lazy::new(|| Mutex::new(Vec::new()));
/// Currently active dataset id.
pub static ACTIVE_DATASET_ID: Lazy<Mutex<Option<String>>> = Lazy::new(|| Mutex::new(None));

// ─────────────────────────────────────────────────────────────────────────────
// Macro: take current DF or return early with an error
// ─────────────────────────────────────────────────────────────────────────────

/// Locks GLOBAL_DF, clones the inner DataFrame and immediately releases the lock.
/// Returns an `ApiResult::failure` if no data is loaded.
#[macro_export]
macro_rules! take_df {
    () => {{
        let guard = $crate::state::GLOBAL_DF.lock().unwrap();
        match guard.as_ref() {
            None => {
                return $crate::types::ApiResult::failure(
                    "No data loaded. Please select a file and click Load.",
                )
            }
            Some(df) => df.clone(),
        }
    }};
}

// ─────────────────────────────────────────────────────────────────────────────
// Dataset persistence helpers
// ─────────────────────────────────────────────────────────────────────────────

pub fn dataset_store_dir() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    Path::new(&home)
        .join(".tauri_vue_bi")
        .join("dataset_registry")
}

fn dataset_state_file() -> PathBuf {
    dataset_store_dir().join("state.json")
}

fn dataset_parquet_file(dataset_id: &str) -> PathBuf {
    dataset_store_dir().join(format!("{dataset_id}.parquet"))
}

fn dataset_legacy_csv_file(dataset_id: &str) -> PathBuf {
    dataset_store_dir().join(format!("{dataset_id}.csv"))
}

pub fn persist_dataset_registry() -> Result<()> {
    let dir = dataset_store_dir();
    fs::create_dir_all(&dir)
        .with_context(|| format!("create dataset store dir failed: {}", dir.display()))?;

    let active_id = ACTIVE_DATASET_ID.lock().unwrap().clone();
    let records: Vec<DatasetRecord> = DATASET_REGISTRY.lock().unwrap().clone();

    for rec in &records {
        let file_path = dataset_parquet_file(rec.meta.id.as_str());
        let mut f = std::fs::File::create(&file_path)
            .with_context(|| format!("create dataset file failed: {}", file_path.display()))?;
        ParquetWriter::new(&mut f)
            .finish(&mut rec.df.clone())
            .map_err(|e| anyhow::anyhow!("write dataset parquet failed: {e}"))?;
    }

    let keep_ids: HashSet<String> = records.iter().map(|r| r.meta.id.clone()).collect();
    for entry in fs::read_dir(&dir)
        .with_context(|| format!("read dataset store dir failed: {}", dir.display()))?
    {
        let entry = entry.map_err(|e| anyhow::anyhow!("read_dir entry error: {e}"))?;
        let path = entry.path();
        let ext = path.extension().and_then(|s| s.to_str());
        if ext != Some("parquet") && ext != Some("csv") {
            continue;
        }
        let stem = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or_default();
        if !keep_ids.contains(stem) {
            let _ = fs::remove_file(path);
        }
    }

    let state = PersistedDatasetState {
        active_dataset_id: active_id,
        datasets: records.into_iter().map(|r| r.meta).collect(),
    };
    let state_file = dataset_state_file();
    let state_text = serde_json::to_string_pretty(&state)?;
    fs::write(&state_file, state_text)
        .with_context(|| format!("write state file failed: {}", state_file.display()))?;

    Ok(())
}

pub fn load_persisted_dataset_registry() -> Result<()> {
    let state_file = dataset_state_file();
    if !state_file.exists() {
        return Ok(());
    }

    let state_text = fs::read_to_string(&state_file)
        .with_context(|| format!("read state file failed: {}", state_file.display()))?;
    let state: PersistedDatasetState = serde_json::from_str(&state_text)?;

    let mut records: Vec<DatasetRecord> = Vec::new();
    let mut migrated_from_csv = false;
    for mut meta in state.datasets {
        let parquet_file = dataset_parquet_file(meta.id.as_str());
        let legacy_csv_file = dataset_legacy_csv_file(meta.id.as_str());

        let df = if parquet_file.exists() {
            let mut f = std::fs::File::open(&parquet_file).with_context(|| {
                format!("open dataset parquet failed: {}", parquet_file.display())
            })?;
            ParquetReader::new(&mut f)
                .finish()
                .map_err(|e| anyhow::anyhow!("read dataset parquet failed: {e}"))?
        } else if legacy_csv_file.exists() {
            migrated_from_csv = true;
            CsvReadOptions::default()
                .with_has_header(true)
                .try_into_reader_with_file_path(Some(legacy_csv_file.clone()))
                .map_err(|e| anyhow::anyhow!("open legacy dataset csv failed: {e}"))?
                .finish()
                .map_err(|e| anyhow::anyhow!("read legacy dataset csv failed: {e}"))?
        } else {
            continue;
        };

        meta.total_rows = df.height();
        meta.total_cols = df.width();
        records.push(DatasetRecord { meta, df });
    }

    let restored_active_id = state.active_dataset_id.and_then(|id| {
        if records.iter().any(|r| r.meta.id == id) {
            Some(id)
        } else {
            None
        }
    });

    *DATASET_REGISTRY.lock().unwrap() = records.clone();
    *ACTIVE_DATASET_ID.lock().unwrap() = restored_active_id.clone();

    let current = if let Some(id) = restored_active_id {
        records.iter().find(|r| r.meta.id == id).cloned()
    } else {
        records.first().cloned()
    };

    if let Some(rec) = current {
        *GLOBAL_DF.lock().unwrap() = Some(rec.df.clone());
        *ORIGINAL_DF.lock().unwrap() = Some(rec.df.clone());
        *ACTIVE_DATASET_ID.lock().unwrap() = Some(rec.meta.id);
    }

    if migrated_from_csv {
        persist_dataset_registry()?;
    }

    Ok(())
}

/// Register a new dataset in the in-memory registry and persist it.
pub fn register_dataset(df: &DataFrame, name: String, source: String) -> DatasetMeta {
    let created_at_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    let id = format!("ds_{created_at_ms}");
    let meta = DatasetMeta {
        id: id.clone(),
        name,
        source,
        total_rows: df.height(),
        total_cols: df.width(),
        created_at_ms,
    };

    {
        DATASET_REGISTRY.lock().unwrap().push(DatasetRecord {
            meta: meta.clone(),
            df: df.clone(),
        });
        *ACTIVE_DATASET_ID.lock().unwrap() = Some(id);
    }
    if let Err(e) = persist_dataset_registry() {
        eprintln!("persist dataset registry failed: {e}");
    }
    meta
}

/// Update the active dataset record with a new (cleaned) DataFrame and persist.
pub fn sync_active_dataset(df: &DataFrame) {
    let active_id = ACTIVE_DATASET_ID.lock().unwrap().clone();
    let Some(id) = active_id else {
        return;
    };

    let mut registry = DATASET_REGISTRY.lock().unwrap();
    if let Some(rec) = registry.iter_mut().find(|r| r.meta.id == id) {
        rec.df = df.clone();
        rec.meta.total_rows = df.height();
        rec.meta.total_cols = df.width();
    }
    drop(registry);
    if let Err(e) = persist_dataset_registry() {
        eprintln!("persist dataset registry failed: {e}");
    }
}
