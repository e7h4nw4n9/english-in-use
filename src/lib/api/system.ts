import { invoke } from '@tauri-apps/api/core'

/**
 * 重启应用
 */
export async function restartApp(): Promise<void> {
  await invoke('restart')
}

/**
 * 示例 Greet 命令
 * @param name 姓名
 */
export async function greet(name: string): Promise<string> {
  return await invoke<string>('greet', { name })
}
