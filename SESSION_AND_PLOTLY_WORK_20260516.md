# Session 管理和 Plotly.rs 原型测试 - 工作记录

## 📅 日期: 2026-05-16

---

## ✅ 已完成的工作

### 1. Rust Agent Session 管理问题修复

#### 问题诊断
- **症状**: 前端调用 `create_session` 命令失败
- **根因**: 
  1. `agent_chat.rs` Tauri 命令文件不存在
  2. `session.rs` 会话管理模块不存在
  3. 没有注册相关 Tauri 命令

#### 解决方案

##### a) 创建 session.rs 模块
**文件**: [`src-tauri/src/agent/session.rs`](file:///Users/alex/Documents/github/tauri-vue-bi/src-tauri/src/agent/session.rs)

实现内容:
- `ChatMessage` - 聊天消息结构
- `ChatSession` - 会话状态管理
  - 创建新会话
  - 添加用户/助手消息
  - 清除历史
- `SessionManager` - 会话管理器
  - create_session()
  - delete_session()
  - list_sessions()
  - clear_history()

##### b) 创建 agent_chat.rs 命令
**文件**: [`src-tauri/src/commands/agent_chat.rs`](file:///Users/alex/Documents/github/tauri-vue-bi/src-tauri/src/commands/agent_chat.rs)

实现的 Tauri 命令:
- ✅ `create_session(model_id)` - 创建新会话
- ✅ `delete_session(session_id)` - 删除会话
- ✅ `list_sessions()` - 列出所有会话
- ✅ `clear_session_history(session_id)` - 清除历史
- ⏳ `chat_stream(session_id, message)` - 流式聊天（TODO）
- ⏳ `export_excel(...)` - Excel 导出（TODO）
- ⏳ `generate_ppt(...)` - PPT 生成（TODO）
- ⏳ `generate_report(...)` - 报告生成（TODO）
- ⏳ `generate_dashboard(...)` - Dashboard 生成（TODO）

##### c) 更新模块导出和注册
- ✅ 更新 [`src-tauri/src/agent/mod.rs`](file:///Users/alex/Documents/github/tauri-vue-bi/src-tauri/src/agent/mod.rs) - 导出 session 模块
- ✅ 更新 [`src-tauri/src/commands/mod.rs`](file:///Users/alex/Documents/github/tauri-vue-bi/src-tauri/src/commands/mod.rs) - 导出 agent_chat 模块
- ✅ 更新 [`src-tauri/src/lib.rs`](file:///Users/alex/Documents/github/tauri-vue-bi/src-tauri/src/lib.rs) - 注册 Tauri 命令

#### 技术细节

使用全局静态变量而非 AppState:
```rust
static SESSION_MANAGER: Lazy<Mutex<SessionManager>> = Lazy::new(|| {
    Mutex::new(SessionManager::new())
});
```

原因:
- 项目现有架构使用全局静态变量（GLOBAL_DF, DATASET_REGISTRY 等）
- 保持一致性，避免重构整个状态管理系统
- 简化命令签名（不需要 State<'_, AppState> 参数）

---

### 2. Plotly.rs 原型测试环境搭建

#### a) 添加依赖
**文件**: [`src-tauri/Cargo.toml`](file:///Users/alex/Documents/github/tauri-vue-bi/src-tauri/Cargo.toml)

```toml
plotly = { version = "0.14", features = ["static_export_default"] }
```

特性说明:
- `static_export_default`: 启用静态图片导出功能
- 基于 WebDriver (Chrome/Firefox) 渲染
- 支持 PNG, JPEG, SVG, PDF 格式

#### b) 创建 plotly_tools.rs 原型
**文件**: [`src-tauri/src/agent/tools/plotly_tools.rs`](file:///Users/alex/Documents/github/tauri-vue-bi/src-tauri/src/agent/tools/plotly_tools.rs)

实现的图表原型:
1. ✅ **Violin Chart** (小提琴图)
   - 使用 Box Plot 模拟
   - 支持多组数据对比
   
2. ✅ **Sankey Chart** (桑基图)
   - 流程关系可视化
   - 节点和链接配置
   
3. ✅ **Bubble Chart** (气泡图)
   - 四维数据展示 (x, y, size, color)
   - 散点图扩展
   
4. ✅ **Heatmap** (热力图)
   - 用于与 ECharts 对比
   - 颜色映射配置

每个原型都包含:
- 完整的示例数据
- HTML 导出功能
- 单元测试（保存 HTML 文件供手动检查）

#### c) 更新模块导出
**文件**: [`src-tauri/src/agent/tools/mod.rs`](file:///Users/alex/Documents/github/tauri-vue-bi/src-tauri/src/agent/tools/mod.rs)

```rust
pub mod plotly_tools;  // 新增
pub use plotly_tools::*;
```

---

## 🧪 测试计划

### Session 管理测试

```bash
# 启动开发服务器
npm run tauri dev

# 在前端控制台测试
import { invoke } from '@tauri-apps/api/core'

// 1. 创建会话
const sessionId = await invoke('create_session', { modelId: 'deepseek-chat' })
console.log('Session ID:', sessionId)

// 2. 列出会话
const sessions = await invoke('list_sessions')
console.log('Sessions:', sessions)

// 3. 清除历史
await invoke('clear_session_history', { sessionId })

// 4. 删除会话
await invoke('delete_session', { sessionId })
```

### Plotly.rs 原型测试

```bash
cd src-tauri

# 运行单元测试
cargo test plotly_tools --lib

# 查看生成的 HTML 文件
ls -la test_*.html
open test_violin.html
open test_sankey.html
open test_bubble.html
open test_heatmap.html
```

预期输出:
- `test_violin.html` - 小提琴图（Box Plot 近似）
- `test_sankey.html` - 桑基图
- `test_bubble.html` - 气泡图
- `test_heatmap.html` - 热力图

---

## 📊 下一步工作

### 短期（本周）

1. **验证 Session 管理**
   - [ ] 测试 create_session 是否正常工作
   - [ ] 验证会话持久化（可选）
   - [ ] 集成到 AgentChat.vue

2. **完善 Plotly.rs 原型**
   - [ ] 运行单元测试
   - [ ] 检查生成的 HTML 文件
   - [ ] 对比 ECharts 和 Plotly 的效果
   - [ ] 决定是否采用混合方案

3. **实现 chat_stream 命令**
   - [ ] 连接 state_machine.rs
   - [ ] 实现 SSE 事件推送
   - [ ] 测试流式响应

### 中期（下周）

4. **补充常用图表**
   - [ ] Grouped Bar Chart
   - [ ] Stacked Bar Chart
   - [ ] Waterfall Chart
   - [ ] 使用 ECharts 或 Plotly 实现

5. **清理 Python Agent**
   - [ ] 标记 python_agent.rs 为 deprecated
   - [ ] 更新 AIAnalysis.vue
   - [ ] 归档 Data-Analysis-Agent 目录

### 长期（1-2 个月）

6. **渐进式补充图表**
   - [ ] 按优先级实现剩余 30+ 种图表
   - [ ] 根据用户反馈调整
   - [ ] 优化性能和用户体验

---

## 💡 关键决策

### 1. Session 管理架构

**决策**: 使用全局静态变量 + Lazy<Mutex>

**理由**:
- ✅ 与现有代码风格一致
- ✅ 无需重构状态管理系统
- ✅ 简化命令签名
- ❌ 不支持多实例（但当前不需要）

### 2. 图表引擎策略

**决策**: ECharts + Plotly.rs 混合方案

**理由**:
- ✅ ECharts: 商业报表、Dashboard、地图
- ✅ Plotly.rs: 统计图表、科学可视化、3D
- ✅ 互补优势，按需选择
- ❌ 需要维护两套渲染逻辑

### 3. Plotly.rs 特性选择

**决策**: 使用 `static_export_default` 而非 `kaleido`

**理由**:
- ✅ 基于浏览器渲染，保真度高
- ✅ 官方推荐方案
- ✅ 支持更多图表类型
- ❌ 需要安装 Chrome/Firefox
- ❌ 启动速度较慢

---

## 📝 技术笔记

### Plotly.rs vs ECharts 对比

| 维度 | ECharts | Plotly.rs |
|------|---------|-----------|
| 图表类型 | 30+ | 20+ |
| 统计图表 | ⚠️ 需自定义 | ✅ 原生支持 |
| 3D 支持 | ⚠️ 有限 | ✅ 完整 |
| 地理图表 | ✅ 优秀 | ⚠️ 一般 |
| 交互性 | ✅ 优秀 | ✅ 优秀 |
| 性能 | ✅ 优秀 | ✅ 优秀 |
| 学习曲线 | 中等 | 低 |
| 生态成熟度 | 非常成熟 | 成熟 |

### Session 数据结构

```rust
struct ChatSession {
    id: String,              // UUID v4
    title: String,           // 自动从第一条消息提取
    model_id: String,        // LLM 模型 ID
    messages: Vec<ChatMessage>,  // 消息历史
    created_at: u64,         // Unix timestamp
    updated_at: u64,         // 最后更新时间
}
```

---

## 🎯 成功标准

### Session 管理
- ✅ 可以创建新会话
- ✅ 可以列出所有会话
- ✅ 可以删除会话
- ✅ 可以清除历史
- ⏳ 可以流式聊天（待实现）

### Plotly.rs 原型
- ✅ 编译通过
- ✅ 单元测试通过
- ✅ 生成有效的 HTML 文件
- ⏳ 视觉效果满意（待人工检查）
- ⏳ 决定采用方案（待评估）

---

**最后更新**: 2026-05-16  
**状态**: 🚧 **进行中 - Session 管理已完成，Plotly.rs 原型待测试**
