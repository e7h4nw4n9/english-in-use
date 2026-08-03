;(function () {
  var LOG_TYPE = 'eiu-exercise-runtime-log'
  function postRuntimeLog(level, payload) {
    try {
      if (!window.parent || typeof window.parent.postMessage !== 'function') return
      window.parent.postMessage(
        JSON.stringify({ type: LOG_TYPE, level: level, payload: payload }),
        '*',
      )
    } catch (e) {}
  }

  // 1. 代理 Console 并捕获错误
  ;(function ProxyConsoleAndErrors() {
    var levels = ['log', 'info', 'warn', 'error']
    levels.forEach(function (level) {
      var original = console[level]
      console[level] = function () {
        var args = Array.prototype.slice.call(arguments)
        postRuntimeLog(level === 'log' ? 'info' : level, {
          message: 'iframe-console',
          args: args.map(function (a) {
            try {
              return typeof a === 'object' ? JSON.stringify(a).slice(0, 200) : String(a)
            } catch (e) {
              return '[unserializable]'
            }
          }),
        })
        if (typeof original === 'function') original.apply(console, arguments)
      }
    })
    window.onerror = function (msg, url, line, col, error) {
      postRuntimeLog('error', {
        message: 'iframe-window-error',
        details: msg,
        line: line,
        col: col,
        stack: error ? error.stack : '',
      })
    }
    window.onunhandledrejection = function (event) {
      postRuntimeLog('error', {
        message: 'iframe-unhandled-rejection',
        reason: String(event.reason),
      })
    }
  })()

  // 2. URI 容错补丁 (解决 URIError: URI error)
  ;(function PatchURI() {
    var fnNames = ['decodeURIComponent', 'decodeURI']
    fnNames.forEach(function (fnName) {
      var originalFn = window[fnName]
      if (typeof originalFn !== 'function') return
      window[fnName] = function (value) {
        try {
          return originalFn.call(window, value)
        } catch (e) {
          postRuntimeLog('warn', {
            message: 'uri-decode-fallback',
            fn: fnName,
            value: String(value).slice(0, 100),
          })
          return String(value)
        }
      }
    })
  })()

  // 3. 布局修复
  function ensureLayout() {
    try {
      var html = document.documentElement
      var body = document.body
      if (!html || !body) return
      html.style.height = '100%'
      html.style.minHeight = '100%'
      body.style.height = '100%'
      body.style.minHeight = '100%'
      body.style.margin = '0'
      body.style.padding = '0'
      body.style.overflow = 'hidden'
      body.style.backgroundColor = 'transparent'

      window.dispatchEvent(new Event('resize'))

      postRuntimeLog('info', {
        message: 'layout-patched',
        bodyHeight: body.clientHeight,
        bodyWidth: body.clientWidth,
      })
    } catch (e) {
      postRuntimeLog('error', { message: 'layout-patch-failed', error: String(e) })
    }
  }

  window.__eiuRuntimeGuardStarted = true
  if (document.readyState === 'loading') {
    document.addEventListener('DOMContentLoaded', ensureLayout)
  } else {
    ensureLayout()
  }
  window.addEventListener('load', function () {
    ensureLayout()
    setTimeout(ensureLayout, 500)
    setTimeout(ensureLayout, 2000)
  })
})()
