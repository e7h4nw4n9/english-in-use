<script setup lang="ts">
import { ref } from 'vue'
import DictionaryLookup from '@/components/dictionary/DictionaryLookup.vue'

defineProps<{ word: string }>()
const emit = defineEmits<{ (event: 'close'): void; (event: 'before-audio'): void }>()
const loginOpen = ref(false)
</script>

<template>
  <a-modal
    :open="true"
    title="单词查询"
    :footer="null"
    :keyboard="!loginOpen"
    :mask-closable="!loginOpen"
    :closable="!loginOpen"
    :width="720"
    centered
    @cancel="emit('close')"
  >
    <div class="reader-dictionary-body">
      <DictionaryLookup
        :initial-query="word"
        presentation="reader"
        compact
        @login-change="loginOpen = $event"
        @before-audio="emit('before-audio')"
      />
    </div>
  </a-modal>
</template>

<style scoped>
.reader-dictionary-body {
  max-height: min(70dvh, 800px);
  overflow: auto;
  overscroll-behavior: contain;
}
</style>
