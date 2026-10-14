function describeClipboardError(error: unknown): string {
  if (typeof error === 'string' && error.trim()) return error

  if (typeof error === 'object' && error !== null) {
    if ('name' in error && error.name === 'NotAllowedError') {
      return '浏览器拒绝了剪贴板访问，请检查权限后重试。'
    }
    if ('message' in error && typeof error.message === 'string' && error.message.trim()) {
      return error.message
    }
  }

  return '未知错误'
}

function copyWithExecCommand(text: string): void {
  const doc = document
  if (typeof doc.execCommand !== 'function') {
    throw new Error('当前浏览器不支持 execCommand("copy")。')
  }
  if (!doc.body) {
    throw new Error('当前页面尚未准备好剪贴板复制。')
  }

  const view = doc.defaultView
  const activeElement = doc.activeElement as HTMLElement | null
  let selection: Selection | null = null
  let selectedRanges: Range[] = []
  try {
    const currentSelection = doc.getSelection()
    selection = currentSelection
    if (currentSelection) {
      selectedRanges = Array.from({ length: currentSelection.rangeCount }, (_, index) =>
        currentSelection.getRangeAt(index).cloneRange(),
      )
    }
  } catch {
    // 读取页面选区失败时仍继续复制。
    selection = null
    selectedRanges = []
  }

  const pageScrollX = view?.scrollX ?? doc.documentElement.scrollLeft
  const pageScrollY = view?.scrollY ?? doc.documentElement.scrollTop
  let controlSelection:
    { start: number; end: number; direction: 'forward' | 'backward' | 'none' | null } | undefined

  if (activeElement instanceof HTMLInputElement || activeElement instanceof HTMLTextAreaElement) {
    try {
      const start = activeElement.selectionStart
      const end = activeElement.selectionEnd
      if (start !== null && end !== null) {
        controlSelection = {
          start,
          end,
          direction: activeElement.selectionDirection,
        }
      }
    } catch {
      // 某些输入类型不支持读取文本选区。
    }
  }

  let temporaryTextarea: HTMLTextAreaElement | undefined
  try {
    temporaryTextarea = doc.createElement('textarea')
    temporaryTextarea.value = text
    temporaryTextarea.readOnly = true
    temporaryTextarea.setAttribute('aria-hidden', 'true')
    temporaryTextarea.tabIndex = -1
    temporaryTextarea.style.position = 'fixed'
    temporaryTextarea.style.top = '0'
    temporaryTextarea.style.left = '-9999px'
    temporaryTextarea.style.width = '1px'
    temporaryTextarea.style.height = '1px'
    temporaryTextarea.style.padding = '0'
    temporaryTextarea.style.border = '0'
    temporaryTextarea.style.fontSize = '16px'
    temporaryTextarea.style.opacity = '0'
    const appendTarget = activeElement?.closest('dialog, [role="dialog"]') ?? doc.body
    appendTarget.appendChild(temporaryTextarea)
    try {
      temporaryTextarea.focus({ preventScroll: true })
    } catch {
      temporaryTextarea.focus()
    }
    temporaryTextarea.select()

    if (doc.execCommand('copy') !== true) {
      throw new Error('execCommand("copy") 返回 false。')
    }
  } finally {
    temporaryTextarea?.remove()

    if (activeElement?.isConnected && typeof activeElement.focus === 'function') {
      try {
        activeElement.focus({ preventScroll: true })
      } catch {
        try {
          activeElement.focus()
        } catch {
          // 焦点恢复失败时保留复制结果。
        }
      }
    }

    if (
      controlSelection &&
      (activeElement instanceof HTMLInputElement || activeElement instanceof HTMLTextAreaElement)
    ) {
      try {
        activeElement.setSelectionRange(
          controlSelection.start,
          controlSelection.end,
          controlSelection.direction ?? undefined,
        )
      } catch {
        // 输入框状态变化后可能无法恢复原选区。
      }
    }

    if (selection && selectedRanges.length > 0) {
      try {
        selection.removeAllRanges()
        for (const range of selectedRanges) selection.addRange(range)
      } catch {
        // 原选区节点可能已从文档移除。
      }
    }

    try {
      view?.scrollTo(pageScrollX, pageScrollY)
    } catch {
      // 恢复滚动位置失败不影响复制结果。
    }
  }
}

export async function copyText(text: string): Promise<void> {
  if (typeof document === 'undefined') {
    throw new Error('当前环境不支持剪贴板复制（缺少 document）。')
  }

  let clipboardError: unknown = new Error('浏览器 Clipboard API 不可用。')
  try {
    const clipboard = typeof navigator === 'undefined' ? undefined : navigator.clipboard
    if (clipboard && typeof clipboard.writeText === 'function') {
      await clipboard.writeText(text)
      return
    }
  } catch (error) {
    clipboardError = error
  }

  try {
    copyWithExecCommand(text)
  } catch (fallbackError) {
    throw new Error(
      `复制失败：${describeClipboardError(clipboardError)}；兼容复制失败：${describeClipboardError(fallbackError)}`,
    )
  }
}
