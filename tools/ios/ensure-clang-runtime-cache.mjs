import { execFileSync, spawnSync } from 'node:child_process'
import { existsSync, readFileSync, readdirSync } from 'node:fs'
import path from 'node:path'
import { fileURLToPath, pathToFileURL } from 'node:url'

const IOS_TARGET = 'aarch64-apple-ios'
const BUILD_PROFILE = 'release'
const SWIFT_LINK_PACKAGES = ['tauri', 'tauri-plugin-dialog', 'tauri-plugin-log']
const scriptDirectory = path.dirname(fileURLToPath(import.meta.url))
const repositoryRoot = path.resolve(scriptDirectory, '../..')
const cargoManifestPath = path.join(repositoryRoot, 'src-tauri', 'Cargo.toml')

/**
 * 从 Cargo 构建脚本输出中提取 Clang runtime 搜索目录。
 *
 * @param {string} output Cargo 构建脚本输出。
 * @returns {string[]} Clang runtime 搜索目录。
 */
export function extractClangRuntimeSearchPaths(output) {
  return output
    .split(/\r?\n/u)
    .filter((line) => line.startsWith('cargo:rustc-link-search='))
    .map((line) =>
      line
        .slice('cargo:rustc-link-search='.length)
        .replace(/^native=/u, '')
        .trim(),
    )
    .filter((searchPath) => /\/usr\/lib\/clang\/[^/]+\/lib\/darwin$/u.test(searchPath))
}

/**
 * 判断缓存的构建脚本输出是否引用了其他 Clang runtime 目录。
 *
 * @param {string} output Cargo 构建脚本输出。
 * @param {string} currentRuntimePath 当前 Xcode 的 Clang runtime 目录。
 * @returns {boolean} 是否需要使缓存失效。
 */
export function hasStaleClangRuntimePath(output, currentRuntimePath) {
  const normalizedCurrentPath = path.normalize(currentRuntimePath)
  return extractClangRuntimeSearchPaths(output).some(
    (cachedPath) => path.normalize(cachedPath) !== normalizedCurrentPath,
  )
}

/**
 * 判断 Cargo build 子目录是否属于指定包。
 *
 * @param {string} directoryName Cargo build 子目录名称。
 * @param {string} packageName Cargo 包名。
 * @returns {boolean} 是否为该包的哈希目录。
 */
export function isPackageBuildDirectory(directoryName, packageName) {
  const prefix = `${packageName}-`
  return directoryName.startsWith(prefix) && /^[0-9a-f]+$/u.test(directoryName.slice(prefix.length))
}

/**
 * 执行只读命令并返回标准输出。
 *
 * @param {string} command 命令名称。
 * @param {string[]} args 命令参数。
 * @returns {string} 去除首尾空白后的标准输出。
 */
function readCommandOutput(command, args) {
  return execFileSync(command, args, {
    cwd: repositoryRoot,
    encoding: 'utf8',
    stdio: ['ignore', 'pipe', 'pipe'],
  }).trim()
}

/**
 * 获取当前 Xcode 的 Clang runtime 目录并验证 iOS runtime 存在。
 *
 * @returns {string} 当前 Clang runtime 目录。
 */
function getCurrentClangRuntimePath() {
  const resourceDirectory = readCommandOutput('xcrun', [
    '--sdk',
    'iphoneos',
    'clang',
    '--print-resource-dir',
  ])
  const runtimePath = path.join(resourceDirectory, 'lib', 'darwin')
  const iosRuntimeLibrary = path.join(runtimePath, 'libclang_rt.ios.a')

  if (!existsSync(iosRuntimeLibrary)) {
    throw new Error(`未找到 iOS Clang runtime：${iosRuntimeLibrary}`)
  }

  return runtimePath
}

/**
 * 读取 Cargo 当前实际使用的 target 目录。
 *
 * @returns {string} Cargo target 目录。
 */
function getCargoTargetDirectory() {
  const metadata = JSON.parse(
    readCommandOutput('cargo', [
      'metadata',
      '--manifest-path',
      cargoManifestPath,
      '--format-version',
      '1',
      '--no-deps',
      '--offline',
    ]),
  )

  if (typeof metadata.target_directory !== 'string' || metadata.target_directory.length === 0) {
    throw new Error('Cargo metadata 未返回有效的 target 目录')
  }

  return path.resolve(metadata.target_directory)
}

/**
 * 找出引用旧 Clang runtime 的 Tauri 包缓存。
 *
 * @param {string} buildDirectory Cargo 的目标 profile 构建目录。
 * @param {string} currentRuntimePath 当前 Clang runtime 目录。
 * @returns {string[]} 需要清理的包名。
 */
function findStalePackages(buildDirectory, currentRuntimePath) {
  if (!existsSync(buildDirectory)) {
    return []
  }

  const entries = readdirSync(buildDirectory, { withFileTypes: true })
  return SWIFT_LINK_PACKAGES.filter((packageName) =>
    entries.some((entry) => {
      if (!entry.isDirectory() || !isPackageBuildDirectory(entry.name, packageName)) {
        return false
      }

      const outputPath = path.join(buildDirectory, entry.name, 'output')
      return (
        existsSync(outputPath) &&
        hasStaleClangRuntimePath(readFileSync(outputPath, 'utf8'), currentRuntimePath)
      )
    }),
  )
}

/**
 * 仅清理会缓存 Clang runtime 路径的包，避免删除完整 iOS 构建缓存。
 *
 * @param {string[]} packageNames 需要清理的包名。
 */
function cleanStalePackages(packageNames) {
  const packageArgs = packageNames.flatMap((packageName) => ['--package', packageName])
  const result = spawnSync(
    'cargo',
    [
      'clean',
      '--manifest-path',
      cargoManifestPath,
      '--target',
      IOS_TARGET,
      '--release',
      ...packageArgs,
    ],
    {
      cwd: repositoryRoot,
      stdio: 'inherit',
    },
  )

  if (result.error) {
    throw result.error
  }
  if (result.status !== 0) {
    throw new Error(`Cargo 缓存清理失败，退出码：${result.status ?? '未知'}`)
  }
}

/**
 * 在 iOS release 构建前检查并修复失效的 Clang runtime 缓存。
 */
function main() {
  const currentRuntimePath = getCurrentClangRuntimePath()
  const targetDirectory = getCargoTargetDirectory()
  const buildDirectory = path.join(targetDirectory, IOS_TARGET, BUILD_PROFILE, 'build')
  const stalePackages = findStalePackages(buildDirectory, currentRuntimePath)

  if (stalePackages.length === 0) {
    return
  }

  console.warn(`检测到 Xcode Clang runtime 已变化，将清理失效缓存：${stalePackages.join(', ')}`)
  cleanStalePackages(stalePackages)
}

const invokedPath = process.argv[1] ? pathToFileURL(path.resolve(process.argv[1])).href : ''
if (import.meta.url === invokedPath) {
  try {
    main()
  } catch (error) {
    const message = error instanceof Error ? error.message : String(error)
    console.error(`iOS Clang runtime 缓存检查失败：${message}`)
    process.exitCode = 1
  }
}
