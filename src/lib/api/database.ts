import { invoke } from '@tauri-apps/api/core'
import type { DatabaseConnection, ConnectionStatus, BookSource } from '../../types'

/**
 * 初始化数据库
 * @returns 是否执行了新的迁移（即是否为首次初始化）
 */
export async function initializeDatabase(): Promise<boolean> {
  return await invoke<boolean>('initialize_database')
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

/**
 * 获取所有可用的迁移版本
 */
export async function getMigrationVersions(): Promise<string[]> {
  return await invoke<string[]>('get_migration_versions')
}

/**
 * 获取当前数据库版本
 */
export async function getCurrentDbVersion(): Promise<string> {
  return await invoke<string>('get_current_db_version')
}

/**
 * 执行数据库升级迁移
 * @param targetVersion 目标版本，如果不传则升级到最新
 */
export async function executeMigrationUp(targetVersion?: string): Promise<void> {
  await invoke('execute_migration_up', { targetVersion })
}

/**
 * 执行数据库降级迁移
 * @param targetVersion 目标版本，如果不传则降级到上一版本
 */
export async function executeMigrationDown(targetVersion?: string): Promise<void> {
  await invoke('execute_migration_down', { targetVersion })
}
