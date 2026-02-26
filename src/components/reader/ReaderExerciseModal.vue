<script setup lang="ts">
import { storeToRefs } from 'pinia'
import { useReaderStore } from '../../stores/reader'

const readerStore = useReaderStore()
const { exerciseVisible, currentExerciseUrl, currentExerciseTitle } = storeToRefs(readerStore)
</script>

<template>
  <a-modal
    v-model:open="exerciseVisible"
    :title="currentExerciseTitle"
    width="90%"
    :footer="null"
    destroy-on-close
  >
    <div class="h-[80vh] w-full overflow-hidden rounded-lg bg-white">
      <iframe
        v-if="currentExerciseUrl"
        :src="currentExerciseUrl"
        class="h-full w-full border-none"
        allow="autoplay"
      ></iframe>
    </div>
  </a-modal>
</template>
