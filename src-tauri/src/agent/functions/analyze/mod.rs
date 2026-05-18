// src-tauri/src/agent/functions/analyze/mod.rs
//
// 分析模块 - 对标 Python Data-Analysis-Agent 的 Function/Analyze/
//
// 包含：
// - registry.rs    : 分析模块注册表
// - decile.rs      : 十分位分析
// - decision_tree.rs: 决策树分析
// - kmeans.rs      : K均值聚类分析

pub mod registry;
pub mod decile;
pub mod decision_tree;
pub mod kmeans;

pub use registry::*;
pub use decile::*;
pub use decision_tree::*;
pub use kmeans::*;
