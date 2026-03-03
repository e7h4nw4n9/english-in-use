import { invoke } from '@tauri-apps/api/core'

/**
 * 重启应用
 */
export async function restartApp(): Promise<void> {
  await invoke('restart')
}

/**
 * 获取当前运行平台类型 (windows, macos, android, ios, linux)
 */
export async function getPlatform(): Promise<string> {
  return await invoke<string>('get_platform')
}

/**
 * 示例 Greet 命令
 * @param name 姓名
 */
export async function greet(name: string): Promise<string> {
  return await invoke<string>('greet', { name })
}
