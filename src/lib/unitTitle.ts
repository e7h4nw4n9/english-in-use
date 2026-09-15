/**
 * 获取目录与学习记录共用的单元前缀；已有编号或附录标题不重复添加。
 * @param resourceId - 单元资源标识，历史记录同样保留此字段。
 * @param title - 原始单元标题。
 * @param unitNumber - 阅读器目录已解析的编号，存在时优先使用。
 */
export function getUnitPrefix(resourceId: string, title: string, unitNumber?: number): string {
  if (
    title.trimStart().startsWith('Appendix ') ||
    /^\s*Unit\s+\d+(?=\s|$|[:：.–—-])/i.test(title)
  ) {
    return ''
  }

  // 与 MetadataService::parse_unit_number 一致：RE_ 后为正整数，范围为 u32。
  const resourceNumber = /^RE_\+?(\d+)$/.exec(resourceId)
  const number =
    unitNumber ?? (resourceNumber?.[0] === resourceId ? Number(resourceNumber[1]) : NaN)
  return Number.isInteger(number) && number > 0 && number <= 0xffffffff ? `Unit ${number}` : ''
}

/**
 * 组合展示标题，不改写数据库中的名称或单元标识。
 * @param resourceId - 单元资源标识。
 * @param title - 原始单元标题。
 */
export function formatUnitTitle(resourceId: string, title: string): string {
  const prefix = getUnitPrefix(resourceId, title)
  return prefix ? `${prefix} ${title}` : title
}
