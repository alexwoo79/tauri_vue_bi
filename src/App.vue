<script setup lang="ts">
// src/App.vue
// 主布局 (Main Layout)
//
// 布局结构：
//   ┌────────┬────────────────────────────────┐
//   │        │  标题栏 (Header)                │
//   │ 侧边栏  ├────────────────────────────────┤
//   │ (Menu) │  路由内容区 (router-view)        │
//   │        │                                │
//   └────────┴────────────────────────────────┘
//
// 侧边栏菜单项对应四个主视图：
//   1. 数据加载与清洗  → /load-clean
//   2. 图表分析        → /chart-analysis
//   3. 多维透视分析    → /pivot-analysis
//   4. 甘特图分析      → /gantt-analysis

import { computed, ref, watch, onMounted, onUnmounted } from 'vue'
import { useRouter, useRoute } from 'vue-router'

const router = useRouter()
const route = useRoute()

// 当前激活的菜单项（与路由 name 对应）
const activeMenu = ref<string>((route.name as string) || 'load-clean')
const sidebarCollapsed = ref(false)

// 跟随系统暗色模式（可手动切换覆盖）
const systemQuery = window.matchMedia('(prefers-color-scheme: dark)')
const isDark = ref(systemQuery.matches)

const sidebarWidth = computed(() => (sidebarCollapsed.value ? '64px' : '220px'))

watch(
  () => route.name,
  (name) => {
    activeMenu.value = (name as string) || 'load-clean'
  }
)

function handleMenuSelect(key: string) {
  activeMenu.value = key
  router.push({ name: key })
}

function applyTheme(dark: boolean) {
  if (dark) {
    document.documentElement.classList.add('dark')
  } else {
    document.documentElement.classList.remove('dark')
  }
}

function toggleDark(dark: boolean) {
  applyTheme(dark)
}

function onSystemThemeChange(e: MediaQueryListEvent) {
  isDark.value = e.matches
  applyTheme(e.matches)
}

onMounted(() => {
  applyTheme(isDark.value)
  systemQuery.addEventListener('change', onSystemThemeChange)
})

onUnmounted(() => {
  systemQuery.removeEventListener('change', onSystemThemeChange)
})

function toggleSidebar() {
  sidebarCollapsed.value = !sidebarCollapsed.value
}
</script>

<template>
  <el-container class="app-container" style="height: 100vh;">
    <!-- 侧边栏 Sidebar -->
    <el-aside :width="sidebarWidth" class="sidebar" :class="{ 'sidebar-collapsed': sidebarCollapsed }">
      <div class="sidebar-logo">
        <span class="logo-icon">📊</span>
        <span v-show="!sidebarCollapsed" class="logo-text">BI 分析工具</span>
      </div>

      <el-menu
        :default-active="activeMenu"
        :collapse="sidebarCollapsed"
        :collapse-transition="false"
        background-color="var(--el-bg-color-overlay)"
        text-color="var(--el-text-color-primary)"
        active-text-color="var(--el-color-primary)"
        class="sidebar-menu"
        @select="handleMenuSelect"
      >
        <el-menu-item index="load-clean">
          <template #title>⬇️ 数据加载与清洗</template>
        </el-menu-item>

        <el-menu-item index="chart-analysis">
          <template #title>📊 图表分析</template>
        </el-menu-item>

        <el-menu-item index="pivot-analysis">
          <template #title>🔢 多维透视分析</template>
        </el-menu-item>

        <el-menu-item index="gantt-analysis">
          <template #title>📅 甘特图分析</template>
        </el-menu-item>
      </el-menu>
    </el-aside>

    <!-- 主内容区 Main content -->
    <el-container direction="vertical">
      <!-- 顶部标题栏 Header -->
      <el-header class="app-header" height="44px">
        <div class="header-main">
          <el-button class="collapse-btn" text @click="toggleSidebar">
            {{ sidebarCollapsed ? '›' : '‹' }}
          </el-button>
          <div class="header-title">
            {{ route.meta?.title ?? 'BI 分析工具' }}
          </div>
        </div>
        <div class="header-actions">
          <!-- 暗色/亮色模式切换 -->
          <el-switch
            v-model="isDark"
            active-text="🌙"
            inactive-text="☀️"
            @change="toggleDark"
          />
        </div>
      </el-header>

      <!-- 路由内容区 -->
      <el-main class="app-main">
        <router-view />
      </el-main>
    </el-container>
  </el-container>
</template>

<style scoped>
.app-container {
  background-color: var(--el-bg-color);
  color: var(--el-text-color-primary);
}

.sidebar {
  border-right: 1px solid var(--el-border-color);
  display: flex;
  flex-direction: column;
  transition: width 0.2s ease;
}

.sidebar-logo {
  height: 44px;
  display: flex;
  align-items: center;
  padding: 0 16px;
  gap: 8px;
  border-bottom: 1px solid var(--el-border-color);
  font-size: 16px;
  font-weight: bold;
  overflow: hidden;
}

.logo-icon {
  font-size: 24px;
  flex: 0 0 auto;
}

.logo-text {
  white-space: nowrap;
}

.sidebar-collapsed .sidebar-logo {
  justify-content: center;
  padding: 0;
}

.sidebar-menu {
  flex: 1;
  border-right: none;
}

.app-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  border-bottom: 1px solid var(--el-border-color);
  padding: 0 24px;
  background-color: var(--el-bg-color-overlay);
}

.header-main {
  display: flex;
  align-items: center;
  gap: 12px;
  min-width: 0;
}

.collapse-btn {
  font-size: 24px;
  padding: 4px 8px;
}

.header-title {
  font-size: 18px;
  font-weight: 600;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.app-main {
  overflow: hidden;
  padding: 16px;
}
</style>
