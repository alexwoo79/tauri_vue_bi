// src/main.ts
// 全局挂载入口 (Global Mount Entry)
//
// 挂载顺序：
//   1. 引入 Element Plus 组件库 + 暗黑主题 CSS
//   2. 引入 ECharts 全量注册（通过 vue-echarts）
//   3. 创建 Vue 应用、Pinia 状态管理、Vue Router
//   4. 挂载到 #app

import './assets/main.css'

import { createApp } from 'vue'
import { createPinia } from 'pinia'

// ── Element Plus ─────────────────────────────────────────────────────────────
import ElementPlus from 'element-plus'
// 引入 Element Plus 默认样式
import 'element-plus/dist/index.css'
// 引入 Element Plus 暗黑模式 CSS 变量（配合 html.dark 类名切换）
import 'element-plus/theme-chalk/dark/css-vars.css'
// 引入中文语言包
import zhCn from 'element-plus/es/locale/lang/zh-cn'

// ── ECharts / vue-echarts ─────────────────────────────────────────────────────
import VChart from 'vue-echarts'
import { use } from 'echarts/core'
import { CanvasRenderer } from 'echarts/renderers'
import {
  BarChart,
  LineChart,
  ScatterChart,
  PieChart,
  HeatmapChart,
  BoxplotChart,
  CustomChart,
} from 'echarts/charts'
import {
  GridComponent,
  TooltipComponent,
  LegendComponent,
  TitleComponent,
  DataZoomComponent,
  VisualMapComponent,
  DatasetComponent,
  TransformComponent,
  ToolboxComponent,
} from 'echarts/components'

// 注册必须的 ECharts 模块（按需注册，减小打包体积）
use([
  CanvasRenderer,
  BarChart,
  LineChart,
  ScatterChart,
  PieChart,
  HeatmapChart,
  BoxplotChart,
  CustomChart,
  GridComponent,
  TooltipComponent,
  LegendComponent,
  TitleComponent,
  DataZoomComponent,
  VisualMapComponent,
  DatasetComponent,
  TransformComponent,
  ToolboxComponent,
])

// ── App ───────────────────────────────────────────────────────────────────────
import App from './App.vue'
import router from './router'

const app = createApp(App)

app.use(createPinia())
app.use(router)
app.use(ElementPlus, { locale: zhCn, size: 'default' })

// 注册全局图表组件
app.component('VChart', VChart)

// 默认启用暗黑模式（在 <html> 元素上添加 dark 类）
document.documentElement.classList.add('dark')

app.mount('#app')
