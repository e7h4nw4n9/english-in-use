<script setup lang="ts">
import { ref } from 'vue'
import type { ParsedDictionaryTip } from '@/features/dictionary/types'
import DictionaryDiscoveryCards from './DictionaryDiscoveryCards.vue'
import DictionaryLookup from './DictionaryLookup.vue'
import DictionaryTipDetail from './DictionaryTipDetail.vue'

const detailVisible = ref(false)
const selectedTip = ref<ParsedDictionaryTip | null>(null)
</script>

<template>
  <div class="dictionary-page">
    <div v-show="!selectedTip">
      <DictionaryLookup @detail-mode-change="detailVisible = $event" />
      <DictionaryDiscoveryCards v-show="!detailVisible" @open-tip="selectedTip = $event" />
    </div>
    <DictionaryTipDetail v-if="selectedTip" :tip="selectedTip" @back="selectedTip = null" />
  </div>
</template>

<style scoped>
.dictionary-page {
  min-height: 100%;
}
</style>
