/// <reference types="vitest" />
import { defineConfig } from 'vite'
import vue from '@vitejs/plugin-vue'
import path from 'path'

// @ts-expect-error process is a nodejs global
const host = process.env.TAURI_DEV_HOST

function getUtc8BuildStamp(): string {
  const formatter = new Intl.DateTimeFormat('en-GB', {
    timeZone: 'Asia/Shanghai',
    year: 'numeric',
    month: '2-digit',
    day: '2-digit',
    hour: '2-digit',
    minute: '2-digit',
    second: '2-digit',
    hour12: false,
  })
  const parts = formatter.formatToParts(new Date())
  const byType = Object.fromEntries(parts.map((part) => [part.type, part.value]))
  return `${byType.year}${byType.month}${byType.day}-${byType.hour}${byType.minute}${byType.second}`
}

const buildStamp = getUtc8BuildStamp()

function parseBooleanEnv(value: string | undefined): boolean | undefined {
  if (value == null) {
    return undefined
  }
  const normalized = value.trim().toLowerCase()
  if (['1', 'true', 'yes', 'on'].includes(normalized)) {
    return true
  }
  if (['0', 'false', 'no', 'off'].includes(normalized)) {
    return false
  }
  return undefined
}

// https://vite.dev/config/
export default defineConfig(async ({ command }) => {
  // @ts-expect-error process is a nodejs global
  const rawDebugFeatures = process.env.VITE_ENABLE_DEBUG_FEATURES as string | undefined
  const parsedDebugFeatures = parseBooleanEnv(rawDebugFeatures)
  const debugFeaturesEnabled = parsedDebugFeatures ?? command !== 'build'

  return {
    plugins: [vue()],
    define: {
      __BUILD_STAMP__: JSON.stringify(buildStamp),
      __DEBUG_FEATURES__: JSON.stringify(debugFeaturesEnabled),
    },
    resolve: {
      alias: {
        '@': path.resolve(__dirname, './src'),
      },
    },
    test: {
      environment: 'jsdom',
      globals: true,
    },

    // Vite options tailored for Tauri development and only applied in `tauri dev` or `tauri build`
    //
    // 1. prevent Vite from obscuring rust errors
    clearScreen: false,
    // 2. tauri expects a fixed port, fail if that port is not available
    server: {
      port: 1420,
      strictPort: true,
      host: host || false,
      hmr: host
        ? {
            protocol: 'ws',
            host,
            port: 1421,
          }
        : undefined,
      watch: {
        // 3. tell Vite to ignore watching `src-tauri`
        ignored: ['**/src-tauri/**'],
      },
    },
  }
})
