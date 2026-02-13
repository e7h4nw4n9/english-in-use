import { invoke } from '@tauri-apps/api/core'
import type { AppConfig } from '../../types'

/**
 * 加载当前应用配置
 */
export async function loadConfig(): Promise<AppConfig> {
  return await invoke<AppConfig>('load_config')
}

/**
 * 保存应用配置
 * @param config 应用配置对象
 */
export async function saveConfig(config: AppConfig): Promise<void> {
  await invoke('save_config', { config })
}

/**
 * 导出配置到指定路径
 * @param path 目标文件路径
 * @param config 要导出的配置
 */
export async function exportConfig(path: string, config: AppConfig): Promise<void> {
  await invoke('export_config', { path, config })
}

/**
 * 从指定路径导入配置
 * @param path 配置文件路径
 */
export async function importConfig(path: string): Promise<AppConfig> {
  return await invoke<AppConfig>('import_config', { path })
}
