// src/router/index.ts
// Vue Router 路由配置 (Vue Router Configuration)

import { createRouter, createWebHashHistory } from 'vue-router'
import DataLoad from '../views/DataLoad.vue'
import DataClean from '../views/DataClean.vue'
import ChartAnalysis from '../views/ChartAnalysis.vue'
import PivotAnalysis from '../views/PivotAnalysis.vue'
import GanttAnalysis from '../views/GanttAnalysis.vue'
import MergeAnalysis from '../views/MergeAnalysis.vue'
import TimeAnalysis from '../views/TimeAnalysis.vue'
import AIAnalysis from '../views/AIAnalysis.vue'

const router = createRouter({
  // Tauri 使用 hash 模式路由（避免 file:// 协议下的 404 问题）
  history: createWebHashHistory(),
  routes: [
    {
      path: '/',
      redirect: '/data-load',
    },
    {
      path: '/load-clean',
      redirect: '/data-load',
    },
    {
      path: '/data-load',
      name: 'data-load',
      component: DataLoad,
      meta: { title: '⬇️ 数据加载' },
    },
    {
      path: '/data-clean',
      name: 'data-clean',
      component: DataClean,
      meta: { title: '🧹 数据清洗' },
    },
    {
      path: '/chart-analysis',
      name: 'chart-analysis',
      component: ChartAnalysis,
      meta: { title: '📊 图表分析' },
    },
    {
      path: '/pivot-analysis',
      name: 'pivot-analysis',
      component: PivotAnalysis,
      meta: { title: '🔢 多维透视分析' },
    },
    {
      path: '/gantt-analysis',
      name: 'gantt-analysis',
      component: GanttAnalysis,
      meta: { title: '📅 甘特图分析' },
    },
    {
      path: '/merge-analysis',
      name: 'merge-analysis',
      component: MergeAnalysis,
      meta: { title: '🔗 数据表合并' },
    },
    {
      path: '/time-analysis',
      name: 'time-analysis',
      component: TimeAnalysis,
      meta: { title: '📈 时间序列分析' },
    },
    {
      path: '/ai-analysis',
      name: 'ai-analysis',
      component: AIAnalysis,
      meta: { title: '✨ AI智能分析' },
    },
  ],
})

export default router
