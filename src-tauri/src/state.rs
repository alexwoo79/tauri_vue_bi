// src-tauri/src/state.rs
//
// 全局共享状态（Global Shared State）
//
// 所有跨命令共享的全局变量及其持久化逻辑集中在此文件。
// 其他模块通过 `use crate::state::*` 引入全部状态与辅助函数。

use anyhow::{Context, Result};
use dirs;
use once_cell::sync::Lazy;
use polars::io::parquet::read::ParquetReader;
use polars::io::parquet::write::ParquetWriter;
use polars::prelude::*;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::types::{DatasetMeta, DatasetRecord, PersistedDatasetState, RuntimeDataset};

// ─────────────────────────────────────────────────────────────────────────────
// Global in-memory state
// ─────────────────────────────────────────────────────────────────────────────

/// The currently loaded DataFrame, shared across all Tauri commands.
pub static GLOBAL_DF: Lazy<Mutex<Option<DataFrame>>> = Lazy::new(|| Mutex::new(None));
/// Snapshot of the DataFrame right after `load_file`, used for rollback.
pub static ORIGINAL_DF: Lazy<Mutex<Option<DataFrame>>> = Lazy::new(|| Mutex::new(None));
/// History stack for step-wise clean undo.
pub static CLEAN_HISTORY: Lazy<Mutex<Vec<DataFrame>>> = Lazy::new(|| Mutex::new(Vec::new()));
/// Available datasets.
pub static DATASETS: Lazy<Mutex<Vec<DatasetMeta>>> = Lazy::new(|| Mutex::new(Vec::new()));
/// Dataset registry for runtime.
pub static DATASET_REGISTRY: Lazy<Mutex<Vec<RuntimeDataset>>> =
    Lazy::new(|| Mutex::new(Vec::new()));
/// Currently active dataset ID.
pub static ACTIVE_DATASET_ID: Lazy<Mutex<Option<String>>> = Lazy::new(|| Mutex::new(None));
/// Currently selected column names.
pub static SELECTED_COLS: Lazy<Mutex<HashSet<String>>> = Lazy::new(|| Mutex::new(HashSet::new()));
/// Global chart store for generated charts
pub static GLOBAL_CHART_STORE: Lazy<Mutex<HashMap<String, String>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

// ─────────────────────────────────────────────────────────────────────────────
// Persistence paths
// ─────────────────────────────────────────────────────────────────────────────

fn base_path() -> PathBuf {
    dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("tauri-vue-bi")
}

fn dataset_path() -> PathBuf {
    base_path().join("datasets")
}

fn state_path() -> PathBuf {
    base_path().join("state.json")
}

// ─────────────────────────────────────────────────────────────────────────────
// Dataset management
// ─────────────────────────────────────────────────────────────────────────────

/// Refresh the list of available datasets from disk.
pub fn refresh_datasets() -> Result<()> {
    let mut datasets = DATASETS.lock().unwrap();
    datasets.clear();

    let ds_path = dataset_path();
    fs::create_dir_all(&ds_path)?;

    for entry in fs::read_dir(&ds_path)? {
        let entry = entry?;
        let path = entry.path();
        let id = uuid::Uuid::new_v4().to_string();

        if path.extension().and_then(|e| e.to_str()) == Some("parquet") {
            let metadata = fs::metadata(&path)?;
            let modified_at = metadata.modified()?.duration_since(UNIX_EPOCH)?.as_secs();

            datasets.push(DatasetMeta {
                id,
                name: path.file_stem().unwrap().to_string_lossy().to_string(),
                path: path.to_string_lossy().to_string(),
                size_bytes: metadata.len(),
                modified_at_ms: modified_at,
                created_at_ms: metadata.created()?.duration_since(UNIX_EPOCH)?.as_secs(),
            });
        }
    }

    datasets.sort_by_key(|d| d.name.clone());
    Ok(())
}

/// Get the list of available datasets.
pub fn get_datasets() -> Vec<DatasetMeta> {
    DATASETS.lock().unwrap().clone()
}

/// Load a dataset by name.
pub fn load_dataset(name: &str) -> Result<DataFrame> {
    let datasets = DATASETS.lock().unwrap();

    let meta = datasets
        .iter()
        .find(|d| d.name == name)
        .ok_or_else(|| anyhow::anyhow!("Dataset not found: {}", name))?;

    let file = fs::File::open(&meta.path)?;
    let df = ParquetReader::new(file).finish()?;

    *ORIGINAL_DF.lock().unwrap() = Some(df.clone());
    *GLOBAL_DF.lock().unwrap() = Some(df.clone());
    Ok(df)
}

/// Save the current DataFrame to a dataset.
pub fn save_dataset(name: &str) -> Result<()> {
    let mut df = GLOBAL_DF.lock().unwrap();
    let df = df
        .as_mut()
        .ok_or_else(|| anyhow::anyhow!("No data loaded"))?;

    let path = dataset_path().join(format!("{}.parquet", name));
    fs::create_dir_all(dataset_path())?;

    let file = fs::File::create(&path)?;
    let writer = ParquetWriter::new(file);
    writer.finish(df)?;

    refresh_datasets()?;
    Ok(())
}

/// Delete a dataset by name.
pub fn delete_dataset(name: &str) -> Result<()> {
    let datasets = DATASETS.lock().unwrap();

    let meta = datasets
        .iter()
        .find(|d| d.name == name)
        .ok_or_else(|| anyhow::anyhow!("Dataset not found: {}", name))?;

    fs::remove_file(&meta.path)?;
    drop(datasets); // Release lock before refresh
    refresh_datasets()?;
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────
// Clean history management
// ─────────────────────────────────────────────────────────────────────────────

/// Push the current DataFrame to the clean history stack.
pub fn push_clean_history() -> Result<()> {
    let df = GLOBAL_DF.lock().unwrap();
    if let Some(df) = df.as_ref() {
        let mut history = CLEAN_HISTORY.lock().unwrap();
        history.push(df.clone());
    }
    Ok(())
}

/// Pop the last state from the clean history stack.
pub fn pop_clean_history() -> Option<DataFrame> {
    let mut history = CLEAN_HISTORY.lock().unwrap();
    let popped = history.pop();

    if let Some(df) = &popped {
        *GLOBAL_DF.lock().unwrap() = Some(df.clone());
    }

    popped
}

/// Clear the clean history stack.
pub fn clear_clean_history() {
    let mut history = CLEAN_HISTORY.lock().unwrap();
    history.clear();
}

// ─────────────────────────────────────────────────────────────────────────────
// Persistence (application state)
// ─────────────────────────────────────────────────────────────────────────────

/// Load persisted application state.
pub fn load_state() -> Result<PersistedDatasetState> {
    let path = state_path();

    if !path.exists() {
        return Ok(PersistedDatasetState {
            datasets: Vec::new(),
            current_id: None,
        });
    }

    let content = fs::read_to_string(&path)?;
    serde_json::from_str(&content).context("Failed to parse state file")
}

/// Save application state.
pub fn save_state(state: &PersistedDatasetState) -> Result<()> {
    fs::create_dir_all(base_path())?;
    let content = serde_json::to_string_pretty(state)?;
    fs::write(state_path(), content)?;
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────
// Chart store management
// ─────────────────────────────────────────────────────────────────────────────

/// Store a chart HTML with a given ID
pub fn store_chart(chart_id: &str, html: &str) {
    let mut store = GLOBAL_CHART_STORE.lock().unwrap();
    store.insert(chart_id.to_string(), html.to_string());
}

/// Retrieve a chart HTML by ID
pub fn get_chart(chart_id: &str) -> Option<String> {
    let store = GLOBAL_CHART_STORE.lock().unwrap();
    store.get(chart_id).cloned()
}

/// Remove a chart from the store
pub fn remove_chart(chart_id: &str) -> bool {
    let mut store = GLOBAL_CHART_STORE.lock().unwrap();
    store.remove(chart_id).is_some()
}

/// Clear all charts from the store
pub fn clear_charts() {
    let mut store = GLOBAL_CHART_STORE.lock().unwrap();
    store.clear();
}

// ─────────────────────────────────────────────────────────────────────────────
// Dataset registry management (for persistence)
// ─────────────────────────────────────────────────────────────────────────────

/// Register a new dataset record.
pub fn register_dataset(df: &DataFrame, name: String, source: String) -> Result<String> {
    let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();

    let id = uuid::Uuid::new_v4().to_string();

    let meta = DatasetMeta {
        id: id.clone(),
        name: name.clone(),
        path: source,
        size_bytes: df.estimated_size() as u64,
        modified_at_ms: now,
        created_at_ms: now,
    };

    let record = RuntimeDataset {
        meta,
        df: df.clone(),
    };

    let mut registry = DATASET_REGISTRY.lock().unwrap();
    registry.push(record);

    *ACTIVE_DATASET_ID.lock().unwrap() = Some(id.clone());

    Ok(id)
}

/// Load persisted dataset registry from disk.
pub fn load_persisted_dataset_registry() -> Result<()> {
    let state = load_state()?;

    let mut registry = DATASET_REGISTRY.lock().unwrap();
    registry.clear();

    for record in state.datasets {
        if let Ok(df) = load_dataset_from_path(&record.path) {
            let meta = DatasetMeta {
                id: record.id.clone(),
                name: record.name.clone(),
                path: record.path,
                size_bytes: record.size_bytes,
                modified_at_ms: record.modified_at,
                created_at_ms: record.created_at,
            };
            registry.push(RuntimeDataset { meta, df });
        }
    }

    *ACTIVE_DATASET_ID.lock().unwrap() = state.current_id;

    Ok(())
}

/// Load dataset from path (helper function)
fn load_dataset_from_path(path: &str) -> Result<DataFrame> {
    let file = fs::File::open(path)?;
    let df = ParquetReader::new(file).finish()?;
    Ok(df)
}

/// Persist the dataset registry to disk.
pub fn persist_dataset_registry() -> Result<()> {
    let registry = DATASET_REGISTRY.lock().unwrap();
    let current_id = ACTIVE_DATASET_ID.lock().unwrap().clone();

    let datasets: Vec<DatasetRecord> = registry
        .iter()
        .map(|r| DatasetRecord {
            id: r.meta.id.clone(),
            name: r.meta.name.clone(),
            path: r.meta.path.clone(),
            size_bytes: r.meta.size_bytes,
            modified_at: r.meta.modified_at_ms,
            created_at: r.meta.created_at_ms,
        })
        .collect();

    let state = PersistedDatasetState {
        datasets,
        current_id,
    };

    save_state(&state)
}

/// Sync the active dataset with the global state.
pub fn sync_active_dataset() -> Result<()> {
    // This is a placeholder - implement actual sync logic as needed
    Ok(())
}

use uuid;
