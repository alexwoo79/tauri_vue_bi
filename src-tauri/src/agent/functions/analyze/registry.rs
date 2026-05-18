// src-tauri/src/agent/functions/analyze/registry.rs
//
// 分析模块注册表 - 对标 Python 的 Function/Analyze/registry.py
//
// 管理所有可用的分析模块，供 Agent 调用

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// 分析元数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisMetadata {
    pub analysis_id: String,
    pub name: String,
    pub desc: String,
    pub required_params: Vec<String>,
    pub optional_params: Vec<String>,
    pub output_tables: Vec<String>,
}

/// 分析注册表
pub struct AnalysisRegistry {
    analyses: HashMap<String, AnalysisMetadata>,
}

impl AnalysisRegistry {
    pub fn new() -> Self {
        let mut registry = Self {
            analyses: HashMap::new(),
        };
        registry.register_all();
        registry
    }

    fn register(&mut self, metadata: AnalysisMetadata) {
        self.analyses.insert(metadata.analysis_id.clone(), metadata);
    }

    fn register_all(&mut self) {
        // 十分位分析
        self.register(AnalysisMetadata {
            analysis_id: "Data_Decile_Analysis".to_string(),
            name: "十分位分析".to_string(),
            desc: "把数值型指标按大小分成 N 个等频桶，计算每个桶的 count/sum/mean/median/min/max/pct_of_total/cumulative_pct".to_string(),
            required_params: vec!["target_column".to_string()],
            optional_params: vec!["groupby_column".to_string(), "n_deciles".to_string()],
            output_tables: vec!["decile_result".to_string(), "breakdown_result".to_string()],
        });

        // 决策树分析
        self.register(AnalysisMetadata {
            analysis_id: "Decision_Tree".to_string(),
            name: "决策树分析".to_string(),
            desc: "使用决策树算法进行分类或回归分析，生成可解释的决策规则".to_string(),
            required_params: vec!["target_column".to_string(), "feature_columns".to_string()],
            optional_params: vec!["max_depth".to_string(), "min_samples_split".to_string()],
            output_tables: vec!["tree_rules".to_string(), "feature_importance".to_string()],
        });

        // K-Means 聚类分析
        self.register(AnalysisMetadata {
            analysis_id: "K_Means".to_string(),
            name: "K均值聚类".to_string(),
            desc: "将数据分成 K 个簇，每个簇内的数据点相似度较高".to_string(),
            required_params: vec!["feature_columns".to_string()],
            optional_params: vec!["n_clusters".to_string(), "max_iter".to_string()],
            output_tables: vec!["cluster_result".to_string(), "cluster_centers".to_string()],
        });
    }

    pub fn get(&self, analysis_id: &str) -> Option<&AnalysisMetadata> {
        self.analyses.get(analysis_id)
    }

    pub fn get_all(&self) -> Vec<&AnalysisMetadata> {
        self.analyses.values().collect()
    }

    pub fn build_agent_desc(&self) -> String {
        let mut lines = Vec::new();
        for analysis in self.get_all() {
            let req = analysis.required_params.join(", ");
            let opt = analysis.optional_params.join(", ");
            lines.push(format!(
                "  {} — {}\n    必填参数: {} │ 可选参数: {}",
                analysis.analysis_id,
                analysis.desc.chars().take(100).collect::<String>(),
                if req.is_empty() { "无" } else { &req },
                if opt.is_empty() { "无" } else { &opt }
            ));
        }
        lines.join("\n")
    }
}

lazy_static::lazy_static! {
    pub static ref ANALYSIS_REGISTRY: AnalysisRegistry = AnalysisRegistry::new();
}

pub fn get_analysis_registry() -> &'static AnalysisRegistry {
    &ANALYSIS_REGISTRY
}
