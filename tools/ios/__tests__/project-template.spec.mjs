import { readFile } from 'node:fs/promises'
import { resolve } from 'node:path'
import { describe, expect, it } from 'vitest'

const templatePath = resolve(process.cwd(), 'src-tauri/ios/project.yml')

describe('iOS 项目模板', () => {
  it('三个自定义脚本区块使用各自的路径和文件列表字段', async () => {
    const template = await readFile(templatePath, 'utf8')

    expect(template.match(/- path: \{\{this\.path\}\}/g)).toHaveLength(3)
    expect(template.match(/inputFileLists: \{\{~#each this\.input-file-lists\}\}/g)).toHaveLength(3)
    expect(template.match(/outputFileLists: \{\{~#each this\.output-file-lists\}\}/g)).toHaveLength(
      3,
    )
    expect(template).not.toContain('inputFileLists: {{~#each this.output-files}}')
  })
})
