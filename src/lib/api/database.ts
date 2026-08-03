import { invoke } from '@tauri-apps/api/core'
import type { CloudflareGatewayConfig, ConnectionStatus, DatabaseConnection } from '../../types'

/**
 * 初始化数据库
 * @returns 是否执行了升级迁移
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
 * 解析 SQLite 路径：
 * - 传入目录且目录存在数据库文件时，优先返回已有数据库文件路径
 * - 传入目录但没有数据库文件时，返回该目录下默认数据库文件路径
 * - 传入文件路径时直接返回
 */
export async function resolveSqlitePath(path: string): Promise<string> {
  return await invoke<string>('resolve_sqlite_path', { path })
}

/**
 * 测试数据库连接
 * @param connection 数据库连接配置
 */
export async function testDatabaseConnection(
  connection: DatabaseConnection,
  gateway?: CloudflareGatewayConfig,
): Promise<void> {
  await invoke('test_database_connection', { connection, gateway })
}

/**
 * 同时测试私有网关的 D1 与 R2 绑定。
 * @param gateway 网关地址和访问令牌。
 */
export async function testCloudflareGateway(
  gateway: CloudflareGatewayConfig,
): Promise<ConnectionStatus> {
  return await invoke<ConnectionStatus>('test_cloudflare_gateway', { gateway })
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
