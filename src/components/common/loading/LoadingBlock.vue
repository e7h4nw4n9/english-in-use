<script setup lang="ts">
import AppLoading from './AppLoading.vue'

interface Props {
  visible?: boolean
  minHeight?: string
  title?: string
  message?: string
}

withDefaults(defineProps<Props>(), {
  visible: true,
  minHeight: '280px',
  title: '',
  message: '',
})
</script>

<template>
  <div
    v-if="visible"
    class="loading-block flex h-full items-center justify-center"
    :style="{ minHeight }"
  >
    <AppLoading :title="title" :message="message" :card="false" />
  </div>
</template>
