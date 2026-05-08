// src/composables/useResize.ts
//
// 侧边栏拖拽宽度调整 Composable
//
// 封装了 4 个分析页面共用的「鼠标拖拽改变侧边栏宽度」逻辑。
// 用法：
//   const { configWidth, startResize } = useResize()            // 默认 min=320, max=600
//   const { configWidth, startResize } = useResize(320, 640)    // 自定义上下限

import { ref } from 'vue'

export function useResize(minWidth = 320, maxWidth = 600) {
  const configWidth = ref(minWidth)

  function startResize(e: MouseEvent) {
    const startX = e.clientX
    const startWidth = configWidth.value

    function onMove(ev: MouseEvent) {
      configWidth.value = Math.max(minWidth, Math.min(maxWidth, startWidth + ev.clientX - startX))
    }
    function onUp() {
      document.removeEventListener('mousemove', onMove)
      document.removeEventListener('mouseup', onUp)
    }

    document.addEventListener('mousemove', onMove)
    document.addEventListener('mouseup', onUp)
  }

  return { configWidth, startResize }
}
