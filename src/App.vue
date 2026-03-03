<script setup lang="ts">
import { ref, onMounted, onUnmounted, computed, watch } from 'vue'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import AppHeader from './components/AppHeader.vue'
import ConfigPage from './components/ConfigPage.vue'
import BookList from './components/BookList.vue'
import ReaderView from './components/ReaderView.vue'
import LoadingOverlay from './components/common/loading/LoadingOverlay.vue'
import type { AppInitProgress } from './types'
import { useI18n } from 'vue-i18n'
import { useTheme } from './composables/useTheme'
import { theme } from 'ant-design-vue'
import { useAppStore } from './stores/app'
import { useReaderStore } from './stores/reader'
import { storeToRefs } from 'pinia'

const { t, locale } = useI18n()
const { isDark, setTheme } = useTheme()
const appStore = useAppStore()
const readerStore = useReaderStore()
const {
  config,
  isLoading,
  loadingMessage,
  globalLoading,
  globalLoadingMessage,
  globalLoadingProgress,
  isConfigValid,
  currentBook,
} = storeToRefs(appStore)
const { isUiVisible } = storeToRefs(readerStore)

const showConfig = ref(false)
const homeReloadKey = ref(0)
const shouldReloadHomeAfterConfigChange = ref(false)

let unlistenOpenSettings: UnlistenFn | null = null
let unlistenProgress: UnlistenFn | null = null

// Ant Design theme configuration
const algorithm = computed(() => {
  return isDark.value ? theme.darkAlgorithm : theme.defaultAlgorithm
})

// Compute padding for main container based on footer visibility
const containerStyle = computed(() => {
  return {} // Flex layout handles this now
})

// Compute current title for the custom header
const currentTitle = computed(() => {
  const titleKey = showConfig.value ? 'config.title' : 'app.title'
  return t(titleKey)
})

const shouldShowHeader = computed(() => {
  // Config page should always keep the app header visible
  return showConfig.value || isUiVisible.value || !currentBook.value
})

const buildStamp = __BUILD_STAMP__

// Apply settings from config whenever it changes
watch(
  config,
  (newConfig) => {
    if (newConfig) {
      if (newConfig.system.language) {
        locale.value = newConfig.system.language
      }
      if (newConfig.system.theme) {
        setTheme(newConfig.system.theme as any)
      }
    }
  },
  { immediate: true, deep: true },
)

// Show config if invalid when loading finishes
watch(
  [isConfigValid, isLoading],
  ([valid, loading]) => {
    if (!valid && !loading) {
      showConfig.value = true
    }
  },
  { immediate: true },
)

function onConfigSaved() {
  shouldReloadHomeAfterConfigChange.value = true
}

function onConfigImported() {
  shouldReloadHomeAfterConfigChange.value = true
}

async function goHome() {
  if (shouldReloadHomeAfterConfigChange.value) {
    await appStore.refreshConfig()
    homeReloadKey.value += 1
    shouldReloadHomeAfterConfigChange.value = false
  }

  if (currentBook.value) {
    currentBook.value = null
  }
  if (showConfig.value) {
    showConfig.value = false
  }
}

async function onConfigBack(options?: { reloadHome?: boolean }) {
  if (options?.reloadHome) {
    await goHome()
    return
  }
  showConfig.value = false
}

onMounted(async () => {
  unlistenProgress = await listen<AppInitProgress>('init-progress', (event) => {
    loadingMessage.value = event.payload.message
  })

  await appStore.initApp()

  unlistenOpenSettings = await listen('open-settings', () => {
    showConfig.value = true
  })
})

onUnmounted(() => {
  if (unlistenOpenSettings) unlistenOpenSettings()
  if (unlistenProgress) unlistenProgress()
})
</script>

<template>
  <a-config-provider :theme="{ algorithm }">
    <div class="app-layout">
      <Transition name="slide-up">
        <AppHeader
          v-show="shouldShowHeader"
          :title="currentTitle"
          :build-stamp="buildStamp"
          :is-sub-page="showConfig"
          @home="goHome"
        />
      </Transition>

      <main class="app-main-container" :style="containerStyle">
        <Transition name="fade" mode="out-in">
          <LoadingOverlay
            v-if="isLoading"
            :title="t('app.loading')"
            :message="loadingMessage"
            tone="soft"
            backdrop="soft"
          />

          <ConfigPage
            v-else-if="showConfig"
            :initial-config="config || undefined"
            :allow-back="isConfigValid"
            @config-imported="onConfigImported"
            @config-saved="onConfigSaved"
            @back="onConfigBack"
          />

          <ReaderView v-else-if="currentBook" />

          <div v-else :key="homeReloadKey" class="main-content">
            <BookList />
          </div>
        </Transition>
      </main>

      <Transition name="fade">
        <LoadingOverlay
          v-if="globalLoading && !isLoading"
          fullscreen
          :z-index="1200"
          :title="t('app.loading')"
          :message="globalLoadingMessage"
          :progress="globalLoadingProgress"
          tone="strong"
          backdrop="strong"
        />
      </Transition>
    </div>
  </a-config-provider>
</template>

<style scoped>
.app-layout {
  width: 100%;
  height: 100vh;
  display: flex;
  flex-direction: column;
  background-color: transparent;
  overflow: hidden;
}

.app-main-container {
  margin: 0;
  padding: 0;
  flex: 1;
  display: flex;
  flex-direction: column;
  width: 100%;
  overflow: hidden;
  position: relative;
}

.main-content {
  padding: 0;
  width: 100%;
  flex: 1;
  overflow-y: auto;
}

/* Transitions */
.fade-enter-active,
.fade-leave-active {
  transition:
    opacity 0.4s cubic-bezier(0.4, 0, 0.2, 1),
    transform 0.4s cubic-bezier(0.4, 0, 0.2, 1);
}

.fade-enter-from,
.fade-leave-to {
  opacity: 0;
  transform: translateY(10px);
}

.slide-up-enter-active,
.slide-up-leave-active {
  transition: all 0.3s ease;
}

.slide-up-enter-from,
.slide-up-leave-to {
  transform: translateY(-100%);
  opacity: 0;
}

.slide-down-enter-active,
.slide-down-leave-active {
  transition: all 0.3s ease;
}

.slide-down-enter-from,
.slide-down-leave-to {
  transform: translateY(100%);
  opacity: 0;
}
</style>

<style>
@import url('https://fonts.googleapis.com/css2?family=Inter:wght@300;400;500;600;700;800;900&display=swap');

body {
  margin: 0;
  padding: 0;
  overflow: hidden;
  font-family:
    'Inter',
    system-ui,
    -apple-system,
    sans-serif;
  -webkit-font-smoothing: antialiased;
  -moz-osx-font-smoothing: grayscale;
}

html,
body,
#app {
  height: 100%;
  width: 100%;
}

html {
  transition: background-color 0.3s ease;
}

html.dark {
  background-color: #141414;
}

html:not(.dark) {
  background-color: #ffffff;
}
</style>
