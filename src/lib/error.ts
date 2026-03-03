export interface ParsedCommandError {
  code: string | null
  message: string
  raw: string
}

const CODE_PREFIX_REGEX = /^\[([A-Z0-9_]+)\]\s*(.*)$/

export function parseCommandError(error: unknown): ParsedCommandError {
  const raw = error instanceof Error ? error.message : String(error ?? '')
  const match = raw.match(CODE_PREFIX_REGEX)

  if (!match) {
    return {
      code: null,
      message: raw,
      raw,
    }
  }

  return {
    code: match[1] || null,
    message: match[2] || raw,
    raw,
  }
}

export function getReadableCommandError(error: unknown): string {
  const parsed = parseCommandError(error)

  if (!parsed.code) return parsed.message

  switch (parsed.code) {
    case 'ERR_RESOURCE_NOT_FOUND':
      return '资源不存在或未同步完整，请检查本地目录结构。'
    case 'ERR_OVERLAY_NO_COURSE_ID':
      return '练习映射配置缺少有效 courseId，无法关联练习资源。'
    case 'ERR_OVERLAY_INVALID_STRUCTURE':
      return '书籍叠加层配置结构异常，请检查 book-overlays.json。'
    case 'ERR_PATH_OUTSIDE_BASE':
    case 'ERR_PATH_INVALID_RELATIVE':
      return '检测到非法资源路径，已阻止访问。'
    default:
      return parsed.message || parsed.raw
  }
}
