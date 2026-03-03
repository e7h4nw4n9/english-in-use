/// <reference types="vite/client" />

declare const __BUILD_STAMP__: string
declare const __DEBUG_FEATURES__: boolean

declare module '*.vue' {
  import type { DefineComponent } from 'vue'
  const component: DefineComponent<{}, {}, any>
  export default component
}
