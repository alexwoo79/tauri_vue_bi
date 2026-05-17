# Rust Agent 快速启动指南

## 🚀 快速开始

### 1. 安装依赖

```bash
# 安装前端依赖
npm ci

# 确保 Rust 工具链已安装
rustup default 1.88.0
```

### 2. 启动开发服务器

```bash
# 方式 1: 使用 Makefile（推荐）
make dev

# 方式 2: 直接使用 npm
npm run tauri dev
```

### 3. 访问 Rust Agent

应用启动后：
1. 点击左侧菜单的 **"🤖 Rust Agent"** 
2. 或直接访问 `#/agent-chat`

---

## 💡 使用示例

### 示例 1: 创建会话并聊天

```typescript
// 在浏览器控制台中测试
import { useAgent } from '@/composables/useAgent'

const agent = useAgent()

// 创建会话
const sessionId = await agent.createSession('openai')
console.log('Session ID:', sessionId)

// 发送消息
await agent.chatStream(sessionId, '你好，请介绍一下自己', (event) => {
  console.log('收到事件:', event)
})
```

### 示例 2: 导出 Excel

```typescript
const { exportExcel } = useAgent()

const result = await exportExcel(['main_data'], 'sales_report.xlsx')
console.log('文件路径:', result.file_path)
```

### 示例 3: 生成 PPT

```typescript
const { generatePPT } = useAgent()

const result = await generatePPT('销售分析报告', 'mckinsey', 5)
console.log('PPT 路径:', result.file_path)
```

---

## 📁 项目结构

```
src/
├── composables/
│   └── useAgent.ts          # Agent Composable（新增）
├── components/
│   ├── FileExport.vue       # 文件导出组件（新增）
│   ├── AiMessageInput.vue   # 消息输入组件（已有）
│   └── BiChart.vue          # 图表组件（已有）
├── views/
│   └── AgentChat.vue        # Agent 聊天视图（新增）
├── router/
│   └── index.ts             # 路由配置（已更新）
├── utils/
│   └── aiTypes.ts           # AI 类型定义（已更新）
└── App.vue                  # 主应用（已更新）

src-tauri/
└── src/
    ├── agent/
    │   ├── state_machine.rs # Agent 状态机
    │   ├── session.rs       # 会话管理
    │   └── tools/
    │       ├── data_tools.rs    # 数据工具
    │       ├── chart_tools.rs   # 图表工具
    │       └── export_tools.rs  # 导出工具
    ├── llm/
    │   ├── client.rs        # LLM 客户端
    │   └── providers/       # 提供商实现
    └── commands/
        └── agent_chat.rs    # Tauri 命令
```

---

## 🔧 故障排除

### 问题 1: TypeScript 编译错误

**症状**: `npm run build` 失败

**解决方案**:
```bash
# 清理缓存
rm -rf node_modules/.vite
npm run build
```

### 问题 2: Rust 编译错误

**症状**: `cargo check` 失败

**解决方案**:
```bash
cd src-tauri
cargo clean
cargo check
```

### 问题 3: 事件监听器未清理

**症状**: 内存泄漏或重复事件

**解决方案**:
确保在组件卸载时调用 `cleanup()`:
```typescript
onUnmounted(() => {
  cleanup()
})
```

---

## 📊 功能清单

### ✅ 已完成

- [x] 会话管理（创建、删除、列出）
- [x] 流式聊天（SSE 事件）
- [x] 消息渲染（多种类型）
- [x] 图表生成（11 种类型）
- [x] Excel 导出
- [x] PPT 生成
- [x] 报告生成
- [x] Dashboard 生成
- [x] 文件路径复制
- [x] 三栏布局
- [x] 自动滚动
- [x] 错误处理

### 🚧 待完成

- [ ] 后端 Tauri 命令完整实现
- [ ] SSE 实时推送优化
- [ ] 会话历史加载
- [ ] 会话持久化
- [ ] 数据分析算法

---

## 🎯 下一步开发

### 本周任务

1. **实现后端 Tauri 命令**
   ```rust
   #[tauri::command]
   async fn export_excel(tables: Vec<String>, filename: Option<String>) -> Result<ExportResult>
   
   #[tauri::command]
   async fn generate_ppt(title: String, color_scheme: String, slide_count: u32) -> Result<ExportResult>
   ```

2. **优化 SSE 推送**
   - 使用 Tauri Event 系统实时推送
   - 避免收集后返回

3. **实现会话历史**
   - 从后端加载历史消息
   - 显示在聊天界面

### 下周任务

1. 前端测试
2. 性能优化
3. 文档完善
4. Bug 修复

---

## 📚 相关文档

- [FRONTEND_INTEGRATION_COMPLETE_20260516.md](./FRONTEND_INTEGRATION_COMPLETE_20260516.md) - 前端集成总结
- [CHART_TOOLS_GUIDE.md](./CHART_TOOLS_GUIDE.md) - 图表工具指南
- [EXCEL_EXPORT_GUIDE.md](./EXCEL_EXPORT_GUIDE.md) - Excel 导出指南
- [RUST_MIGRATION_PROGRESS.md](./RUST_MIGRATION_PROGRESS.md) - 迁移进度

---

**最后更新**: 2026-05-16  
**状态**: ✅ 前端集成完成，等待后端命令实现
