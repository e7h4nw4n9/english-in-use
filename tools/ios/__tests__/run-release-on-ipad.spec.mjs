import { readFile } from 'node:fs/promises'
import { resolve } from 'node:path'
import { describe, expect, it } from 'vitest'

const scriptPath = resolve(process.cwd(), 'tools/ios/run-release-on-ipad.sh')

describe('iPad release 运行脚本', () => {
  it('保留构建日志并在退出时清理临时信息', async () => {
    const script = await readFile(scriptPath, 'utf8')

    expect(script).toContain('2>&1 | tee "${build_log_path}"')
    expect(script).not.toContain('--export-method debugging >/dev/null')
    expect(script).toContain('rm -f "${device_list_path:?}" "${build_log_path:?}"')
  })

  it('识别签名错误并提供 Personal Team 续签指引', async () => {
    const script = await readFile(scriptPath, 'utf8')

    expect(script).toContain('No profiles for|Your team has no devices|No signing certificate')
    expect(script).toContain('print_signing_guidance')
    expect(script).toContain('Signing & Capabilities')
  })

  it('安装前同时校验 IPA 和 APP 构建产物', async () => {
    const script = await readFile(scriptPath, 'utf8')

    expect(script).toContain('[[ ! -f "${ipa_path}" ]]')
    expect(script).toContain('[[ ! -d "${app_path}" ]]')
    expect(script.indexOf('[[ ! -f "${ipa_path}" ]]')).toBeLessThan(
      script.indexOf('xcrun devicectl device install app'),
    )
    expect(script.indexOf('[[ ! -d "${app_path}" ]]')).toBeLessThan(
      script.indexOf('xcrun devicectl device install app'),
    )
  })
})
