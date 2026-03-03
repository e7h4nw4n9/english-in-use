<script setup lang="ts">
import { computed } from 'vue'
import { SyncOutlined } from '@ant-design/icons-vue'

type LoadingTone = 'soft' | 'strong'

interface Props {
  title?: string
  message?: string
  progress?: number | null
  card?: boolean
  tone?: LoadingTone
  spinSize?: 'small' | 'default' | 'large'
}

const props = withDefaults(defineProps<Props>(), {
  title: '',
  message: '',
  progress: null,
  card: true,
  tone: 'soft',
  spinSize: 'large',
})

const normalizedProgress = computed(() => {
  if (props.progress === null || Number.isNaN(props.progress)) return null
  return Math.max(0, Math.min(100, props.progress))
})

const cardToneClass = computed(() => {
  if (props.tone === 'strong') return 'bg-white/70 dark:bg-black/35'
  return 'bg-white/50 dark:bg-black/20'
})
</script>

<template>
  <div
    :class="
      card
        ? `app-loading-card flex flex-col items-center gap-6 rounded-3xl p-12 backdrop-blur-xl ${cardToneClass}`
        : 'app-loading-inline flex flex-col items-center gap-3'
    "
  >
    <a-spin :size="spinSize">
      <template #indicator>
        <SyncOutlined spin style="font-size: 32px" />
      </template>
    </a-spin>

    <div v-if="title || message" class="flex flex-col items-center gap-1">
      <span v-if="title" class="text-sm font-bold uppercase tracking-widest text-blue-500">
        {{ title }}
      </span>
      <span v-if="message" class="text-xs font-medium text-gray-400 dark:text-gray-500">
        {{ message }}
      </span>
    </div>

    <a-progress
      v-if="normalizedProgress !== null"
      class="w-72"
      size="small"
      :percent="Math.round(normalizedProgress)"
      :show-info="true"
      status="active"
    />
  </div>
</template>

<style scoped>
.app-loading-card {
  box-shadow: 0 20px 50px rgba(0, 0, 0, 0.05);
  border: 1px solid rgba(255, 255, 255, 0.2);
}
</style>
