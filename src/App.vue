<script setup lang="ts">
import { ref, onMounted, onUnmounted, computed, watch } from 'vue'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import AppHeader from './components/AppHeader.vue'
import AppFooter from './components/AppFooter.vue'
import ConfigPage from './components/ConfigPage.vue'
import BookList from './components/BookList.vue'
import type { AppInitProgress } from './types'
import { useI18n } from 'vue-i18n'
import { useTheme } from './composables/useTheme'
import { theme } from 'ant-design-vue'
import { useAppStore } from './stores/app'
import { storeToRefs } from 'pinia'

const { t, locale } = useI18n()
const { isDark, setTheme } = useTheme()
const appStore = useAppStore()
const { config, isLoading, loadingMessage, isConfigValid } = storeToRefs(appStore)

const showConfig = ref(false)

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

// Show config if invalid
watch(
  isConfigValid,
  (valid) => {
    if (!valid && !isLoading.value) {
      showConfig.value = true
    }
  },
  { immediate: true },
)

function onConfigSaved() {
  appStore.refreshConfig()
  if (isConfigValid.value) {
    showConfig.value = false
  }
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
      <AppHeader :title="currentTitle" />

      <main class="app-main-container" :style="containerStyle">
        <div v-if="isLoading" class="loading-container">
          <a-spin size="large" :tip="loadingMessage" />
        </div>

        <ConfigPage
          v-else-if="showConfig"
          :initial-config="config || undefined"
          :allow-back="isConfigValid"
          @config-saved="onConfigSaved"
          @back="showConfig = false"
        />

        <div v-else class="main-content">
          <BookList />
        </div>
      </main>

      <AppFooter v-show="config && config.system.enable_auto_check" />
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
  overflow-y: auto;
  position: relative;
}

.loading-container {
  flex: 1;
  display: flex;
  justify-content: center;
  align-items: center;
}

.main-content {
  padding: 0;
  width: 100%;
}
</style>

<style>
body {
  margin: 0;
  padding: 0;
  overflow: hidden;
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
