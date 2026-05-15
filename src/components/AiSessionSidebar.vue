/**
 * AI 会话侧边栏组件
 * 显示会话列表、创建新会话、管理会话
 */

<template>
  <div class="session-sidebar">
    <!-- 新建对话按钮 -->
    <el-button type="primary" class="new-session-btn" @click="createNewSession" block>
      <el-icon><DocumentAdd /></el-icon>
      新建对话
    </el-button>

    <!-- 会话列表 -->
    <div class="session-list">
      <div
        v-for="sess in sessionList"
        :key="sess.sessionId"
        :class="['session-item', { active: sess.sessionId === currentSessionId }]"
        @click="switchToSession(sess.sessionId)"
      >
        <div class="session-item-main">
          <div class="session-title">{{ sess.title }}</div>
          <div class="session-meta">
            {{ formatTime(sess.updatedAt) }} • {{ sess.messages.length }} 条消息
          </div>
        </div>
        <div class="session-actions">
          <el-popover
            placement="right"
            :width="200"
            trigger="hover"
            :show-arrow="false"
          >
            <template #reference>
              <el-icon class="icon-btn"><MoreFilled /></el-icon>
            </template>
            <div class="action-menu">
              <div class="action-item" @click="renameSession(sess.sessionId)">
                <el-icon><Edit /></el-icon>
                重命名
              </div>
              <el-divider style="margin: 4px 0" />
              <div class="action-item danger" @click="deleteSession(sess.sessionId)">
                <el-icon><Delete /></el-icon>
                删除
              </div>
            </div>
          </el-popover>
        </div>
      </div>

      <!-- 空状态 -->
      <div v-if="sessionList.length === 0" class="session-empty">
        <el-icon><DocumentCopy /></el-icon>
        <div>暂无对话</div>
        <div style="font-size: 12px; color: #909399; margin-top: 4px">
          点击"新建对话"开始
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import { ElMessage, ElMessageBox } from 'element-plus'
import { Edit, Delete, MoreFilled, DocumentAdd, DocumentCopy } from '@element-plus/icons-vue'
import { useSessionStore } from '../stores/sessionStore'

const sessionStore = useSessionStore()

const sessionList = computed(() => sessionStore.sessionList)
const currentSessionId = computed(() => sessionStore.currentSessionId)

// 创建新会话
function createNewSession() {
  const session = sessionStore.createSession()
  ElMessage.success(`新建对话: ${session.title}`)
}

// 切换会话
function switchToSession(sessionId: string) {
  sessionStore.switchSession(sessionId)
}

// 重命名会话
async function renameSession(sessionId: string) {
  const session = sessionStore.sessions.find((s) => s.sessionId === sessionId)
  if (!session) return

  const { value } = await ElMessageBox.prompt('输入新的对话名称', '重命名', {
    inputValue: session.title,
    confirmButtonText: '确定',
    cancelButtonText: '取消',
  }).catch(() => ({ value: null }))

  if (value) {
    sessionStore.renameSession(sessionId, value)
    ElMessage.success('已重命名')
  }
}

// 删除会话
async function deleteSession(sessionId: string) {
  const session = sessionStore.sessions.find((s) => s.sessionId === sessionId)
  if (!session) return

  await ElMessageBox.confirm(
    `确定要删除对话"${session.title}"吗?`,
    '删除对话',
    { confirmButtonText: '删除', cancelButtonText: '取消', type: 'warning' }
  ).catch(() => {})
    .then(() => {
      sessionStore.deleteSession(sessionId)
      ElMessage.success('已删除')
    })
    .catch(() => {})
}

// 格式化时间
function formatTime(timestamp: number): string {
  const now = Date.now()
  const diff = now - timestamp
  const seconds = Math.floor(diff / 1000)
  const minutes = Math.floor(seconds / 60)
  const hours = Math.floor(minutes / 60)
  const days = Math.floor(hours / 24)

  if (minutes < 1) return '刚刚'
  if (minutes < 60) return `${minutes}分钟前`
  if (hours < 24) return `${hours}小时前`
  if (days < 7) return `${days}天前`

  const date = new Date(timestamp)
  return date.toLocaleDateString('zh-CN')
}
</script>

<style scoped>
.session-sidebar {
  display: flex;
  flex-direction: column;
  gap: 12px;
  height: auto;
}

.new-session-btn {
  width: 100%;
  height: 36px;
}

.session-list {
  flex: 0 0 auto;
  overflow: visible;
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.session-empty {
  min-height: 72px;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  color: #909399;
  font-size: 12px;
}

.session-empty :deep(.el-icon) {
  font-size: 32px;
  margin-bottom: 8px;
  opacity: 0.3;
}

.session-item {
  padding: 8px 12px;
  border-radius: 6px;
  cursor: pointer;
  display: flex;
  justify-content: space-between;
  align-items: center;
  transition: all 0.2s ease;
  background-color: var(--el-fill-color-light);
  border: 1px solid transparent;

  &:hover {
    background-color: var(--el-fill-color);
    border-color: var(--el-border-color-darker);
  }

  &.active {
    background-color: var(--el-color-primary-light-9);
    border-color: var(--el-color-primary);
  }
}

.session-item-main {
  flex: 1;
  min-width: 0;
}

.session-title {
  font-size: 13px;
  font-weight: 500;
  color: var(--el-text-color-primary);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.session-meta {
  font-size: 11px;
  color: var(--el-text-color-regular);
  margin-top: 2px;
}

.session-actions {
  display: flex;
  gap: 4px;
}

.icon-btn {
  cursor: pointer;
  color: var(--el-text-color-regular);
  transition: color 0.2s;

  &:hover {
    color: var(--el-text-color-primary);
  }
}

.action-menu {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.action-item {
  padding: 6px 8px;
  border-radius: 4px;
  cursor: pointer;
  display: flex;
  align-items: center;
  gap: 6px;
  font-size: 12px;
  transition: background-color 0.2s;

  &:hover {
    background-color: var(--el-fill-color);
  }

  &.danger {
    color: var(--el-color-danger);
  }
}

::-webkit-scrollbar {
  width: 6px;
}

::-webkit-scrollbar-track {
  background: transparent;
}

::-webkit-scrollbar-thumb {
  background: var(--el-border-color);
  border-radius: 3px;

  &:hover {
    background: var(--el-border-color-darker);
  }
}
</style>
