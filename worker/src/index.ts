const MAX_BATCH_STATEMENTS = 32
const MAX_PARAMS_PER_STATEMENT = 100
const MAX_SQL_BYTES = 100 * 1024
const MAX_LIST_LIMIT = 1000
const OBJECT_CACHE_CONTROL = 'private, max-age=31536000, immutable'
const EDGE_CACHE_CONTROL = 'public, s-maxage=31536000, immutable'

export interface Env {
  DB: D1Database
  BOOKS: R2Bucket
  GATEWAY_TOKEN: string
}

export interface ExecutionContextLike {
  waitUntil(promise: Promise<unknown>): void
}

export interface CacheLike {
  match(request: Request): Promise<Response | undefined>
  put(request: Request, response: Response): Promise<void>
}

type SqlParam = null | boolean | number | string

interface BatchStatement {
  sql: string
  params: SqlParam[]
}

interface BatchRequestBody {
  mode: 'read' | 'write'
  statements: BatchStatement[]
}

/** 返回不包含内部实现细节的统一错误。 */
function errorResponse(status: number, code: string, message: string, requestId: string): Response {
  return Response.json({ error: { code, message, requestId } }, { status })
}

/** 校验网关 Bearer Token。 */
function isAuthorized(request: Request, env: Env): boolean {
  const authorization = request.headers.get('authorization')
  return authorization === `Bearer ${env.GATEWAY_TOKEN}` && env.GATEWAY_TOKEN.length > 0
}

/** 解析并限制批处理请求，防止单次请求占用过多资源。 */
function validateBatchBody(value: unknown): BatchRequestBody {
  if (!value || typeof value !== 'object') throw new Error('请求体必须是 JSON 对象')
  const body = value as Partial<BatchRequestBody>
  if (body.mode !== 'read' && body.mode !== 'write') throw new Error('mode 必须为 read 或 write')
  if (!Array.isArray(body.statements) || body.statements.length === 0) {
    throw new Error('statements 不能为空')
  }
  if (body.statements.length > MAX_BATCH_STATEMENTS) {
    throw new Error(`statements 不能超过 ${MAX_BATCH_STATEMENTS} 条`)
  }

  const encoder = new TextEncoder()
  for (const statement of body.statements) {
    if (!statement || typeof statement.sql !== 'string' || statement.sql.trim().length === 0) {
      throw new Error('SQL 不能为空')
    }
    if (encoder.encode(statement.sql).byteLength > MAX_SQL_BYTES) {
      throw new Error('单条 SQL 不能超过 100 KB')
    }
    if (!Array.isArray(statement.params) || statement.params.length > MAX_PARAMS_PER_STATEMENT) {
      throw new Error(`单条 SQL 参数不能超过 ${MAX_PARAMS_PER_STATEMENT} 个`)
    }
    if (
      statement.params.some(
        (param) =>
          param !== null &&
          typeof param !== 'boolean' &&
          typeof param !== 'number' &&
          typeof param !== 'string',
      )
    ) {
      throw new Error('SQL 参数仅支持 null、boolean、number 和 string')
    }
  }
  return body as BatchRequestBody
}

/** 执行有序 D1 批处理并传递 Session Bookmark。 */
async function handleD1Batch(request: Request, env: Env, requestId: string): Promise<Response> {
  let body: BatchRequestBody
  try {
    body = validateBatchBody(await request.json())
  } catch (error) {
    return errorResponse(400, 'INVALID_BATCH', String(error), requestId)
  }

  const bookmark = request.headers.get('x-d1-bookmark')
  const session = env.DB.withSession(
    bookmark || (body.mode === 'read' ? 'first-unconstrained' : 'first-primary'),
  )
  const statements = body.statements.map((statement) => {
    const params = statement.params.map((param) =>
      typeof param === 'boolean' ? Number(param) : param,
    )
    return session.prepare(statement.sql).bind(...params)
  })
  const results = await session.batch(statements)
  const headers = new Headers({ 'content-type': 'application/json', 'x-request-id': requestId })
  const nextBookmark = session.getBookmark()
  if (nextBookmark) headers.set('x-d1-bookmark', nextBookmark)
  return new Response(JSON.stringify({ results, requestId }), { headers })
}

/** 解析并限制允许访问的不可变对象键。 */
function parseObjectKey(pathname: string): string | null {
  let key: string
  try {
    key = decodeURIComponent(pathname.slice('/v1/r2/objects/'.length)).replace(/^\/+/, '')
  } catch {
    return null
  }
  if (
    !key ||
    key.includes('\\') ||
    key.split('/').some((part) => part === '.' || part === '..') ||
    (!key.startsWith('books/') && !key.startsWith('courses/'))
  ) {
    return null
  }
  return key
}

/** 将边缘缓存响应改写为仅允许客户端私有缓存。 */
function privateObjectResponse(
  response: Response,
  cacheStatus: 'HIT' | 'MISS',
  requestId: string,
): Response {
  const headers = new Headers(response.headers)
  headers.set('cache-control', OBJECT_CACHE_CONTROL)
  headers.set('x-gateway-cache', cacheStatus)
  headers.set('x-request-id', requestId)
  return new Response(response.body, { status: response.status, headers })
}

/** 获取 R2 对象，并在鉴权后使用不含凭据的规范缓存键。 */
async function handleR2Object(
  request: Request,
  env: Env,
  context: ExecutionContextLike,
  cache: CacheLike,
  requestId: string,
): Promise<Response> {
  const url = new URL(request.url)
  const key = parseObjectKey(url.pathname)
  if (!key) return errorResponse(400, 'INVALID_OBJECT_KEY', '对象键无效', requestId)

  const cacheUrl = new URL(request.url)
  cacheUrl.search = ''
  const cacheRequest = new Request(cacheUrl.toString(), { method: 'GET' })
  const cached = await cache.match(cacheRequest)
  if (cached) return privateObjectResponse(cached, 'HIT', requestId)

  const object = await env.BOOKS.get(key)
  if (!object) return errorResponse(404, 'OBJECT_NOT_FOUND', '对象不存在', requestId)

  const headers = new Headers()
  object.writeHttpMetadata(headers)
  headers.set('cache-control', EDGE_CACHE_CONTROL)
  headers.set('etag', object.httpEtag)
  headers.set('x-request-id', requestId)
  const edgeResponse = new Response(object.body, { headers })
  context.waitUntil(cache.put(cacheRequest, edgeResponse.clone()))
  return privateObjectResponse(edgeResponse, 'MISS', requestId)
}

/** 分页列出指定前缀下的 R2 对象。 */
async function handleR2List(request: Request, env: Env, requestId: string): Promise<Response> {
  const url = new URL(request.url)
  const prefix = url.searchParams.get('prefix') || ''
  if (
    prefix.includes('\\') ||
    prefix.split('/').some((part) => part === '..') ||
    (prefix && !prefix.startsWith('books/') && !prefix.startsWith('courses/'))
  ) {
    return errorResponse(400, 'INVALID_PREFIX', '对象前缀无效', requestId)
  }
  const requestedLimit = Number(url.searchParams.get('limit') || MAX_LIST_LIMIT)
  if (!Number.isInteger(requestedLimit) || requestedLimit < 1) {
    return errorResponse(400, 'INVALID_LIMIT', 'limit 必须是正整数', requestId)
  }
  const result = await env.BOOKS.list({
    prefix,
    cursor: url.searchParams.get('cursor') || undefined,
    limit: Math.min(requestedLimit, MAX_LIST_LIMIT),
  })
  return Response.json(
    {
      keys: result.objects.map((object) => object.key),
      cursor: result.truncated ? result.cursor : null,
      truncated: result.truncated,
    },
    { headers: { 'x-request-id': requestId } },
  )
}

/** 并行检查 D1 与 R2，避免状态检查遍历整个存储桶。 */
async function handleHealth(env: Env, requestId: string): Promise<Response> {
  const database = env.DB.withSession('first-primary')
  const checkDatabase = async () => {
    const table = await database
      .prepare("SELECT name FROM sqlite_master WHERE type='table' AND name='_app_meta' LIMIT 1")
      .first<{ name: string }>()
    if (!table) return { version: '0.0.0' }
    return await database
      .prepare('SELECT version FROM _app_meta LIMIT 1')
      .first<{ version: string }>()
  }
  const [databaseResult, r2Result] = await Promise.allSettled([
    checkDatabase(),
    env.BOOKS.list({ limit: 1 }),
  ])
  const databaseConnected = databaseResult.status === 'fulfilled'
  const r2Connected = r2Result.status === 'fulfilled'
  const schemaVersion = databaseConnected ? (databaseResult.value?.version ?? '0.0.0') : null
  return Response.json(
    {
      database: databaseConnected ? 'connected' : 'disconnected',
      r2: r2Connected ? 'connected' : 'disconnected',
      schemaVersion,
      requestId,
    },
    {
      status: databaseConnected && r2Connected ? 200 : 503,
      headers: { 'x-request-id': requestId },
    },
  )
}

/** Worker 请求入口；鉴权必须先于缓存读取和绑定访问。 */
export async function handleRequest(
  request: Request,
  env: Env,
  context: ExecutionContextLike,
  cache?: CacheLike,
): Promise<Response> {
  const requestId = crypto.randomUUID()
  if (!isAuthorized(request, env)) {
    return errorResponse(401, 'UNAUTHORIZED', '访问令牌无效', requestId)
  }

  try {
    const url = new URL(request.url)
    if (request.method === 'POST' && url.pathname === '/v1/d1/batch') {
      return await handleD1Batch(request, env, requestId)
    }
    if (request.method === 'GET' && url.pathname.startsWith('/v1/r2/objects/')) {
      const objectCache = cache ?? (caches as unknown as { default: CacheLike }).default
      return await handleR2Object(request, env, context, objectCache, requestId)
    }
    if (request.method === 'GET' && url.pathname === '/v1/r2/list') {
      return await handleR2List(request, env, requestId)
    }
    if (request.method === 'GET' && url.pathname === '/v1/health') {
      return await handleHealth(env, requestId)
    }
    return errorResponse(404, 'NOT_FOUND', '接口不存在', requestId)
  } catch {
    return errorResponse(502, 'UPSTREAM_ERROR', 'Cloudflare 绑定操作失败', requestId)
  }
}

export default { fetch: handleRequest }
