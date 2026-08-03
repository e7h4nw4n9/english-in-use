# Cloudflare 私有网关

该 Worker 为桌面端提供经过鉴权的 D1 批处理和 R2 私有读取接口。

1. 将 `wrangler.example.jsonc` 复制为 `wrangler.jsonc`，填写 D1 与 R2 绑定。
2. 执行 `pnpm exec wrangler secret put GATEWAY_TOKEN` 配置访问令牌。
3. 为 Worker 配置自定义域名后执行 `pnpm deploy`。
4. 在应用配置中填写自定义域名和相同的访问令牌。

R2 对象键必须位于 `books/` 或 `courses/` 下。对象按不可变资源缓存一年，更新资源时应使用新的对象键。
