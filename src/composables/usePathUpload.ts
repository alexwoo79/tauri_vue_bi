import { ref } from 'vue'
import { open as openDialog } from '@tauri-apps/plugin-dialog'
import { ElMessageBox } from 'element-plus'

interface UsePathUploadOptions {
  fileExtensions?: string[]
}

export function usePathUpload(options?: UsePathUploadOptions) {
  const dragOver = ref(false)
  const extensions = options?.fileExtensions ?? ['csv', 'xlsx', 'xls', 'xlsm', 'ods']

  function dedupePaths(paths: string[]): string[] {
    return Array.from(new Set(paths.map((p) => p.trim()).filter(Boolean)))
  }

  function normalizeDroppedPath(raw: string): string {
    const trimmed = raw.trim()
    if (!trimmed) return ''
    if (!trimmed.startsWith('file://')) return trimmed
    try {
      const url = new URL(trimmed)
      return decodeURIComponent(url.pathname)
    } catch {
      return ''
    }
  }

  function parseUriListToPaths(raw: string): string[] {
    if (!raw) return []
    return raw
      .split('\n')
      .map((l) => l.trim())
      .filter((l) => l && !l.startsWith('#'))
      .map((u) => normalizeDroppedPath(u))
      .filter(Boolean)
  }

  function extractDroppedPaths(e: DragEvent): string[] {
    const dt = e.dataTransfer
    if (!dt) return []

    const picked: string[] = []
    const files = Array.from(dt.files ?? [])
    for (const f of files) {
      const abs = (f as File & { path?: string })?.path
      if (abs) picked.push(abs)
    }

    const uriList = dt.getData('text/uri-list')
    picked.push(...parseUriListToPaths(uriList))

    const plain = dt.getData('text/plain')
    if (plain) {
      const p = normalizeDroppedPath(plain)
      if (p) picked.push(p)
    }

    return dedupePaths(picked)
  }

  async function chooseFiles() {
    const selected = await openDialog({
      multiple: true,
      directory: false,
      filters: [{ name: 'Data Files', extensions }],
    })
    if (!selected) return []
    return dedupePaths(Array.isArray(selected) ? selected : [selected])
  }

  async function chooseFolder() {
    const selected = await openDialog({
      multiple: false,
      directory: true,
    })
    if (!selected || Array.isArray(selected)) return []
    return [selected]
  }

  async function pickByClick() {
    try {
      await ElMessageBox.confirm('请选择上传方式', '导入路径', {
        confirmButtonText: '选择文件(多选)',
        cancelButtonText: '选择文件夹',
        distinguishCancelAndClose: true,
        type: 'info',
      })
      return await chooseFiles()
    } catch (action) {
      if (action === 'cancel') {
        return await chooseFolder()
      }
      return []
    }
  }

  function onDragEnter() {
    dragOver.value = true
  }

  function onDragLeave() {
    dragOver.value = false
  }

  function onDrop(e: DragEvent) {
    e.preventDefault()
    dragOver.value = false
    return extractDroppedPaths(e)
  }

  return {
    dragOver,
    dedupePaths,
    extractDroppedPaths,
    pickByClick,
    chooseFiles,
    chooseFolder,
    onDragEnter,
    onDragLeave,
    onDrop,
  }
}
