// src/router/index.ts
// Vue Router 路由配置 (Vue Router Configuration)

import { createRouter, createWebHashHistory } from 'vue-router'
import LoadClean from '../views/LoadClean.vue'
import ChartAnalysis from '../views/ChartAnalysis.vue'
import PivotAnalysis from '../views/PivotAnalysis.vue'
import GanttAnalysis from '../views/GanttAnalysis.vue'
import MergeAnalysis from '../views/MergeAnalysis.vue'
import TimeAnalysis from '../views/TimeAnalysis.vue'

const router = createRouter({
  // Tauri 使用 hash 模式路由（避免 file:// 协议下的 404 问题）
  history: createWebHashHistory(),
  routes: [
    {
      path: '/',
      redirect: '/load-clean',
    },
    {
      path: '/load-clean',
      name: 'load-clean',
      component: LoadClean,
      meta: { title: '⬇️ 数据加载与清洗' },
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
  ],
})

export default router
