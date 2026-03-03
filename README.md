# English In Use

## 项目说明

本项目旨在为剑桥“English In Use”系列丛书提供一个现代化、跨平台的阅读体验。
鉴于官方客户端仅限于桌面平台且交互体验存在局限，本项目基于 **Tauri 2.0** 与 **Vue 3** 构建，
致力于为个人学习提供更流畅、便捷的数字化支持，并探索在多端（桌面端及未来的移动端）运行的可能性。

## 声明

1. **尊重版权**：请务必支持原作者，购买正版纸质书籍或官方电子授权。
2. **资源免责**：本项目**不包含也不提供**任何书籍的正文内容、音频、图片或相关受版权保护的电子资源。用户需自行拥有合法资源并导入使用。
3. **使用限制**：本项目源代码仅供技术交流与个人学习使用，禁止用于任何商业用途。

## 本地资源目录约定

当图书来源配置为本地目录时，建议目录结构如下：

```text
<本地根目录>/
  books/
    <product_code>/
      meta/definition.json
      assets/imgbook-meta/book.json
      assets/imgbook-meta/book-overlays.json
      ...
  courses/                       # 推荐；缺失时练习资源可能不可用
    <course_id>/
      meta/definition.json
      assets/...
```

- `books/` 是必需目录；缺失时会在配置保存/导入阶段提示错误。
- `courses/` 是推荐目录；缺失时会提示告警，阅读功能仍可使用。
- 系统会拒绝包含 `..` 的非法相对路径，避免越界读取本地文件。

## 开发说明

本项目使用 Tauri 2.0、Vue 3 开发。

### 环境准备

1. **Node.js**: 请确保已安装 Node.js (建议使用 LTS 版本)。
2. **包管理器**: 本项目推荐使用 `pnpm`。
3. **Rust**: 请确保已安装 Rust 语言环境（建议通过 [rustup](https://rustup.rs/) 安装最新稳定版）。
4. **系统依赖**: 请参考 [Tauri 官方文档](https://v2.tauri.app/start/prerequisites/) 确保您的操作系统（Windows/macOS/Linux）已安装必要的构建工具。

### 开始开发

1. 安装依赖：

   ```shell
   pnpm install
   ```

2. 启动开发环境（由 Tauri 托管）：

   ```shell
   pnpm tauri dev
   ```

### 构建应用

构建生产环境的安装包（构建产物位于 `src-tauri/target/release/bundle` 下）：

```shell
pnpm tauri build
```

### iOS / iPad 测试

本项目已使用 Tauri 2，支持 iOS（含 iPad）开发与测试。

0. 确认 Xcode Developer Directory 已指向 Xcode（而非 CommandLineTools）：

   ```shell
   xcode-select -p
   ```

   切换到 Xcode：

   ```shell
   sudo xcode-select --switch /Applications/Xcode.app/Contents/Developer
   ```

   重置为系统默认：

   ```shell
   sudo xcode-select --reset
   ```

1. 安装 iOS 目标（一次性）：

   ```shell
   rustup target add aarch64-apple-ios aarch64-apple-ios-sim x86_64-apple-ios
   ```

2. 初始化 iOS 工程（一次性）：

   ```shell
   pnpm tauri:ios:init
   ```

   生成目录通常为 `src-tauri/gen/apple/`。

3. 模拟器测试（推荐先跑）：

   ```shell
   pnpm tauri:ios:dev
   ```

   可先用 `xcrun simctl list devices available` 查看可用模拟器，再在 Xcode 中选择 iPad 机型运行。

4. iPad/iPhone 真机测试：

   由于真机无法访问 `localhost`，请把 `TAURI_DEV_HOST` 设为开发机局域网 IP（例如 `192.168.1.10`）：

   ```shell
   TAURI_DEV_HOST=192.168.1.10 pnpm tauri:ios:dev
   ```

   同时在 Xcode 中完成 Team/Signing 配置后，切换到真机运行。

5. 生成 iOS 发布产物（可选）：

   ```shell
   pnpm tauri:ios:build
   ```

建议最少覆盖以下测试项：启动与渲染、核心业务路径、Rust command 错误分支、插件能力（dialog/fs/log/opener）和真机网络连通性。
