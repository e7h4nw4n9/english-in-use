<script setup lang="ts">
import { ref, onMounted, onUnmounted, computed } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import AppHeader from "./components/AppHeader.vue";
import ConfigPage from "./components/ConfigPage.vue";
import type { AppConfig } from "./types";
import { useI18n } from 'vue-i18n';
import { useTheme } from './composables/useTheme';
import { theme } from 'ant-design-vue';

const { t, locale } = useI18n();
const { currentTheme, isDark, setTheme } = useTheme();

const isLoading = ref(true);
const showConfig = ref(false);
const appConfig = ref<AppConfig | null>(null);

const greetMsg = ref("");
const name = ref("");

let unlistenOpenSettings: UnlistenFn | null = null;

// Ant Design theme configuration
const algorithm = computed(() => {
  return isDark.value ? theme.darkAlgorithm : theme.defaultAlgorithm;
});

// Compute current title for the custom header
const currentTitle = computed(() => {
  const titleKey = showConfig.value ? 'config.title' : 'app.title';
  return t(titleKey);
});

async function loadConfiguration() {
  try {
    const config = await invoke<AppConfig>("load_config");
    appConfig.value = config;
    
    // Set language from config if available
    if (config.system && config.system.language) {
      locale.value = config.system.language;
    }
    
    // Set theme from config
    if (config.system && config.system.theme) {
      setTheme(config.system.theme as any);
    }

    if (!config.book_source) {
      showConfig.value = true;
    } else {
      showConfig.value = false;
    }
  } catch (e) {
    console.error("Failed to load config:", e);
    showConfig.value = true;
  } finally {
    isLoading.value = false;
  }
}

function onConfigSaved(newConfig: AppConfig) {
  appConfig.value = newConfig;
  if (newConfig.system && newConfig.system.language) {
    locale.value = newConfig.system.language;
  }
  if (newConfig.system && newConfig.system.theme) {
    setTheme(newConfig.system.theme as any);
  }
  showConfig.value = false;
}

async function greet() {
  greetMsg.value = await invoke("greet", { name: name.value });
}

onMounted(async () => {
  loadConfiguration();
  
  unlistenOpenSettings = await listen("open-settings", () => {
    showConfig.value = true;
  });
});

onUnmounted(() => {
  if (unlistenOpenSettings) {
    unlistenOpenSettings();
  }
});
</script>

<template>
  <a-config-provider :theme="{ algorithm }">
    <div class="app-layout">
      <AppHeader :title="currentTitle" />
      
      <main class="container">
        <div v-if="isLoading" class="loading-container">
          <a-spin size="large" :tip="t('app.loading')" />
        </div>

        <ConfigPage 
          v-else-if="showConfig" 
          :initial-config="appConfig || undefined"
          @config-saved="onConfigSaved" 
          @back="showConfig = false"
        />

        <div v-else class="main-content">
          <a-typography-paragraph>
            {{ t('app.bookSourceConfigured', { type: appConfig?.book_source?.type }) }}
          </a-typography-paragraph>
          
          <div v-if="appConfig?.book_source?.type === 'Local'" class="info-item">
            <a-tag color="blue">{{ t('app.path') }}</a-tag>
            <a-typography-text code>{{ appConfig.book_source.details.path }}</a-typography-text>
          </div>
          <div v-else-if="appConfig?.book_source?.type === 'CloudflareR2'" class="info-item">
            <a-tag color="orange">{{ t('app.bucket') }}</a-tag>
            <a-typography-text code>{{ appConfig.book_source.details.bucket_name }}</a-typography-text>
          </div>

          <div class="logo-row">
            <a href="https://vite.dev" target="_blank">
              <img src="/vite.svg" class="logo vite" alt="Vite logo" />
            </a>
            <a href="https://tauri.app" target="_blank">
              <img src="/tauri.svg" class="logo tauri" alt="Tauri logo" />
            </a>
            <a href="https://vuejs.org/" target="_blank">
              <img src="./assets/vue.svg" class="logo vue" alt="Vue logo" />
            </a>
          </div>
          
          <div class="greet-row">
            <a-input-search
              v-model:value="name"
              :placeholder="t('app.greetPlaceholder') || 'Enter a name...'"
              enter-button
              @search="greet"
              class="greet-input"
            >
              <template #enterButton>
                <a-button type="primary">{{ t('app.greetButton') }}</a-button>
              </template>
            </a-input-search>
          </div>
          
          <a-typography-title v-if="greetMsg" :level="4" class="greet-msg">
            {{ greetMsg }}
          </a-typography-title>
        </div>
      </main>
    </div>
  </a-config-provider>
</template>

<style scoped>
.app-layout {
  width: 100%;
  min-height: 100vh;
  background-color: transparent;
}

.container {
  margin: 0;
  padding-top: 28px; 
  display: flex;
  flex-direction: column;
  min-height: 100vh;
  width: 100%;
}

.loading-container {
  display: flex;
  justify-content: center;
  align-items: center;
  height: calc(100vh - 28px);
}

.main-content {
  padding: 2rem;
  text-align: center;
}

.logo-row {
  display: flex;
  justify-content: center;
  margin: 2rem 0;
}

.logo {
  height: 6em;
  padding: 1.5em;
  will-change: filter;
  transition: 0.75s;
}

.logo.vite:hover {
  filter: drop-shadow(0 0 2em #747bff);
}

.logo.tauri:hover {
  filter: drop-shadow(0 0 2em #24c8db);
}

.logo.vue:hover {
  filter: drop-shadow(0 0 2em #249b73);
}

.greet-row {
  display: flex;
  justify-content: center;
  margin-bottom: 1.5rem;
}

.greet-input {
  max-width: 400px;
}

.greet-msg {
  margin-top: 1rem;
}

.info-item {
  margin: 0.5rem 0;
}

.edit-config-btn {
  margin-top: 3rem;
}
</style>

<style>
body {
  margin: 0;
  padding: 0;
  overflow: hidden;
}

/* Ensure background color transition */
html, body, #app {
  height: 100%;
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