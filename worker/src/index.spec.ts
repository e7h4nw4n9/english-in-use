import { beforeEach, describe, expect, it, vi } from 'vitest'
import { handleRequest, type CacheLike, type Env, type ExecutionContextLike } from './index'

class MemoryCache implements CacheLike {
  private readonly responses = new Map<string, Response>()

  async match(request: Request): Promise<Response | undefined> {
    return this.responses.get(request.url)?.clone()
  }

  async put(request: Request, response: Response): Promise<void> {
    this.responses.set(request.url, response.clone())
  }
}

function createContext(): ExecutionContextLike {
  return {
    waitUntil(promise) {
      void promise
    },
  }
}

function createEnv(overrides: Partial<Env> = {}) {
  const session = {
    prepare: vi.fn((sql: string) => ({
      bind: vi.fn((...params: unknown[]) => ({ sql, params })),
      first: vi.fn(async () => ({ version: '0.5.0' })),
    })),
    batch: vi.fn(async (statements: unknown[]) =>
      statements.map(() => ({ success: true, results: [{ value: 1 }], meta: {} })),
    ),
    getBookmark: vi.fn(() => '0002-bookmark'),
  }
  const env = {
    GATEWAY_TOKEN: 'secret',
    DB: {
      withSession: vi.fn(() => session),
    },
    BOOKS: {
      get: vi.fn(async () => ({
        body: new ReadableStream({
          start(controller) {
            controller.enqueue(new TextEncoder().encode('object'))
            controller.close()
          },
        }),
        httpEtag: 'etag',
        writeHttpMetadata: vi.fn(),
      })),
      list: vi.fn(async () => ({ objects: [{ key: 'books/demo/a' }], truncated: false })),
    },
    ...overrides,
  } as unknown as Env
  return { env, session }
}

function authorizedRequest(path: string, init: RequestInit = {}): Request {
  const headers = new Headers(init.headers)
  headers.set('authorization', 'Bearer secret')
  return new Request(`https://gateway.example.com${path}`, { ...init, headers })
}

describe('Cloudflare gateway', () => {
  beforeEach(() => vi.restoreAllMocks())

  it('rejects unauthenticated requests before accessing bindings', async () => {
    const { env } = createEnv()
    const response = await handleRequest(
      new Request('https://gateway.example.com/v1/health'),
      env,
      createContext(),
      new MemoryCache(),
    )

    expect(response.status).toBe(401)
    expect(env.DB.withSession).not.toHaveBeenCalled()
    expect(env.BOOKS.list).not.toHaveBeenCalled()
  })

  it('executes an ordered batch and forwards the bookmark', async () => {
    const { env, session } = createEnv()
    const response = await handleRequest(
      authorizedRequest('/v1/d1/batch', {
        method: 'POST',
        headers: { 'x-d1-bookmark': '0001-bookmark' },
        body: JSON.stringify({
          mode: 'write',
          statements: [{ sql: 'SELECT ?', params: [true] }],
        }),
      }),
      env,
      createContext(),
      new MemoryCache(),
    )

    expect(response.status).toBe(200)
    expect(env.DB.withSession).toHaveBeenCalledWith('0001-bookmark')
    expect(session.batch).toHaveBeenCalledTimes(1)
    expect(response.headers.get('x-d1-bookmark')).toBe('0002-bookmark')
  })

  it('rejects oversized batches without calling D1', async () => {
    const { env, session } = createEnv()
    const statements = Array.from({ length: 33 }, () => ({ sql: 'SELECT 1', params: [] }))
    const response = await handleRequest(
      authorizedRequest('/v1/d1/batch', {
        method: 'POST',
        body: JSON.stringify({ mode: 'read', statements }),
      }),
      env,
      createContext(),
      new MemoryCache(),
    )

    expect(response.status).toBe(400)
    expect(session.batch).not.toHaveBeenCalled()
  })

  it('caches immutable R2 objects after authentication', async () => {
    const { env } = createEnv()
    const cache = new MemoryCache()
    const first = await handleRequest(
      authorizedRequest('/v1/r2/objects/books/demo/page.jpg'),
      env,
      createContext(),
      cache,
    )
    await first.text()
    const second = await handleRequest(
      authorizedRequest('/v1/r2/objects/books/demo/page.jpg'),
      env,
      createContext(),
      cache,
    )

    expect(first.headers.get('x-gateway-cache')).toBe('MISS')
    expect(second.headers.get('x-gateway-cache')).toBe('HIT')
    expect(second.headers.get('cache-control')).toContain('private')
    expect(second.headers.get('x-request-id')).not.toBe(first.headers.get('x-request-id'))
    expect(env.BOOKS.get).toHaveBeenCalledTimes(1)
  })

  it('rejects traversal and unapproved R2 prefixes', async () => {
    const { env } = createEnv()
    const response = await handleRequest(
      authorizedRequest('/v1/r2/objects/private/secret.txt'),
      env,
      createContext(),
      new MemoryCache(),
    )

    expect(response.status).toBe(400)
    expect(env.BOOKS.get).not.toHaveBeenCalled()
  })

  it('reports D1 and R2 health independently', async () => {
    const { env } = createEnv({
      BOOKS: {
        get: vi.fn(),
        list: vi.fn(async () => {
          throw new Error('r2 unavailable')
        }),
      } as unknown as R2Bucket,
    })
    const response = await handleRequest(
      authorizedRequest('/v1/health'),
      env,
      createContext(),
      new MemoryCache(),
    )
    const body = await response.json<{ database: string; r2: string }>()

    expect(response.status).toBe(503)
    expect(body.database).toBe('connected')
    expect(body.r2).toBe('disconnected')
  })
})
