// src-tauri/src/agent/functions/base.rs
//
// 图表基础类型定义
// 严格对标 Python Data-Analysis-Agent 的 charts/base.py

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// 统一返回结构
/// 所有图表 generate() 必须返回此结构
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChartResult {
    pub html: String,
    pub spec: serde_json::Value,
    pub warnings: Vec<String>,
    pub meta: serde_json::Value,
}

impl ChartResult {
    pub fn success(html: String, meta: serde_json::Value) -> Self {
        Self {
            html,
            spec: serde_json::Value::Object(serde_json::Map::new()),
            warnings: Vec::new(),
            meta,
        }
    }

    pub fn error(warnings: Vec<String>) -> Self {
        Self {
            html: String::new(),
            spec: serde_json::Value::Object(serde_json::Map::new()),
            warnings,
            meta: serde_json::Value::Object(serde_json::Map::new()),
        }
    }

    pub fn is_valid(&self) -> bool {
        !self.html.trim().is_empty() && self.html.len() > 500
    }
}

/// 字段映射标准名称
/// 图表使用 mapping.xxx 读取实际列名
/// 严格对标 Python 的 FieldMapping
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldMapping {
    // 基础字段
    pub label: Option<String>,
    pub value: Option<String>,
    pub x: Option<String>,
    pub y: Option<String>,
    pub series: Option<String>,
    pub size: Option<String>,
    pub color: Option<String>,
    pub group: Option<String>,
    
    // 关系/流向字段
    pub source: Option<String>,
    pub target: Option<String>,
    
    // 地理字段
    pub lat: Option<String>,
    pub lon: Option<String>,
    pub geo: Option<String>,
    
    // 时间字段
    pub date: Option<String>,
    pub time: Option<String>,
    
    // 层级/树形字段
    pub parent: Option<String>,
    pub child: Option<String>,
    pub parents: Option<String>,  // 用于 treemap/sunburst
    pub labels: Option<String>,   // 用于 treemap/sunburst
    pub values: Option<String>,   // 用于 treemap/sunburst
    
    // 金融/股票字段
    pub open: Option<String>,
    pub high: Option<String>,
    pub low: Option<String>,
    pub close: Option<String>,
    pub volume: Option<String>,
    
    // 其他字段
    pub path: Option<String>,
    pub text: Option<String>,
    pub frequency: Option<String>,
    pub rank: Option<String>,
    pub actual: Option<String>,
    pub order: Option<String>,
    pub dimensions: Option<Vec<String>>,
}

impl FieldMapping {
    pub fn to_dict(&self) -> HashMap<String, String> {
        let mut result = HashMap::new();
        
        if let Some(v) = &self.label { result.insert("label".to_string(), v.clone()); }
        if let Some(v) = &self.value { result.insert("value".to_string(), v.clone()); }
        if let Some(v) = &self.x { result.insert("x".to_string(), v.clone()); }
        if let Some(v) = &self.y { result.insert("y".to_string(), v.clone()); }
        if let Some(v) = &self.series { result.insert("series".to_string(), v.clone()); }
        if let Some(v) = &self.size { result.insert("size".to_string(), v.clone()); }
        if let Some(v) = &self.color { result.insert("color".to_string(), v.clone()); }
        if let Some(v) = &self.group { result.insert("group".to_string(), v.clone()); }
        if let Some(v) = &self.source { result.insert("source".to_string(), v.clone()); }
        if let Some(v) = &self.target { result.insert("target".to_string(), v.clone()); }
        if let Some(v) = &self.lat { result.insert("lat".to_string(), v.clone()); }
        if let Some(v) = &self.lon { result.insert("lon".to_string(), v.clone()); }
        if let Some(v) = &self.geo { result.insert("geo".to_string(), v.clone()); }
        if let Some(v) = &self.date { result.insert("date".to_string(), v.clone()); }
        if let Some(v) = &self.time { result.insert("time".to_string(), v.clone()); }
        if let Some(v) = &self.parent { result.insert("parent".to_string(), v.clone()); }
        if let Some(v) = &self.child { result.insert("child".to_string(), v.clone()); }
        if let Some(v) = &self.parents { result.insert("parents".to_string(), v.clone()); }
        if let Some(v) = &self.labels { result.insert("labels".to_string(), v.clone()); }
        if let Some(v) = &self.values { result.insert("values".to_string(), v.clone()); }
        if let Some(v) = &self.open { result.insert("open".to_string(), v.clone()); }
        if let Some(v) = &self.high { result.insert("high".to_string(), v.clone()); }
        if let Some(v) = &self.low { result.insert("low".to_string(), v.clone()); }
        if let Some(v) = &self.close { result.insert("close".to_string(), v.clone()); }
        if let Some(v) = &self.volume { result.insert("volume".to_string(), v.clone()); }
        if let Some(v) = &self.path { result.insert("path".to_string(), v.clone()); }
        if let Some(v) = &self.text { result.insert("text".to_string(), v.clone()); }
        if let Some(v) = &self.frequency { result.insert("frequency".to_string(), v.clone()); }
        if let Some(v) = &self.rank { result.insert("rank".to_string(), v.clone()); }
        if let Some(v) = &self.actual { result.insert("actual".to_string(), v.clone()); }
        if let Some(v) = &self.order { result.insert("order".to_string(), v.clone()); }
        
        result
    }

    pub fn from_dict(dict: &HashMap<String, String>) -> Self {
        Self {
            label: dict.get("label").cloned(),
            value: dict.get("value").cloned(),
            x: dict.get("x").cloned(),
            y: dict.get("y").cloned(),
            series: dict.get("series").cloned(),
            size: dict.get("size").cloned(),
            color: dict.get("color").cloned(),
            group: dict.get("group").cloned(),
            source: dict.get("source").cloned(),
            target: dict.get("target").cloned(),
            lat: dict.get("lat").cloned(),
            lon: dict.get("lon").cloned(),
            geo: dict.get("geo").cloned(),
            date: dict.get("date").cloned(),
            time: dict.get("time").cloned(),
            parent: dict.get("parent").cloned(),
            child: dict.get("child").cloned(),
            parents: dict.get("parents").cloned(),
            labels: dict.get("labels").cloned(),
            values: dict.get("values").cloned(),
            open: dict.get("open").cloned(),
            high: dict.get("high").cloned(),
            low: dict.get("low").cloned(),
            close: dict.get("close").cloned(),
            volume: dict.get("volume").cloned(),
            path: dict.get("path").cloned(),
            text: dict.get("text").cloned(),
            frequency: dict.get("frequency").cloned(),
            rank: dict.get("rank").cloned(),
            actual: dict.get("actual").cloned(),   
            order: dict.get("order").cloned(),
            dimensions: None,
        }
    }
}

/// 图表选项
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ChartOptions {
    pub title: Option<String>,
    pub color_scheme: Option<String>,
    pub orientation: Option<String>,
    pub sort: Option<bool>,
    pub top_n: Option<usize>,
    pub width: Option<u32>,
    pub height: Option<u32>,
}

// 为 FieldMapping 添加 Default 实现
impl Default for FieldMapping {
    fn default() -> Self {
        Self {
            label: None,
            value: None,
            x: None,
            y: None,
            series: None,
            size: None,
            color: None,
            group: None,
            source: None,
            target: None,
            lat: None,
            lon: None,
            geo: None,
            date: None,
            time: None,
            parent: None,
            child: None,
            parents: None,
            labels: None,
            values: None,
            open: None,
            high: None,
            low: None,
            close: None,
            volume: None,
            path: None,
            text: None,
            frequency: None,
            rank: None,
            actual: None,
            order: None,
            dimensions: None,
        }
    }
}