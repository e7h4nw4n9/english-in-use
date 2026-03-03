<script setup lang="ts">
import { computed } from 'vue'
import AppLoading from './AppLoading.vue'

type OverlayMode = 'card' | 'inline'
type LoadingTone = 'soft' | 'strong'
type BackdropTone = 'none' | 'soft' | 'strong'

interface Props {
  visible?: boolean
  fullscreen?: boolean
  title?: string
  message?: string
  progress?: number | null
  mode?: OverlayMode
  tone?: LoadingTone
  backdrop?: BackdropTone
  zIndex?: number
}

const props = withDefaults(defineProps<Props>(), {
  visible: true,
  fullscreen: false,
  title: '',
  message: '',
  progress: null,
  mode: 'card',
  tone: 'soft',
  backdrop: 'soft',
  zIndex: undefined,
})

const overlayClasses = computed(() => {
  return [
    'loading-overlay',
    props.fullscreen ? 'is-fullscreen' : 'is-contained',
    props.backdrop === 'soft' ? 'has-backdrop-soft' : '',
    props.backdrop === 'strong' ? 'has-backdrop-strong' : '',
  ]
})

const overlayStyle = computed(() => {
  const defaultZIndex = props.fullscreen ? 1100 : 40
  return {
    zIndex: props.zIndex ?? defaultZIndex,
  }
})
</script>

<template>
  <div v-if="visible" :class="overlayClasses" :style="overlayStyle">
    <AppLoading
      :title="title"
      :message="message"
      :progress="progress"
      :card="mode === 'card'"
      :tone="tone"
    />
  </div>
</template>

<style scoped>
.loading-overlay {
  inset: 0;
  display: flex;
  justify-content: center;
  align-items: center;
}

.is-fullscreen {
  position: fixed;
}

.is-contained {
  position: absolute;
}

.has-backdrop-soft {
  background: radial-gradient(circle at center, rgba(59, 130, 246, 0.05) 0%, transparent 70%);
}

.has-backdrop-strong {
  background: radial-gradient(circle at center, rgba(59, 130, 246, 0.08) 0%, transparent 70%);
}
</style>
