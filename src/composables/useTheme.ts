import { ref, watch, onUnmounted, computed } from 'vue'

type Theme = 'system' | 'light' | 'dark'

/** 提供主题状态读取、应用和持久化能力。 */
export function useTheme() {
  const currentTheme = ref<Theme>('system')
  const systemDarkMode = window.matchMedia('(prefers-color-scheme: dark)')
  const isSystemDark = ref(systemDarkMode.matches)

  const isDark = computed(() => {
    if (currentTheme.value === 'dark') return true
    if (currentTheme.value === 'light') return false
    return isSystemDark.value
  })

  function updateDOM(dark: boolean) {
    if (dark) {
      document.documentElement.classList.add('dark')
    } else {
      document.documentElement.classList.remove('dark')
    }
  }

  function handleSystemChange(e: MediaQueryListEvent) {
    isSystemDark.value = e.matches
  }

  // 监听系统主题的初始变化。
  systemDarkMode.addEventListener('change', handleSystemChange)

  onUnmounted(() => {
    systemDarkMode.removeEventListener('change', handleSystemChange)
  })

  // 响应主题状态变化并同步更新 DOM。
  watch(
    isDark,
    (val) => {
      updateDOM(val)
    },
    { immediate: true },
  )

  function setTheme(theme: Theme) {
    currentTheme.value = theme
  }

  return {
    currentTheme,
    isDark,
    setTheme,
  }
}
