import { ref, watch, onUnmounted, computed } from 'vue';

type Theme = 'system' | 'light' | 'dark';

export function useTheme() {
  const currentTheme = ref<Theme>('system');
  const systemDarkMode = window.matchMedia('(prefers-color-scheme: dark)');
  const isSystemDark = ref(systemDarkMode.matches);
  
  const isDark = computed(() => {
    if (currentTheme.value === 'dark') return true;
    if (currentTheme.value === 'light') return false;
    return isSystemDark.value;
  });

  function updateDOM(dark: boolean) {
    if (dark) {
      document.documentElement.classList.add('dark');
    } else {
      document.documentElement.classList.remove('dark');
    }
  }

  function handleSystemChange(e: MediaQueryListEvent) {
    isSystemDark.value = e.matches;
  }

  // Initial listener
  systemDarkMode.addEventListener('change', handleSystemChange);

  onUnmounted(() => {
    systemDarkMode.removeEventListener('change', handleSystemChange);
  });

  // Watch for isDark changes and update DOM
  watch(isDark, (val) => {
    updateDOM(val);
  }, { immediate: true });

  function setTheme(theme: Theme) {
    currentTheme.value = theme;
  }

  return {
    currentTheme,
    isDark,
    setTheme
  };
}
