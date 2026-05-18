use anyhow::{anyhow, bail, Context, Result};
use polars::prelude::*;
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION};
use reqwest::Proxy;
use reqwest::StatusCode;
use rusqlite::{types::ValueRef, Connection};
use serde_json::Value;
use sqlx::any::{AnyPoolOptions, AnyRow};
use sqlx::{Column as _, Row as _, TypeInfo as _, ValueRef as _};
use sqlx::Executor;
use std::collections::BTreeSet;
use std::error::Error;
use std::env;
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};
use std::time::Duration;

use crate::commands::loader::load_file_impl;
use crate::df_util::df_to_payload;
use crate::state::{
    persist_dataset_registry, register_dataset, ACTIVE_DATASET_ID, CLEAN_HISTORY, GLOBAL_DF,
    ORIGINAL_DF,
};
use crate::types::{ApiResult, ChartPayload};

fn now_ts_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

fn set_loaded_df(df: DataFrame, dataset_name: String, source: &str) -> ApiResult<ChartPayload> {
    let payload = df_to_payload(&df, None);
    *GLOBAL_DF.lock().unwrap() = Some(df.clone());
    *ORIGINAL_DF.lock().unwrap() = Some(df.clone());
    CLEAN_HISTORY.lock().unwrap().clear();
    let id = register_dataset(&df, dataset_name, source.to_string());
    if let Ok(id) = id {
        *ACTIVE_DATASET_ID.lock().unwrap() = Some(id);
    }
    if let Err(e) = persist_dataset_registry() {
        eprintln!("persist dataset registry failed: {e}");
    }

    match payload {
        Ok(p) => ApiResult::success(p),
        Err(e) => ApiResult::failure(e.to_string()),
    }
}

fn parse_sqlite_path(connection_string: &str) -> Result<String> {
    let s = connection_string.trim();
    if s.is_empty() {
        bail!("连接字符串不能为空");
    }
    if s == ":memory:" {
        return Ok(s.to_string());
    }

    let raw = if let Some(rest) = s.strip_prefix("sqlite://") {
        rest
    } else if let Some(rest) = s.strip_prefix("sqlite:") {
        rest
    } else {
        s
    };

    if raw.trim().is_empty() {
        bail!("无效的 SQLite 连接字符串");
    }
    Ok(raw.to_string())
}

fn is_sqlite_connection(connection_string: &str) -> bool {
    let s = connection_string.trim().to_lowercase();
    s == ":memory:"
        || s.starts_with("sqlite://")
        || s.starts_with("sqlite:")
        || !s.contains("://")
}

fn detect_sql_driver(connection_string: &str) -> &'static str {
    let s = connection_string.trim().to_lowercase();
    if is_sqlite_connection(&s) {
        return "sqlite";
    }
    if s.starts_with("mysql://") {
        return "mysql";
    }
    if s.starts_with("postgres://") || s.starts_with("postgresql://") {
        return "postgres";
    }
    "unknown"
}

fn sql_value_to_string(v: ValueRef<'_>) -> String {
    match v {
        ValueRef::Null => String::new(),
        ValueRef::Integer(i) => i.to_string(),
        ValueRef::Real(f) => f.to_string(),
        ValueRef::Text(t) => String::from_utf8_lossy(t).to_string(),
        ValueRef::Blob(b) => format!("<blob:{} bytes>", b.len()),
    }
}

fn csv_escape_cell(raw: &str) -> String {
    let needs_quote = raw.contains(',') || raw.contains('"') || raw.contains('\n') || raw.contains('\r');
    if !needs_quote {
        return raw.to_string();
    }
    let escaped = raw.replace('"', "\"\"");
    format!("\"{escaped}\"")
}

fn json_to_df(value: Value) -> Result<DataFrame> {
    let rows: Vec<serde_json::Map<String, Value>> = match value {
        Value::Array(arr) => arr
            .into_iter()
            .filter_map(|v| v.as_object().cloned())
            .collect(),
        Value::Object(obj) => {
            let mut list_hit: Option<Vec<serde_json::Map<String, Value>>> = None;
            for v in obj.values() {
                if let Value::Array(arr) = v {
                    let mapped: Vec<_> = arr
                        .iter()
                        .filter_map(|x| x.as_object().cloned())
                        .collect();
                    if !mapped.is_empty() {
                        list_hit = Some(mapped);
                        break;
                    }
                }
            }
            if let Some(v) = list_hit {
                v
            } else {
                vec![obj]
            }
        }
        _ => bail!("JSON 根节点必须是对象或对象数组"),
    };

    if rows.is_empty() {
        bail!("JSON 中未找到可转换的对象行");
    }

    let mut keys: BTreeSet<String> = BTreeSet::new();
    for row in &rows {
        for k in row.keys() {
            keys.insert(k.clone());
        }
    }

    let mut cols: Vec<Column> = Vec::with_capacity(keys.len());
    for k in keys {
        let values: Vec<Option<String>> = rows
            .iter()
            .map(|r| {
                r.get(&k).map(|v| match v {
                    Value::Null => String::new(),
                    Value::Bool(b) => b.to_string(),
                    Value::Number(n) => n.to_string(),
                    Value::String(s) => s.clone(),
                    Value::Array(_) | Value::Object(_) => v.to_string(),
                })
            })
            .collect();
        cols.push(Series::new(k.into(), values).into());
    }

    DataFrame::new(cols).map_err(|e| anyhow!("构造 DataFrame 失败: {e}"))
}

fn extract_sheet_id(url_or_id: &str) -> String {
    let s = url_or_id.trim();
    if let Some(start) = s.find("/spreadsheets/d/") {
        let start_idx = start + "/spreadsheets/d/".len();
        let rest = &s[start_idx..];
        if let Some(end) = rest.find('/') {
            return rest[..end].to_string();
        }
        return rest.to_string();
    }
    s.to_string()
}

fn extract_sheet_gid(url: &str) -> Option<String> {
    let marker = "gid=";
    let start = url.find(marker)? + marker.len();
    let tail = &url[start..];
    let digits: String = tail.chars().take_while(|c| c.is_ascii_digit()).collect();
    if digits.is_empty() {
        None
    } else {
        Some(digits)
    }
}

fn format_reqwest_error(url: &str, err: &reqwest::Error) -> String {
    let kind = if err.is_timeout() {
        "请求超时"
    } else if err.is_connect() {
        "连接失败"
    } else if err.is_request() {
        "请求构建失败"
    } else if err.is_decode() {
        "响应解析失败"
    } else {
        "请求异常"
    };

    let mut details = Vec::new();
    let mut src = err.source();
    while let Some(cause) = src {
        details.push(cause.to_string());
        src = cause.source();
    }
    let chain = if details.is_empty() {
        String::new()
    } else {
        format!("；原因链: {}", details.join(" -> "))
    };

    format!("{kind}: URL={url}；错误={err}{chain}")
}

fn temp_csv_path(prefix: &str) -> PathBuf {
    std::env::temp_dir().join(format!("{}_{}.csv", prefix, now_ts_secs()))
}

fn any_cell_to_text(row: &AnyRow, i: usize) -> Result<String> {
    if let Ok(v) = row.try_get::<Option<String>, _>(i) {
        return Ok(v.unwrap_or_default());
    }
    if let Ok(v) = row.try_get::<Option<i64>, _>(i) {
        return Ok(v.map(|x| x.to_string()).unwrap_or_default());
    }
    if let Ok(v) = row.try_get::<Option<i32>, _>(i) {
        return Ok(v.map(|x| x.to_string()).unwrap_or_default());
    }
    if let Ok(v) = row.try_get::<Option<f64>, _>(i) {
        return Ok(v.map(|x| x.to_string()).unwrap_or_default());
    }
    if let Ok(v) = row.try_get::<Option<f32>, _>(i) {
        return Ok(v.map(|x| x.to_string()).unwrap_or_default());
    }
    if let Ok(v) = row.try_get::<Option<bool>, _>(i) {
        return Ok(v.map(|x| x.to_string()).unwrap_or_default());
    }
    if let Ok(v) = row.try_get::<Option<Vec<u8>>, _>(i) {
        return Ok(v
            .map(|bytes| String::from_utf8_lossy(&bytes).to_string())
            .unwrap_or_default());
    }

    let raw = row
        .try_get_raw(i)
        .with_context(|| format!("读取 SQL 结果第 {} 列失败", i + 1))?;
    if raw.is_null() {
        return Ok(String::new());
    }
    Ok(format!("<{}>", raw.type_info().name()))
}

async fn query_sql_via_sqlx(connection_string: &str, query: &str) -> Result<String> {
    let pool = AnyPoolOptions::new()
        .max_connections(1)
        .acquire_timeout(Duration::from_secs(20))
        .connect(connection_string)
        .await
        .with_context(|| "连接数据库失败，请检查连接串、账号密码与网络")?;

    let describe = pool
        .describe(query)
        .await
        .with_context(|| "SQL 预处理失败，请检查 SQL 语法")?;

    if describe.columns().is_empty() {
        pool.close().await;
        bail!("SQL 未返回可读取列（请使用 SELECT 查询）");
    }

    let headers: Vec<String> = describe
        .columns()
        .iter()
        .map(|c| c.name().to_string())
        .collect();

    let rows: Vec<AnyRow> = sqlx::query(query)
        .fetch_all(&pool)
        .await
        .with_context(|| "SQL 执行失败")?;

    let mut csv_text = String::new();
    csv_text.push_str(
        &headers
            .iter()
            .map(|h| csv_escape_cell(h))
            .collect::<Vec<_>>()
            .join(","),
    );
    csv_text.push('\n');

    for row in &rows {
        let mut line: Vec<String> = Vec::with_capacity(headers.len());
        for i in 0..headers.len() {
            let text = any_cell_to_text(row, i)?;
            line.push(csv_escape_cell(&text));
        }
        csv_text.push_str(&line.join(","));
        csv_text.push('\n');
    }

    pool.close().await;
    Ok(csv_text)
}

fn query_sqlite_csv(path: &str, query: &str) -> Result<String> {
    let conn = Connection::open(path)
        .with_context(|| format!("当前仅支持 SQLite 连接（sqlite:///... 或绝对路径）；打开失败: {path}"))?;

    let mut stmt = conn
        .prepare(query)
        .with_context(|| "SQL 预处理失败，请检查 SQL 语法")?;

    let col_count = stmt.column_count();
    if col_count == 0 {
        bail!("SQL 未返回可读取列（请使用 SELECT 查询）");
    }

    let headers: Vec<String> = stmt
        .column_names()
        .iter()
        .map(|s| s.to_string())
        .collect();

    let mut csv_text = String::new();
    csv_text.push_str(
        &headers
            .iter()
            .map(|h| csv_escape_cell(h))
            .collect::<Vec<_>>()
            .join(","),
    );
    csv_text.push('\n');

    let mut rows = stmt.query([]).with_context(|| "SQL 执行失败")?;
    while let Ok(Some(row)) = rows.next() {
        let mut line: Vec<String> = Vec::with_capacity(col_count);
        for i in 0..col_count {
            let v = row
                .get_ref(i)
                .with_context(|| format!("读取 SQL 结果第 {} 列失败", i + 1))?;
            line.push(csv_escape_cell(&sql_value_to_string(v)));
        }
        csv_text.push_str(&line.join(","));
        csv_text.push('\n');
    }
    Ok(csv_text)
}

#[tauri::command]
pub async fn load_sql_dataset(
    connection_string: String,
    query: String,
    dataset_name: Option<String>,
) -> ApiResult<ChartPayload> {
    let q = query.trim();
    if q.is_empty() {
        return ApiResult::failure("SQL 查询不能为空");
    }

    let conn = connection_string.trim();
    if conn.is_empty() {
        return ApiResult::failure("连接字符串不能为空");
    }

    let driver = detect_sql_driver(conn);
    let csv_text = match driver {
        "sqlite" => {
            let path = match parse_sqlite_path(conn) {
                Ok(p) => p,
                Err(e) => return ApiResult::failure(e.to_string()),
            };
            match query_sqlite_csv(&path, q) {
                Ok(text) => text,
                Err(e) => return ApiResult::failure(e.to_string()),
            }
        }
        "mysql" | "postgres" => match query_sql_via_sqlx(conn, q).await {
            Ok(text) => text,
            Err(e) => return ApiResult::failure(e.to_string()),
        },
        _ => {
            return ApiResult::failure(
                "当前支持数据库连接串前缀: sqlite:///、mysql://、postgres://、postgresql://".to_string(),
            )
        }
    };

    let tmp = temp_csv_path("tauri_bi_sql");
    if let Err(e) = fs::write(&tmp, csv_text.as_bytes()) {
        return ApiResult::failure(format!("写入临时 CSV 失败: {e}"));
    }

    let output = load_file_impl(tmp.to_string_lossy().as_ref(), 0, 0, -1, false);
    let _ = fs::remove_file(&tmp);

    match output {
        Ok(out) => {
            let name = dataset_name
                .filter(|s| !s.trim().is_empty())
                .unwrap_or_else(|| format!("SQL数据_{}", now_ts_secs()));
            set_loaded_df(out.df, name, "load_sql_dataset")
        }
        Err(e) => ApiResult::failure(e.to_string()),
    }
}

#[tauri::command]
pub async fn load_google_sheet_dataset(
    spreadsheet: String,
    gid: Option<String>,
    proxy_url: Option<String>,
    dataset_name: Option<String>,
) -> ApiResult<ChartPayload> {
    let sid = extract_sheet_id(&spreadsheet);
    if sid.trim().is_empty() {
        return ApiResult::failure("Google Sheets 链接或 ID 不能为空");
    }

    let gid_from_url = extract_sheet_gid(&spreadsheet);
    let gid_text = gid.unwrap_or_default();
    let gid_trimmed = gid_text.trim();
    let gid_effective = if !gid_trimmed.is_empty() {
        gid_trimmed.to_string()
    } else if let Some(g) = gid_from_url {
        g
    } else {
        "0".to_string()
    };

    let mut url_primary = format!("https://docs.google.com/spreadsheets/d/{sid}/export?format=csv");
    if !gid_effective.is_empty() {
        url_primary.push_str("&gid=");
        url_primary.push_str(&gid_effective);
    }

    // 兼容部分地区/网络环境：主 URL 失败后尝试 gviz CSV 导出地址。
    let mut url_fallback = format!(
        "https://docs.google.com/spreadsheets/d/{sid}/gviz/tq?tqx=out:csv"
    );
    if !gid_effective.is_empty() {
        url_fallback.push_str("&gid=");
        url_fallback.push_str(&gid_effective);
    }

    let mut client_builder = reqwest::Client::builder()
        .connect_timeout(Duration::from_secs(12))
        .timeout(Duration::from_secs(30));

    let effective_proxy = proxy_url
        .unwrap_or_default()
        .trim()
        .to_string();
    if !effective_proxy.is_empty() {
        match Proxy::all(&effective_proxy) {
            Ok(p) => {
                client_builder = client_builder.proxy(p);
            }
            Err(e) => {
                return ApiResult::failure(format!("代理地址无效: {effective_proxy}，错误: {e}"));
            }
        }
    } else if let Ok(env_proxy) = env::var("HTTPS_PROXY") {
        if !env_proxy.trim().is_empty() {
            if let Ok(p) = Proxy::all(env_proxy.trim()) {
                client_builder = client_builder.proxy(p);
            }
        }
    }

    let client = match client_builder.build() {
        Ok(c) => c,
        Err(e) => return ApiResult::failure(format!("初始化 HTTP 客户端失败: {e}")),
    };

    async fn fetch_sheet_csv(client: &reqwest::Client, url: &str) -> Result<Vec<u8>> {
        let resp = client
            .get(url)
            .header(
                "User-Agent",
                "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/124.0.0.0 Safari/537.36",
            )
            .header("Accept", "text/csv,*/*")
            .send()
            .await;

        let resp = match resp {
            Ok(r) => r,
            Err(e) => bail!("{}", format_reqwest_error(url, &e)),
        };

        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            let preview = body.chars().take(180).collect::<String>();
            let reason = if status == StatusCode::FORBIDDEN {
                "权限不足（可能未公开共享）"
            } else if status == StatusCode::NOT_FOUND {
                "文档不存在或无权限访问"
            } else {
                "返回非 2xx 状态码"
            };
            bail!("{reason}，状态码: {status}，URL: {url}，响应片段: {preview}");
        }

        let bytes = resp
            .bytes()
            .await
            .with_context(|| format!("读取响应体失败: {url}"))?;
        Ok(bytes.to_vec())
    }

    let bytes = match fetch_sheet_csv(&client, &url_primary).await {
        Ok(b) => b,
        Err(primary_err) => match fetch_sheet_csv(&client, &url_fallback).await {
            Ok(b) => b,
            Err(fallback_err) => {
                return ApiResult::failure(format!(
                    "请求 Google Sheets 失败。\n主地址: {url_primary}\n主错误: {primary_err}\n备用地址: {url_fallback}\n备用错误: {fallback_err}\n当前生效 gid: {gid_effective}\n请检查网络连通性（是否可访问 docs.google.com）、表格共享权限（是否对链接可见）以及代理设置。"
                ))
            }
        },
    };

    let tmp = temp_csv_path("tauri_bi_gsheet");
    if let Err(e) = fs::write(&tmp, &bytes) {
        return ApiResult::failure(format!("写入临时 CSV 失败: {e}"));
    }

    let output = load_file_impl(tmp.to_string_lossy().as_ref(), 0, 0, -1, false);
    let _ = fs::remove_file(&tmp);

    match output {
        Ok(out) => {
            let name = dataset_name
                .filter(|s| !s.trim().is_empty())
                .unwrap_or_else(|| format!("GoogleSheets_{}", now_ts_secs()));
            set_loaded_df(out.df, name, "load_google_sheet_dataset")
        }
        Err(e) => ApiResult::failure(e.to_string()),
    }
}

#[tauri::command]
pub async fn load_http_api_dataset(
    url: String,
    auth_type: Option<String>,
    auth_value: Option<String>,
    dataset_name: Option<String>,
) -> ApiResult<ChartPayload> {
    let endpoint = url.trim();
    if endpoint.is_empty() {
        return ApiResult::failure("API URL 不能为空");
    }

    let mut headers = HeaderMap::new();
    let kind = auth_type.unwrap_or_else(|| "none".to_string()).to_lowercase();
    let token = auth_value.unwrap_or_default();

    if kind == "bearer" && !token.trim().is_empty() {
        let auth = format!("Bearer {}", token.trim());
        match HeaderValue::from_str(&auth) {
            Ok(v) => {
                headers.insert(AUTHORIZATION, v);
            }
            Err(e) => return ApiResult::failure(format!("无效 Bearer Token: {e}")),
        }
    } else if kind == "api_key" && !token.trim().is_empty() {
        match HeaderValue::from_str(token.trim()) {
            Ok(v) => {
                headers.insert("x-api-key", v);
            }
            Err(e) => return ApiResult::failure(format!("无效 API Key: {e}")),
        }
    }

    let client = reqwest::Client::new();
    let resp = match client.get(endpoint).headers(headers).send().await {
        Ok(r) => r,
        Err(e) => return ApiResult::failure(format!("请求 API 失败: {e}")),
    };

    if !resp.status().is_success() {
        return ApiResult::failure(format!("API 返回状态码: {}", resp.status()));
    }

    let content_type = resp
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_lowercase();

    let body = match resp.text().await {
        Ok(t) => t,
        Err(e) => return ApiResult::failure(format!("读取 API 响应失败: {e}")),
    };

    let trimmed = body.trim_start();

    let df = if content_type.contains("application/json")
        || trimmed.starts_with('{')
        || trimmed.starts_with('[')
    {
        match serde_json::from_str::<Value>(&body)
            .context("JSON 解析失败")
            .and_then(json_to_df)
        {
            Ok(df) => df,
            Err(e) => return ApiResult::failure(format!("API JSON 转表格失败: {e}")),
        }
    } else {
        let tmp = temp_csv_path("tauri_bi_api");
        if let Err(e) = fs::write(&tmp, body.as_bytes()) {
            return ApiResult::failure(format!("写入临时 CSV 失败: {e}"));
        }
        let output = load_file_impl(tmp.to_string_lossy().as_ref(), 0, 0, -1, false);
        let _ = fs::remove_file(&tmp);
        match output {
            Ok(out) => out.df,
            Err(e) => return ApiResult::failure(e.to_string()),
        }
    };

    let name = dataset_name
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| format!("API数据_{}", now_ts_secs()));
    set_loaded_df(df, name, "load_http_api_dataset")
}