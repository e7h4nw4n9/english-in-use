<script setup lang="ts">
import { ref, onMounted, onUnmounted, computed, watch } from 'vue'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import AppHeader from './components/AppHeader.vue'
import ConfigPage from './components/ConfigPage.vue'
import BookList from './components/BookList.vue'
import ReaderView from './components/ReaderView.vue'
import StudyPlanPage from './components/study-plan/StudyPlanPage.vue'
import StudyStatsPage from './components/study-stats/StudyStatsPage.vue'
import LoadingOverlay from './components/common/loading/LoadingOverlay.vue'
import type { AppInitProgress } from './types'
import { useI18n } from 'vue-i18n'
import { useTheme } from './composables/useTheme'
import { theme } from 'ant-design-vue'
import { BookOutlined, CalendarOutlined, BarChartOutlined } from '@ant-design/icons-vue'
import { useAppStore } from './stores/app'
import { useReaderStore } from './stores/reader'
import { storeToRefs } from 'pinia'

const { t, locale } = useI18n()
const { isDark, setTheme } = useTheme()
const { useToken } = theme
const { token } = useToken()
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
const homeTab = ref<'books' | 'studyPlan' | 'studyStats'>('books')
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

const buildStamp = __DEBUG_FEATURES__ ? __BUILD_STAMP__ : undefined

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
    loadingMessage.value = t(event.payload.message)
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
            <div class="home-tabs" :data-active-tab="homeTab" role="tablist" aria-label="Home tabs">
              <button
                type="button"
                class="home-tab-btn"
                :class="{ active: homeTab === 'books' }"
                :aria-selected="homeTab === 'books'"
                :tabindex="homeTab === 'books' ? 0 : -1"
                @click="homeTab = 'books'"
              >
                <BookOutlined class="home-tab-icon" />
                {{ t('app.homeTabs.books') }}
              </button>
              <button
                type="button"
                class="home-tab-btn"
                :class="{ active: homeTab === 'studyPlan' }"
                :aria-selected="homeTab === 'studyPlan'"
                :tabindex="homeTab === 'studyPlan' ? 0 : -1"
                @click="homeTab = 'studyPlan'"
              >
                <CalendarOutlined class="home-tab-icon" />
                {{ t('app.homeTabs.studyPlan') }}
              </button>
              <button
                type="button"
                class="home-tab-btn"
                :class="{ active: homeTab === 'studyStats' }"
                :aria-selected="homeTab === 'studyStats'"
                :tabindex="homeTab === 'studyStats' ? 0 : -1"
                @click="homeTab = 'studyStats'"
              >
                <BarChartOutlined class="home-tab-icon" />
                {{ t('app.homeTabs.studyStats') }}
              </button>
            </div>

            <div class="home-content-surface">
              <BookList v-if="homeTab === 'books'" />
              <StudyPlanPage v-else-if="homeTab === 'studyPlan'" />
              <StudyStatsPage v-else />
            </div>
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
  position: relative;
  isolation: isolate;
  padding: 12px;
  width: 100%;
  max-width: 100%;
  min-width: 0;
  flex: 1;
  overflow-y: auto;
  overflow-x: hidden;
}

.main-content::before,
.main-content::after {
  content: '';
  position: absolute;
  z-index: -1;
  border-radius: 999px;
  filter: blur(64px);
  pointer-events: none;
  opacity: 0.55;
}

.main-content::before {
  width: 260px;
  height: 260px;
  top: -120px;
  left: -70px;
  background: v-bind('token.colorPrimaryBg');
}

.main-content::after {
  width: 320px;
  height: 320px;
  top: 140px;
  right: -140px;
  background: v-bind('token.colorFillSecondary');
}

.home-tabs {
  position: sticky;
  top: 0;
  z-index: 20;
  display: grid;
  grid-template-columns: repeat(3, minmax(0, 1fr));
  gap: 8px;
  margin-bottom: 18px;
  padding: 6px;
  border: 1px solid color-mix(in srgb, #ffffff 20%, transparent);
  border-radius: 22px;
  background: color-mix(in srgb, v-bind('token.colorBgContainer') 40%, transparent);
  backdrop-filter: blur(24px);
  box-shadow:
    0 12px 24px -22px color-mix(in srgb, v-bind('token.colorText') 20%, transparent),
    inset 0 1px 0 color-mix(in srgb, #ffffff 30%, transparent);
}

.home-tabs::before {
  content: '';
  position: absolute;
  top: 6px;
  bottom: 6px;
  left: 6px;
  width: calc((100% - 28px) / 3);
  border-radius: 16px;
  background: linear-gradient(135deg, v-bind('token.colorPrimary'), v-bind('token.colorInfo'));
  box-shadow: 0 4px 12px color-mix(in srgb, v-bind('token.colorPrimary') 40%, transparent);
  transition: transform 0.3s cubic-bezier(0.34, 1.56, 0.64, 1);
}

.home-tabs[data-active-tab='studyPlan']::before {
  transform: translateX(calc(100% + 8px));
}

.home-tabs[data-active-tab='studyStats']::before {
  transform: translateX(calc((100% + 8px) * 2));
}

.home-tab-btn {
  position: relative;
  z-index: 1;
  border: 0;
  background: transparent;
  color: v-bind('token.colorTextSecondary');
  border-radius: 16px;
  padding: 12px 14px;
  font-size: 14px;
  font-weight: 700;
  cursor: pointer;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  gap: 8px;
  transition: all 0.24s ease;
}

.home-tab-btn:hover {
  color: v-bind('token.colorText');
}

.home-tab-btn.active {
  color: #ffffff;
}

.home-tab-btn:focus-visible {
  outline: 2px solid v-bind('token.colorPrimary');
  outline-offset: 4px;
}

.home-tab-icon {
  font-size: 16px;
}

.home-content-surface {
  min-height: calc(100% - 64px);
  max-width: 100%;
  min-width: 0;
  border: 1px solid color-mix(in srgb, #ffffff 15%, transparent);
  border-radius: 28px;
  background: color-mix(in srgb, v-bind('token.colorBgContainer') 30%, transparent);
  backdrop-filter: blur(12px);
  box-shadow:
    inset 0 1px 0 color-mix(in srgb, #ffffff 20%, transparent),
    0 30px 48px -42px color-mix(in srgb, v-bind('token.colorText') 25%, transparent);
  overflow: hidden;
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
@import url('https://fonts.googleapis.com/css2?family=Plus+Jakarta+Sans:wght@300;400;500;600;700;800&display=swap');

*,
*::before,
*::after {
  box-sizing: border-box;
}

body {
  margin: 0;
  padding: 0;
  overflow: hidden;
  font-family:
    'Plus Jakarta Sans',
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
  background-color: #082f49; /* Deep cyan dark background */
}

html:not(.dark) {
  background-color: #ecfeff; /* Fresh cyan background */
}
</style>
