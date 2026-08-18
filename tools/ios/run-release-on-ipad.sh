#!/usr/bin/env bash

set -euo pipefail

script_directory="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repository_root="$(cd "${script_directory}/../.." && pwd)"
device_list_path="$(mktemp "${TMPDIR:-/tmp}/english-in-use-devices.XXXXXX.json")"
build_log_path="$(mktemp "${TMPDIR:-/tmp}/english-in-use-ios-build.XXXXXX.log")"

# 删除本次执行产生的临时文件，避免保留设备标识和构建信息。
cleanup() {
  rm -f "${device_list_path:?}" "${build_log_path:?}"
}
trap cleanup EXIT

# 输出 Personal Team 开发签名的续期步骤。
print_signing_guidance() {
  {
    echo "检测到 iOS 开发签名或描述文件不可用。"
    echo "Personal Team 的开发描述文件通常只有 7 天有效期，请在 Xcode 中完成续签："
    echo "1. 打开 src-tauri/gen/apple/EnglishInUse.xcodeproj。"
    echo "2. 在 EnglishInUse_iOS 的 Signing & Capabilities 中启用自动签名并选择当前团队。"
    echo "3. 选择已连接的 iPad 作为运行目标，执行一次 Product > Run。"
    echo "4. 成功安装后重新运行 pnpm tauri:ios:run:release。"
  } >&2
}

cd "${repository_root}"

echo "正在检测当前连接的 iPad..."
xcrun devicectl list devices --json-output "${device_list_path}" --quiet

# devicectl 的表格输出可能随 Xcode 变化，因此读取其用于脚本处理的 JSON 输出。
device_identifier="$(
  node - "${device_list_path}" <<'NODE'
const { readFileSync } = require('node:fs')

const deviceListPath = process.argv[2]
const output = JSON.parse(readFileSync(deviceListPath, 'utf8'))
const devices = Array.isArray(output.result?.devices) ? output.result.devices : []

const normalize = (value) => (typeof value === 'string' ? value.toLowerCase() : '')
const isIPad = (device) => {
  const hardware = device.hardwareProperties ?? {}
  return [hardware.deviceType, hardware.productType, hardware.marketingName].some((value) =>
    normalize(value).startsWith('ipad'),
  )
}
const isConnected = (device) =>
  normalize(device.connectionProperties?.tunnelState) === 'connected'

const connectedIPads = devices.filter((device) => isIPad(device) && isConnected(device))

if (connectedIPads.length !== 1) {
  const descriptions = connectedIPads.map((device) => {
    const name = device.deviceProperties?.name ?? '未命名 iPad'
    return `${name} (${device.identifier ?? '无设备标识'})`
  })
  const details = descriptions.length > 0 ? `：${descriptions.join('、')}` : ''
  console.error(`需要恰好连接一台 iPad，当前检测到 ${connectedIPads.length} 台${details}`)
  process.exit(1)
}

const device = connectedIPads[0]
if (typeof device.identifier !== 'string' || device.identifier.length === 0) {
  console.error('当前 iPad 缺少可用于安装的设备标识')
  process.exit(1)
}

console.error(`将安装到：${device.deviceProperties?.name ?? '未命名 iPad'}`)
process.stdout.write(device.identifier)
NODE
)"

echo "正在构建 iOS release 应用..."
node tools/ios/ensure-clang-runtime-cache.mjs
if ! VITE_ENABLE_DEBUG_FEATURES=false pnpm tauri ios build --export-method debugging 2>&1 | tee "${build_log_path}"; then
  if grep -Eiq \
    "No profiles for|Your team has no devices|No signing certificate|provisioning profiles matching" \
    "${build_log_path}"; then
    print_signing_guidance
  fi
  echo "iOS release 构建失败，完整错误已显示在上方。" >&2
  exit 1
fi

archive_path="src-tauri/gen/apple/build/EnglishInUse_iOS.xcarchive"
ipa_path="src-tauri/gen/apple/build/arm64/english-in-use.ipa"
app_path="${archive_path}/Products/Applications/english-in-use.app"
if [[ ! -f "${ipa_path}" ]]; then
  echo "未找到 IPA 构建产物：${ipa_path}" >&2
  exit 1
fi
if [[ ! -d "${app_path}" ]]; then
  echo "未找到 APP 构建产物：${app_path}" >&2
  exit 1
fi

echo "IPA 构建完成：${ipa_path}"
echo "正在安装应用..."
xcrun devicectl device install app --device "${device_identifier}" "${app_path}"

echo "正在启动应用..."
xcrun devicectl device process launch \
  --device "${device_identifier}" \
  --terminate-existing \
  com.ethan.english-in-use
