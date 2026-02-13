import { invoke } from '@tauri-apps/api/core'
import type { DatabaseConnection, ConnectionStatus, BookSource } from '../../types'

/**
 * 初始化数据库
 */
export async function initializeDatabase(): Promise<void> {
  await invoke('initialize_database')
}

/**
 * 获取默认的 SQLite 数据库路径
 */
export async function getDefaultSqlitePath(): Promise<string> {
  return await invoke<string>('get_default_sqlite_path')
}

/**
 * 测试数据库连接
 * @param connection 数据库连接配置
 */
export async function testDatabaseConnection(connection: DatabaseConnection): Promise<void> {
  await invoke('test_database_connection', { connection })
}

/**
 * 测试 Cloudflare R2 连接
 * @param source 图书源配置
 */
export async function testR2Connection(source: BookSource): Promise<string[]> {
  return await invoke<string[]>('test_r2_connection', { source })
}

/**
 * 检查当前配置的所有服务连接状态
 */
export async function checkConnectionStatus(): Promise<ConnectionStatus> {
  return await invoke<ConnectionStatus>('check_connection_status')
}
