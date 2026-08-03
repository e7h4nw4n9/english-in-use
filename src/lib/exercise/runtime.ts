const RUNTIME_LOG_TYPE = 'eiu-exercise-runtime-log'

type ChannelMethodHandler = (transaction: unknown, params: Record<string, unknown>) => unknown

interface ChannelInstance {
  bind(method: string, handler: ChannelMethodHandler): void
  destroy?(): void
}

interface ChannelBuilder {
  build(options: {
    window: Window
    origin: string
    scope: string
    onReady?: (channel: ChannelInstance) => void
  }): ChannelInstance
}

export interface ExerciseRuntimeContext {
  resourceId: string
  title?: string
  paths?: ExerciseRuntimePaths
}

export interface ExerciseBridgeOptions {
  iframe: HTMLIFrameElement
  getContext: () => ExerciseRuntimeContext
  onLog?: (message: string, payload?: unknown) => void
}

interface ParsedMessage {
  type?: string
  event?: string
  action?: string
  method?: string
  id?: string | number
  params?: Record<string, unknown>
  [key: string]: unknown
}

export interface ExerciseRuntimePaths {
  engine?: string
  dp?: string
}

const ENGINE_ROOT_RE = /A5\.ENGINE_ROOT\s*=\s*['"]([^'"]+)['"]/i
const SKIN_URL_RE = /A5\.SKIN_URL\s*=\s*['"]([^'"]+)['"]/i

/**
 * 从练习 HTML 中提取 Engine、Design Pack 和媒体资源基准路径。
 * @param html - 后端处理后的练习 HTML。
 */
export function extractExerciseRuntimePaths(html: string): ExerciseRuntimePaths {
  if (!html) {
    return {}
  }

  const engine = ENGINE_ROOT_RE.exec(html)?.[1]?.trim() ?? ''
  const dp = SKIN_URL_RE.exec(html)?.[1]?.trim() ?? ''
  const paths: ExerciseRuntimePaths = {}

  if (engine) {
    paths.engine = engine
  }
  if (dp) {
    paths.dp = dp
  }

  return paths
}

/** 解析练习运行时可能发送的对象或 JSON 字符串消息。
 * @param data - 原始消息数据。
 */
function parseMessage(data: unknown): ParsedMessage | null {
  if (typeof data === 'string') {
    try {
      const parsed = JSON.parse(data)
      return parsed && typeof parsed === 'object' ? (parsed as ParsedMessage) : null
    } catch {
      return null
    }
  }

  return data && typeof data === 'object' ? (data as ParsedMessage) : null
}

/** 根据当前练习上下文构造兼容旧运行时的初始化载荷。
 * @param context - 当前练习运行上下文。
 */
function buildInitPayload(context: ExerciseRuntimeContext): Record<string, unknown> {
  const payload: Record<string, unknown> = {
    id: context.resourceId || context.title || 'exercise',
    state: {},
    loWithoutControls: false,
    loExternalResultScreen: false,
    loProfile: 'reader',
    sttAudioLang: 'en',
  }

  const engine = context.paths?.engine?.trim()
  const dp = context.paths?.dp?.trim()
  if (engine || dp) {
    payload.paths = {
      ...(engine ? { engine } : {}),
      ...(dp ? { dp } : {}),
    }
  }

  return payload
}

function postRawInitResponse(
  source: MessageEventSource | null,
  targetOrigin: string,
  context: ExerciseRuntimeContext,
  id?: string | number,
  onLog?: (message: string, payload?: unknown) => void,
) {
  if (!source || typeof (source as Window).postMessage !== 'function') return
  const result = buildInitPayload(context)
  onLog?.('bridge: sending init fallback response', { id, result })
  postRawResultResponse(source, targetOrigin, id, result)
}

function postRawResultResponse(
  source: MessageEventSource | null,
  targetOrigin: string,
  id: unknown,
  result: unknown,
) {
  if (!source || typeof (source as Window).postMessage !== 'function') return
  const payload = {
    id,
    result,
  }
  ;(source as Window).postMessage(JSON.stringify(payload), targetOrigin)
}

function postRawReadyResponse(
  source: MessageEventSource | null,
  targetOrigin: string,
  method: string,
) {
  if (!source || typeof (source as Window).postMessage !== 'function') return
  ;(source as Window).postMessage(
    JSON.stringify({
      method,
      params: {
        type: 'publish-reply',
        publish: [{ action: 'bind', method: 'sendMessageToContainer' }],
      },
    }),
    targetOrigin,
  )
}

/** 判断消息来源协议是否在练习桥接允许范围内。
 * @param origin - 消息来源 origin。
 */
function isAllowedFrameOrigin(origin: string): boolean {
  return origin === '' || origin === 'null' || origin.startsWith('eiuasset://')
}

/** 验证消息是否来自目标练习 iframe 且协议受信任。
 * @param event - 浏览器消息事件。
 * @param iframe - 当前目标练习 iframe。
 */
function isFromTargetFrame(event: MessageEvent, iframe: HTMLIFrameElement): boolean {
  const targetWindow = iframe.contentWindow
  return Boolean(
    targetWindow && event.source === targetWindow && isAllowedFrameOrigin(event.origin),
  )
}

/** 选择响应练习 iframe 时使用的最窄目标 origin。
 * @param eventOrigin - 请求消息的 origin。
 * @param iframe - 当前目标练习 iframe。
 */
function resolveTargetOrigin(eventOrigin: string, iframe: HTMLIFrameElement): string {
  if (eventOrigin && eventOrigin !== 'null') {
    return eventOrigin
  }

  try {
    const iframeOrigin = new URL(iframe.src).origin
    if (iframeOrigin && iframeOrigin !== 'null') {
      return iframeOrigin
    }
  } catch {
    // 自定义协议在部分 WebView 中属于不透明源，只能依靠窗口身份校验。
  }

  return '*'
}

/**
 * 建立父页面与练习 iframe 的消息桥接，并返回解绑函数。
 * @param options - iframe、上下文读取器和可选日志回调。
 */
export function createExerciseBridge(options: ExerciseBridgeOptions): () => void {
  let disposed = false
  let channelReady = false
  let channel: ChannelInstance | null = null
  let channelEnabled = false
  let rawMessageCount = 0

  const onLog = options.onLog ?? (() => {})

  /** 处理经过窗口和来源校验后的练习运行时协议消息。 */
  const onMessage = (event: MessageEvent) => {
    const message = parseMessage(event.data)
    if (disposed || !isFromTargetFrame(event, options.iframe)) return
    const targetOrigin = resolveTargetOrigin(event.origin, options.iframe)

    if (rawMessageCount < 60) {
      const raw =
        typeof event.data === 'string'
          ? event.data.slice(0, 200)
          : (() => {
              try {
                return JSON.stringify(event.data).slice(0, 200)
              } catch {
                return String(event.data)
              }
            })()
      onLog('bridge: raw message', {
        origin: event.origin || '',
        raw,
      })
      rawMessageCount += 1
    }

    if (!message) return

    const context = options.getContext()

    // 转发练习运行时内部日志。
    if (message.type === RUNTIME_LOG_TYPE) {
      const level = typeof message.level === 'string' ? message.level : 'debug'
      onLog(`bridge: iframe ${level}`, message.payload)
      return
    }

    // 响应运行时通道就绪消息。
    if (message.method && /::__ready$/.test(message.method) && !channelEnabled) {
      const msgType = message.params?.type
      onLog('bridge: channel fallback ready', { method: message.method, type: msgType })
      if (msgType === 'publish-request') {
        postRawReadyResponse(event.source, targetOrigin, message.method)
      }
      return
    }

    // 兼容简单的握手和初始化消息。
    if (message.type === 'hello' || message.event === 'hello' || message.action === 'init') {
      onLog('bridge: simple handshake request', message)
      ;(event.source as Window | null)?.postMessage(
        {
          type: 'hello-ack',
          id: message.id,
          data: {
            id: context.resourceId || context.title || 'exercise',
            state: {},
          },
        },
        targetOrigin,
      )
      return
    }

    // 处理练习向宿主容器发送的请求。
    if (message.method && /sendMessageToContainer$/.test(message.method)) {
      if (channelReady) return
      onLog('bridge: channel fallback init request', {
        method: message.method,
        id: message.id,
      })
      postRawInitResponse(event.source, targetOrigin, context, message.id, onLog)
      return
    }

    // 处理 Channel 协议的通用方法调用。
    if (message.method && message.id !== undefined && !channelEnabled) {
      onLog('bridge: channel fallback default result for method', {
        method: message.method,
        id: message.id,
      })
      postRawResultResponse(event.source, targetOrigin, message.id, {})
      return
    }
  }

  window.addEventListener('message', onMessage)
  ;(async () => {
    try {
      const iframeWindow = options.iframe.contentWindow
      if (!iframeWindow) return

      // 生产环境的 tauri:// 与 eiuasset:// 存在跨源隔离，直接访问通常会失败，
      // 此时继续使用下面的兼容回退通道。
      const maybeChannel = (window as Window & { Channel?: ChannelBuilder }).Channel
      if (!maybeChannel?.build) {
        onLog('bridge: jschannel unavailable, using fallback', {
          reason: 'Channel.build not found in parent window',
        })
        return
      }

      if (disposed) return

      channel = maybeChannel.build({
        window: iframeWindow,
        origin: resolveTargetOrigin('', options.iframe),
        scope: 'default',
        onReady: () => {
          channelEnabled = true
          channelReady = true
          onLog('bridge: jschannel ready')
        },
      })

      channel.bind('sendMessageToContainer', (_transaction, params) => {
        onLog('bridge: jschannel sendMessageToContainer', params)
        return buildInitPayload(options.getContext())
      })
    } catch (error) {
      onLog('bridge: jschannel unavailable, using fallback', { error: String(error) })
    }
  })()

  return () => {
    disposed = true
    window.removeEventListener('message', onMessage)
    if (channel && typeof channel.destroy === 'function') {
      channel.destroy()
    }
  }
}
