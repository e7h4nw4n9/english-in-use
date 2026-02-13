<script setup lang="ts">
import { getCurrentWindow } from '@tauri-apps/api/window'
import { theme } from 'ant-design-vue'

const { useToken } = theme
const { token } = useToken()

defineProps<{
  title: string
}>()

function startDrag() {
  getCurrentWindow().startDragging()
}
</script>

<template>
  <div
    data-tauri-drag-region
    class="titlebar"
    @mousedown="startDrag"
    :style="{ backgroundColor: token.colorBgContainer }"
  >
    <div class="title-text" :style="{ color: token.colorTextSecondary }">{{ title }}</div>
  </div>
</template>

<style scoped>
.titlebar {
  height: 28px;
  user-select: none;
  display: flex;
  justify-content: center;
  align-items: center;
  width: 100%;
  flex-shrink: 0;
  z-index: 1000;
  cursor: default;
  border-bottom: 1px solid v-bind('token.colorBorderSecondary');
}

.title-text {
  font-weight: 600;
  font-size: 12px;
}
</style>
