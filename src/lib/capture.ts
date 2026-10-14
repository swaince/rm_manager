import { invoke } from '@tauri-apps/api/core'
import { emit } from '@tauri-apps/api/event'
import { readText } from '@tauri-apps/plugin-clipboard-manager'
import { isRegistered, register } from '@tauri-apps/plugin-global-shortcut'
import { harvestClipboard } from '@/apis/items'

/** 条目变更通知：速记窗提交 / 剪贴板收割后广播，列表与整理台据此重新查询 */
export const ITEMS_CHANGED = 'items-changed'

/** 迷你速记窗（tauri.conf.json 预置的隐藏窗口）：显示与聚焦都走 Rust 命令，避免额外窗口权限 */
export function openCaptureWindow(): Promise<void> {
  return invoke('open_capture_window')
}

/** 收起速记窗（Esc / 取消） */
export function hideCaptureWindow(): Promise<void> {
  return invoke('hide_capture_window')
}

/** 把焦点交还主窗（速记提交后） */
export function focusMainWindow(): Promise<void> {
  return invoke('focus_main_window')
}

/** 剪贴板收割：整段文本存为 inbox 条目；剪贴板没有文本时不动（核心不允许空标题） */
async function harvest(): Promise<void> {
  const text = await readText()
  if (!text?.trim()) return
  await harvestClipboard(text)
  await emit(ITEMS_CHANGED)
}

/** 开发时热更新会重复执行注册，已注册的快捷键直接沿用 */
async function bind(shortcut: string, handler: () => void): Promise<void> {
  if (await isRegistered(shortcut)) return
  // 一次按键会收到 Pressed / Released 两个事件，只认按下，避免重复触发
  await register(shortcut, (event) => {
    if (event.state === 'Pressed') handler()
  })
}

/** 主窗注册全局快捷键：失焦时同样可用（快捷键注册在 Rust 侧，与窗口焦点无关） */
export async function registerCaptureShortcuts(): Promise<void> {
  await bind('CommandOrControl+Shift+Space', () => void openCaptureWindow())
  await bind('CommandOrControl+Shift+V', () => void harvest())
}
