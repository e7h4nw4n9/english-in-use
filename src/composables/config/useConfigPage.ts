import type { AppConfig } from '../../types'
import { createConfigPageContext, type ConfigPageEmit } from './configPageContext'
import { useConfigPathOperations } from './useConfigPathOperations'
import { useConfigFileAccess } from './useConfigFileAccess'
import { useConfigPersistenceOperations } from './useConfigPersistenceOperations'

/**
 * 组合配置页面的表单、路径和持久化职责。
 * @param initialConfig - 页面首次加载的配置。
 * @param allowBack - 是否显示返回入口。
 * @param emit - 配置页面事件发送器。
 */
export function useConfigPage(
  initialConfig: AppConfig | undefined,
  allowBack: boolean,
  emit: ConfigPageEmit,
) {
  const context = createConfigPageContext(initialConfig, allowBack, emit)
  const paths = useConfigPathOperations(context)
  const fileAccess = useConfigFileAccess(context, paths)
  const persistence = useConfigPersistenceOperations(context, { ...paths, ...fileAccess })
  return { ...context, ...paths, ...fileAccess, ...persistence }
}
