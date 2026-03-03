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

export interface PreparedExerciseHtml {
  html: string
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

/**
 * No longer performs actual injection as runtime-guard is now injected by the backend.
 * Kept for signature compatibility.
 */
export function prepareExerciseHtml(inputHtml: string): PreparedExerciseHtml {
  return {
    html: inputHtml,
  }
}

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
  context: ExerciseRuntimeContext,
  id?: string | number,
  onLog?: (message: string, payload?: unknown) => void,
) {
  if (!source || typeof (source as Window).postMessage !== 'function') return
  const result = buildInitPayload(context)
  onLog?.('bridge: sending init fallback response', { id, result })
  postRawResultResponse(source, id, result)
}

function postRawResultResponse(source: MessageEventSource | null, id: unknown, result: unknown) {
  if (!source || typeof (source as Window).postMessage !== 'function') return
  const payload = {
    id,
    result,
  }
  ;(source as Window).postMessage(JSON.stringify(payload), '*')
}

function postRawReadyResponse(source: MessageEventSource | null, method: string) {
  if (!source || typeof (source as Window).postMessage !== 'function') return
  ;(source as Window).postMessage(
    JSON.stringify({
      method,
      params: {
        type: 'publish-reply',
        publish: [{ action: 'bind', method: 'sendMessageToContainer' }],
      },
    }),
    '*',
  )
}

function isHandshakeMessage(message: ParsedMessage | null): boolean {
  if (!message) return false
  if (message.type === 'hello' || message.event === 'hello' || message.action === 'init') {
    return true
  }
  if (!message.method) return false
  return /sendMessageToContainer$/.test(message.method) || /::__ready$/.test(message.method)
}

function isOpaqueOrEmptyOrigin(origin: string): boolean {
  return origin === '' || origin === 'null'
}

function isFromTargetFrame(
  event: MessageEvent,
  iframe: HTMLIFrameElement,
  message: ParsedMessage | null,
): boolean {
  const targetWindow = iframe.contentWindow
  if (!targetWindow) return true
  if (!event.source) return true
  // In cross-origin scenarios with some WebViews, strict equality might fail.
  // We first check known-safe cases and then allow a strict fallback for handshake messages.
  if (event.origin.startsWith('eiuasset://')) return true
  if (event.source === targetWindow) return true
  if (isOpaqueOrEmptyOrigin(event.origin) && isHandshakeMessage(message)) return true
  return false
}

export function createExerciseBridge(options: ExerciseBridgeOptions): () => void {
  let disposed = false
  let channelReady = false
  let channel: ChannelInstance | null = null
  let channelEnabled = false
  let rawMessageCount = 0

  const onLog = options.onLog ?? (() => {})

  const onMessage = (event: MessageEvent) => {
    const message = parseMessage(event.data)
    if (disposed || !isFromTargetFrame(event, options.iframe, message)) return

    if (
      message &&
      event.source &&
      options.iframe.contentWindow &&
      event.source !== options.iframe.contentWindow &&
      isOpaqueOrEmptyOrigin(event.origin) &&
      isHandshakeMessage(message)
    ) {
      onLog('bridge: accepting opaque-origin handshake fallback', {
        origin: event.origin || '',
        type: message.type || message.event || message.action || '',
        method: message.method || '',
      })
    }

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

    // Handle internal runtime logs
    if (message.type === RUNTIME_LOG_TYPE) {
      const level = typeof message.level === 'string' ? message.level : 'debug'
      onLog(`bridge: iframe ${level}`, message.payload)
      return
    }

    // Handle channel ready
    if (message.method && /::__ready$/.test(message.method) && !channelEnabled) {
      const msgType = message.params?.type
      onLog('bridge: channel fallback ready', { method: message.method, type: msgType })
      if (msgType === 'publish-request') {
        postRawReadyResponse(event.source, message.method)
      }
      return
    }

    // Handle simple hello/init
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
        '*',
      )
      return
    }

    // Handle sendMessageToContainer
    if (message.method && /sendMessageToContainer$/.test(message.method)) {
      if (channelReady) return
      onLog('bridge: channel fallback init request', {
        method: message.method,
        id: message.id,
      })
      postRawInitResponse(event.source, context, message.id, onLog)
      return
    }

    // Handle generic method calls
    if (message.method && message.id !== undefined && !channelEnabled) {
      onLog('bridge: channel fallback default result for method', {
        method: message.method,
        id: message.id,
      })
      postRawResultResponse(event.source, message.id, {})
      return
    }
  }

  window.addEventListener('message', onMessage)
  ;(async () => {
    try {
      const iframeWindow = options.iframe.contentWindow
      if (!iframeWindow) return

      // In production (tauri:// origin), this will fail due to cross-origin isolation
      // with eiuasset:// origin. Fallback will be used.
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
        origin: '*',
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
