<script setup lang="ts">
import { ArrowLeftOutlined } from '@ant-design/icons-vue'
import type { DictionaryTipBlock, ParsedDictionaryTip } from '@/features/dictionary/types'

defineProps<{ tip: ParsedDictionaryTip }>()
const emit = defineEmits<{ (event: 'back'): void }>()

/** 返回贴士块对应的语义化 HTML 标签。 */
function blockTag(kind: DictionaryTipBlock['kind']) {
  if (kind === 'heading1') return 'h1'
  if (kind === 'heading2') return 'h2'
  return 'p'
}
</script>

<template>
  <article class="dictionary-tip-detail">
    <header class="tip-toolbar">
      <a-button
        class="operation-button tip-back"
        type="text"
        aria-label="返回词典查询"
        @click="emit('back')"
      >
        <template #icon><ArrowLeftOutlined /></template>
      </a-button>
      <strong>{{ tip.title }}</strong>
      <span aria-hidden="true"></span>
    </header>

    <div class="tip-content">
      <component
        :is="blockTag(block.kind)"
        v-for="(block, blockIndex) in tip.blocks"
        :key="blockIndex"
        :class="`tip-${block.kind}`"
      >
        <template v-for="(fragment, fragmentIndex) in block.fragments" :key="fragmentIndex">
          <span
            :class="{
              bold: fragment.bold,
              italic: fragment.italic,
              chinese: fragment.chinese,
            }"
            >{{ fragment.text }}</span
          >
          <br v-if="fragment.breakAfter" />
        </template>
      </component>
    </div>
  </article>
</template>

<style scoped>
.dictionary-tip-detail {
  min-height: 100%;
  background: var(--ant-color-bg-container);
  color: var(--ant-color-text);
}
.tip-toolbar {
  position: sticky;
  top: 0;
  z-index: 2;
  display: grid;
  grid-template-columns: 44px minmax(0, 1fr) 44px;
  align-items: center;
  min-height: 56px;
  padding: 6px 14px;
  background: var(--ant-color-fill-tertiary);
  text-align: center;
}
.tip-back {
  width: 36px;
}
.tip-content {
  width: min(100%, 1180px);
  margin: 0 auto;
  padding: 28px 32px 56px;
  font-family: Georgia, 'Times New Roman', serif;
  font-size: 18px;
  line-height: 1.65;
}
.tip-heading1 {
  margin: 0 0 20px;
  color: #102a56;
  font-size: 29px;
  font-weight: 400;
}
.tip-heading2 {
  margin: 24px 0 14px;
  color: #102a56;
  font-size: 20px;
  font-weight: 400;
}
.tip-paragraph,
.tip-listItem {
  margin: 0 0 9px;
}
.bold {
  font-weight: 700;
}
.italic {
  font-style: italic;
}
.chinese {
  font-family: system-ui, sans-serif;
}
:global(html.dark .tip-heading1),
:global(html.dark .tip-heading2) {
  color: #93c5fd;
}
@media (max-width: 640px) {
  .tip-toolbar strong {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .tip-content {
    padding: 22px 20px 40px;
    font-size: 16px;
  }
  .tip-heading1 {
    font-size: 24px;
  }
}
</style>
