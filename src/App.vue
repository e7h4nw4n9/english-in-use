<script setup lang="ts">
import { ref, onMounted, onUnmounted, computed, watch, defineAsyncComponent } from 'vue'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import AppHeader from './components/AppHeader.vue'
import LoadingOverlay from './components/common/loading/LoadingOverlay.vue'
import type { AppInitProgress } from './types'
import { useI18n } from 'vue-i18n'
import { useTheme } from './composables/useTheme'
import { theme } from 'ant-design-vue'
import {
  BookOutlined,
  CalendarOutlined,
  BarChartOutlined,
  SettingOutlined,
} from '@ant-design/icons-vue'
import { useAppStore } from './stores/app'
import { useReaderStore } from './stores/reader'
import { storeToRefs } from 'pinia'

const ConfigPage = defineAsyncComponent(() => import('./components/ConfigPage.vue'))
const BookList = defineAsyncComponent(() => import('./components/BookList.vue'))
const ReaderView = defineAsyncComponent(() => import('./components/ReaderView.vue'))
const StudyPlanPage = defineAsyncComponent(
  () => import('./components/study-plan/StudyPlanPage.vue'),
)
const StudyStatsPage = defineAsyncComponent(
  () => import('./components/study-stats/StudyStatsPage.vue'),
)

type HomeTab = 'books' | 'studyPlan' | 'studyStats' | 'settings'

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
const homeTab = ref<HomeTab>('books')
const homeReloadKey = ref(0)
const shouldReloadHomeAfterConfigChange = ref(false)

const themeTokens = {
  light: {
    colorPrimary: '#2563EB',
    colorInfo: '#0EA5E9',
    colorSuccess: '#16A34A',
    colorWarning: '#D97706',
    colorError: '#DC2626',
    colorBgBase: '#F5F7FB',
    colorBgContainer: '#FFFFFF',
    colorBgElevated: '#FFFFFF',
    colorTextBase: '#111827',
    colorText: '#111827',
    colorTextSecondary: '#4B5563',
    colorBorder: '#D1D5DB',
    colorBorderSecondary: '#E5E7EB',
  },
  dark: {
    colorPrimary: '#60A5FA',
    colorInfo: '#38BDF8',
    colorSuccess: '#4ADE80',
    colorWarning: '#FBBF24',
    colorError: '#FB7185',
    colorBgBase: '#0F172A',
    colorBgContainer: '#1E293B',
    colorBgElevated: '#334155',
    colorTextBase: '#F8FAFC',
    colorText: '#F8FAFC',
    colorTextSecondary: '#94A3B8',
    colorBorder: '#334155',
    colorBorderSecondary: '#1E293B',
  },
}

let unlistenOpenSettings: UnlistenFn | null = null
let unlistenProgress: UnlistenFn | null = null

const appTheme = computed(() => ({
  algorithm: isDark.value ? theme.darkAlgorithm : theme.defaultAlgorithm,
  token: isDark.value ? themeTokens.dark : themeTokens.light,
}))

const currentTitle = computed(() => {
  if (showConfig.value || homeTab.value === 'settings') {
    return t('config.title')
  }
  return t('app.title')
})

const shouldShowHeader = computed(() => {
  return showConfig.value || isUiVisible.value || !currentBook.value
})

const buildStamp = __DEBUG_FEATURES__ ? __BUILD_STAMP__ : undefined

watch(
  config,
  (newConfig) => {
    if (!newConfig) return

    if (newConfig.system.language) {
      locale.value = newConfig.system.language
    }

    if (newConfig.system.theme) {
      setTheme(newConfig.system.theme as any)
    }
  },
  { immediate: true, deep: true },
)

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

  homeTab.value = 'books'
  showConfig.value = false
}

async function onConfigBack(options?: { reloadHome?: boolean }) {
  if (options?.reloadHome) {
    await goHome()
    return
  }

  if (showConfig.value) {
    showConfig.value = false
    return
  }

  homeTab.value = 'books'
}

function activateHomeTab(tab: HomeTab) {
  homeTab.value = tab
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
  <a-config-provider :theme="appTheme">
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

      <main class="app-main-container">
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
            <div class="home-tabs" role="tablist" aria-label="Home tabs">
              <button
                type="button"
                class="home-tab-btn"
                :class="{ active: homeTab === 'books' }"
                :aria-selected="homeTab === 'books'"
                :tabindex="homeTab === 'books' ? 0 : -1"
                @click="activateHomeTab('books')"
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
                @click="activateHomeTab('studyPlan')"
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
                @click="activateHomeTab('studyStats')"
              >
                <BarChartOutlined class="home-tab-icon" />
                {{ t('app.homeTabs.studyStats') }}
              </button>
              <button
                type="button"
                class="home-tab-btn"
                :class="{ active: homeTab === 'settings' }"
                :aria-selected="homeTab === 'settings'"
                :tabindex="homeTab === 'settings' ? 0 : -1"
                @click="activateHomeTab('settings')"
              >
                <SettingOutlined class="home-tab-icon" />
                {{ t('app.homeTabs.settings') }}
              </button>
            </div>

            <div class="home-content-surface">
              <BookList v-if="homeTab === 'books'" />
              <StudyPlanPage v-else-if="homeTab === 'studyPlan'" />
              <StudyStatsPage v-else-if="homeTab === 'studyStats'" />
              <ConfigPage
                v-else
                :initial-config="config || undefined"
                :allow-back="false"
                @config-imported="onConfigImported"
                @config-saved="onConfigSaved"
                @back="onConfigBack"
              />
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
  padding: 12px;
  padding-bottom: 96px; /* Space for the floating dock */
  width: 100%;
  max-width: 100%;
  min-width: 0;
  flex: 1;
  overflow-y: auto;
  overflow-x: hidden;
}

.home-tabs {
  position: fixed;
  bottom: 24px;
  left: 50%;
  transform: translateX(-50%);
  z-index: 100;
  display: flex;
  gap: 4px;
  width: auto;
  min-width: 280px;
  max-width: min(92vw, 520px);
  padding: 6px;
  border: 1px solid color-mix(in srgb, #ffffff 25%, transparent);
  border-radius: 999px;
  background: color-mix(in srgb, v-bind('appTheme.token.colorBgElevated') 65%, transparent);
  backdrop-filter: blur(20px);
  box-shadow:
    0 12px 40px -8px rgba(0, 0, 0, 0.25),
    0 8px 16px -4px rgba(0, 0, 0, 0.15);
}

.dark .home-tabs {
  border: 1px solid color-mix(in srgb, #ffffff 10%, transparent);
  box-shadow: 0 16px 48px -12px rgba(0, 0, 0, 0.5);
}

.home-tab-btn {
  border: 0;
  background: transparent;
  color: v-bind('appTheme.token.colorTextSecondary');
  border-radius: 999px;
  padding: 8px 16px;
  flex: 1;
  font-size: 13px;
  font-weight: 600;
  cursor: pointer;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  gap: 8px;
  transition: all 0.2s cubic-bezier(0.4, 0, 0.2, 1);
  white-space: nowrap;
}

.home-tab-btn:hover {
  background: color-mix(in srgb, v-bind('appTheme.token.colorPrimary') 10%, transparent);
  color: v-bind('appTheme.token.colorPrimary');
}

.home-tab-btn.active {
  color: #ffffff;
  background: v-bind('appTheme.token.colorPrimary');
  box-shadow: 0 4px 12px color-mix(in srgb, v-bind('appTheme.token.colorPrimary') 30%, transparent);
}

.home-tab-btn:focus-visible {
  outline: 2px solid v-bind('appTheme.token.colorPrimary');
  outline-offset: 2px;
}

.home-tab-icon {
  font-size: 16px;
}

.home-content-surface {
  min-height: 100%;
  max-width: 100%;
  min-width: 0;
  border: 1px solid
    color-mix(in srgb, v-bind('appTheme.token.colorBorderSecondary') 80%, transparent);
  border-radius: 14px;
  background: v-bind('appTheme.token.colorBgContainer');
  overflow: hidden;
}

.fade-enter-active,
.fade-leave-active {
  transition:
    opacity 0.2s ease,
    transform 0.2s ease;
}

.fade-enter-from,
.fade-leave-to {
  opacity: 0;
  transform: translateY(4px);
}

.slide-up-enter-active,
.slide-up-leave-active {
  transition: all 0.2s ease;
}

.slide-up-enter-from,
.slide-up-leave-to {
  transform: translateY(-100%);
  opacity: 0;
}

@media (max-width: 880px) {
  .main-content {
    padding: 10px;
    padding-bottom: 88px;
  }

  .home-tabs {
    min-width: auto;
    padding: 5px;
    bottom: 20px;
  }

  .home-tab-btn {
    font-size: 12px;
    padding: 8px 12px;
    gap: 6px;
  }

  .home-tab-icon {
    font-size: 15px;
  }
}

@media (max-width: 640px) {
  .home-tab-btn {
    padding: 10px;
    font-size: 0;
    gap: 0;
  }

  .home-tab-icon {
    font-size: 18px;
  }
}

@supports (-webkit-touch-callout: none) {
  @media (hover: none) and (pointer: coarse) {
    .main-content {
      padding-bottom: 104px;
    }

    .home-tabs {
      bottom: 28px;
      gap: 6px;
      padding: 8px;
    }

    .home-tab-btn {
      padding: 10px 18px;
      font-size: 14px;
      gap: 8px;
    }

    .home-tab-icon {
      font-size: 18px;
    }
  }

  @media (hover: none) and (pointer: coarse) and (max-width: 880px) {
    .main-content {
      padding-bottom: 96px;
    }

    .home-tabs {
      bottom: 24px;
      padding: 7px;
    }

    .home-tab-btn {
      padding: 10px 14px;
      font-size: 13px;
      gap: 7px;
    }

    .home-tab-icon {
      font-size: 17px;
    }
  }

  @media (hover: none) and (pointer: coarse) and (max-width: 640px) {
    .home-tab-btn {
      padding: 11px;
      font-size: 0;
      gap: 0;
    }

    .home-tab-icon {
      font-size: 20px;
    }
  }
}
</style>

<style>
@import url('https://fonts.googleapis.com/css2?family=Plus+Jakarta+Sans:wght@400;500;600;700&display=swap');

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
  background-color: #f5f7fb;
}

html.dark {
  background-color: #0f172a;
  color: #f8fafc;
}
</style>
