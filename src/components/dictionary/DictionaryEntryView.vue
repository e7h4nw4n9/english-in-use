<script setup lang="ts">
import { computed, onUnmounted, ref, watch } from 'vue'
import { message, theme } from 'ant-design-vue'
import { SoundOutlined } from '@ant-design/icons-vue'
import { getDictionaryAudio } from '@/lib/api'
import type { ParsedDictionaryEntry } from '@/features/dictionary/types'

const props = withDefaults(
  defineProps<{ entry: ParsedDictionaryEntry; presentation?: 'full' | 'reader' }>(),
  { presentation: 'full' },
)
const showExamples = ref(props.presentation === 'full')
const hasExamples = computed(() =>
  props.entry.partOfSpeechGroups.some((group) =>
    group.sections
      .filter((section) => section.key === 'definitions')
      .some((section) =>
        section.items.some((item) => item.senses.some((sense) => sense.examples.length)),
      ),
  ),
)
const emit = defineEmits<{ (event: 'before-audio'): void }>()
const playingKey = ref('')
const activeGroupKey = ref('')
const { token } = theme.useToken()
const tabTextColor = computed(() => token.value.colorText)
let audio: HTMLAudioElement | null = null
let objectUrl = ''
let audioRequestVersion = 0

/** 停止旧词条的播放并淘汰尚未完成的音频请求。 */
function stopAudio() {
  audioRequestVersion++
  audio?.pause()
  audio = null
  if (objectUrl) URL.revokeObjectURL(objectUrl)
  objectUrl = ''
  playingKey.value = ''
}

const activeGroup = computed(
  () =>
    props.entry.partOfSpeechGroups.find((group) => group.key === activeGroupKey.value) ||
    props.entry.partOfSpeechGroups[0],
)
watch(
  () => props.entry.id,
  () => {
    stopAudio()
    showExamples.value = props.presentation === 'full'
    activeGroupKey.value = props.entry.partOfSpeechGroups[0]?.key || ''
  },
  { immediate: true },
)

function badgeColor(badge: string) {
  const colors: Record<string, string> = {
    A1: 'green',
    A2: 'cyan',
    B1: 'blue',
    B2: 'geekblue',
    C1: 'purple',
    C2: 'magenta',
    CET4: 'orange',
    CET6: 'volcano',
    NETM: 'geekblue',
  }
  return colors[badge] || 'default'
}

/** 将完整词性转换为词典 Tab 使用的标准缩写。 */
function partOfSpeechTabLabel(label: string) {
  const labels: Record<string, string> = {
    noun: 'n.',
    'n.': 'n.',
    verb: 'v.',
    'v.': 'v.',
    adjective: 'adj.',
    'adj.': 'adj.',
    adverb: 'adv.',
    'adv.': 'adv.',
    preposition: 'prep.',
    'prep.': 'prep.',
    pronoun: 'pron.',
    'pron.': 'pron.',
    conjunction: 'conj.',
    'conj.': 'conj.',
    determiner: 'det.',
    'det.': 'det.',
    interjection: 'interj.',
    'modal verb': 'modal v.',
  }
  return label
    .split(/\s*\/\s*/)
    .map((item) => labels[item.toLowerCase()] || item)
    .join(' / ')
}

/** 判断释义是否包含可展示的正文，避免为只有例句的空释义生成编号。 */
function hasDefinitionText(english: string, chinese: string) {
  return Boolean(english.trim() || chinese.trim())
}

/** 播放由后端白名单代理取得的词典音频。 */
async function playAudio(kind: 'word' | 'example', name: string, key: string) {
  if (!name) return
  stopAudio()
  const requestVersion = audioRequestVersion
  emit('before-audio')
  playingKey.value = key
  try {
    const response = await getDictionaryAudio(kind, name)
    if (requestVersion !== audioRequestVersion) return
    objectUrl = URL.createObjectURL(
      new Blob([new Uint8Array(response.bytes)], { type: response.mimeType }),
    )
    audio = new Audio(objectUrl)
    audio.addEventListener('ended', () => (playingKey.value = ''), { once: true })
    await audio.play()
  } catch (error) {
    if (requestVersion !== audioRequestVersion) return
    playingKey.value = ''
    message.error(String(error))
  }
}

watch(activeGroupKey, stopAudio)
watch(showExamples, (visible) => {
  if (!visible) stopAudio()
})
onUnmounted(stopAudio)
</script>

<template>
  <article class="dictionary-entry">
    <div v-if="$slots.actions || (presentation === 'reader' && hasExamples)" class="entry-actions">
      <slot name="actions" />
      <a-button
        v-if="presentation === 'reader' && hasExamples"
        class="examples-toggle"
        :aria-pressed="showExamples"
        @click="showExamples = !showExamples"
      >
        {{ showExamples ? '隐藏例句' : '显示例句' }}
      </a-button>
    </div>
    <header class="entry-header">
      <div class="headword-row">
        <h1>{{ entry.word }}</h1>
        <a-tag v-for="badge in activeGroup?.badges || []" :key="badge" :color="badgeColor(badge)">{{
          badge
        }}</a-tag>
      </div>
      <div v-if="activeGroup?.pronunciations.length" class="pronunciations">
        <button
          v-for="pronunciation in activeGroup.pronunciations"
          :key="`${pronunciation.region}-${pronunciation.audio}`"
          type="button"
          class="pronunciation"
          :aria-label="`播放 ${pronunciation.region} 发音`"
          @click="playAudio('word', pronunciation.audio, pronunciation.audio)"
        >
          <span>{{ pronunciation.region }}</span>
          <span>/{{ pronunciation.phonetic }}/</span>
          <SoundOutlined :class="{ playing: playingKey === pronunciation.audio }" />
        </button>
      </div>
    </header>

    <a-tabs
      v-model:active-key="activeGroupKey"
      class="part-of-speech-tabs"
      :class="{ 'single-part-of-speech': entry.partOfSpeechGroups.length <= 1 }"
      :destroy-inactive-tab-pane="true"
      :animated="{ inkBar: true, tabPane: false }"
    >
      <a-tab-pane v-for="group in entry.partOfSpeechGroups" :key="group.key">
        <template #tab>
          <span class="part-of-speech-tab-label">{{ partOfSpeechTabLabel(group.label) }}</span>
        </template>

        <div class="entry-content-layout">
          <div class="entry-sections">
            <a-tag class="part-of-speech-tag">{{ group.label }}</a-tag>
            <div
              v-for="section in group.sections.filter(
                (section) => presentation === 'full' || section.key === 'definitions',
              )"
              :key="section.key"
              class="content-section"
            >
              <h2 v-if="section.key !== 'definitions'" class="content-section-title">
                {{ section.title }}
              </h2>

              <section v-for="item in section.items" :key="item.id" class="section-item">
                <div v-if="item.heading" class="section-item-heading">
                  <h3>{{ item.heading }}</h3>
                  <a-tag
                    v-for="partOfSpeech in item.partsOfSpeech"
                    :key="partOfSpeech"
                    class="subentry-part-of-speech-tag"
                  >
                    {{ partOfSpeech }}
                  </a-tag>
                </div>
                <div v-if="item.pronunciations.length" class="subentry-pronunciations">
                  <button
                    v-for="pronunciation in item.pronunciations"
                    :key="`${pronunciation.region}-${pronunciation.audio}`"
                    type="button"
                    class="pronunciation"
                    :aria-label="`播放 ${item.heading} ${pronunciation.region} 发音`"
                    @click="playAudio('word', pronunciation.audio, pronunciation.audio)"
                  >
                    <span>{{ pronunciation.region }}</span>
                    <span>/{{ pronunciation.phonetic }}/</span>
                    <SoundOutlined :class="{ playing: playingKey === pronunciation.audio }" />
                  </button>
                </div>
                <p v-if="item.text" class="section-text">{{ item.text }}</p>

                <section v-for="(sense, senseIndex) in item.senses" :key="sense.id" class="sense">
                  <h3 v-if="sense.sectionTitle" class="sense-section-title">
                    {{ sense.sectionTitle }}
                  </h3>
                  <div v-if="hasDefinitionText(sense.english, sense.chinese)" class="definition">
                    <span class="sense-number">{{ sense.marker || `${senseIndex + 1}.` }}</span>
                    <span v-if="sense.grammar.length" class="grammar-tags">
                      <a-tag v-for="grammar in sense.grammar" :key="grammar" class="grammar-tag">
                        {{ grammar }}
                      </a-tag>
                    </span>
                    <span class="definition-text">{{ sense.english }}</span>
                    <span class="definition-text chinese">{{ sense.chinese }}</span>
                  </div>
                  <aside v-if="sense.notes && presentation === 'full'" class="sense-notes">
                    {{ sense.notes }}
                  </aside>
                  <div
                    v-for="(example, exampleIndex) in showExamples ? sense.examples : []"
                    :key="exampleIndex"
                    class="example"
                  >
                    <div class="example-english">
                      <span class="example-marker" aria-hidden="true">◆</span>
                      <span
                        v-for="(fragment, fragmentIndex) in example.english"
                        :key="fragmentIndex"
                        :class="{ bold: fragment.bold, italic: fragment.italic }"
                        >{{ fragment.text }}</span
                      >
                      <button
                        v-for="exampleAudio in example.audios"
                        :key="exampleAudio.name"
                        type="button"
                        class="audio-icon-button"
                        :aria-label="`播放例句 ${exampleAudio.region} 发音`"
                        @click="playAudio('example', exampleAudio.name, exampleAudio.name)"
                      >
                        <SoundOutlined :class="{ playing: playingKey === exampleAudio.name }" />
                        <span class="audio-region">{{ exampleAudio.region }}</span>
                      </button>
                    </div>
                    <div v-if="example.chinese" class="example-chinese">
                      {{ example.chinese }}
                    </div>
                  </div>
                </section>
              </section>
            </div>
          </div>
        </div>
      </a-tab-pane>
    </a-tabs>
  </article>
</template>

<style scoped>
.entry-actions {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 8px;
  margin-bottom: 12px;
}
.dictionary-entry {
  color: var(--ant-color-text);
  font-family: Georgia, 'Times New Roman', serif;
}
.entry-header {
  padding-bottom: 4px;
}
.headword-row {
  display: flex;
  align-items: center;
  flex-wrap: wrap;
  gap: 8px;
}
h1 {
  margin: 0 8px 0 0;
  color: #102a56;
  font-size: clamp(34px, 6vw, 52px);
  line-height: 1.05;
}
:global(html.dark .dictionary-entry h1) {
  color: #93c5fd;
}
.pronunciations {
  display: flex;
  flex-wrap: wrap;
  gap: 10px;
  margin-top: 8px;
}
.pronunciation {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  gap: 5px;
  border: 0;
  border-radius: 8px;
  padding: 7px 10px;
  background: var(--ant-color-fill-tertiary);
  color: inherit;
  cursor: pointer;
  font-style: italic;
  line-height: 1;
}
.part-of-speech-tabs {
  --dictionary-tab-accent: #e32645;
  margin-top: 0;
}
.part-of-speech-tabs :deep(.ant-tabs-nav) {
  margin-bottom: 0;
}
.part-of-speech-tabs :deep(.ant-tabs-nav::before) {
  border-bottom-color: var(--ant-color-border-secondary);
}
.part-of-speech-tabs :deep(.ant-tabs-tab) {
  margin: 0;
  padding: 12px 28px 10px;
}
.part-of-speech-tabs :deep(.ant-tabs-tab + .ant-tabs-tab) {
  margin-left: 8px;
}
.part-of-speech-tabs :deep(.ant-tabs-tab-active .ant-tabs-tab-btn) {
  text-shadow: none;
}
.part-of-speech-tabs :deep(.ant-tabs-ink-bar) {
  height: 3px;
  background: var(--dictionary-tab-accent);
}
.part-of-speech-tabs.single-part-of-speech :deep(.ant-tabs-nav) {
  display: none;
}
.part-of-speech-tab-label {
  min-width: 48px;
  color: v-bind(tabTextColor);
  font-size: 18px;
  font-weight: 600;
  text-align: center;
}
.entry-content-layout {
  padding-top: 12px;
}
.pronunciation:focus-visible,
.audio-icon-button:focus-visible {
  outline: 2px solid var(--ant-color-primary);
  outline-offset: 2px;
}
.entry-sections {
  min-width: 0;
}
.content-section {
  scroll-margin-top: 16px;
  padding: 6px 0 24px;
}
.content-section + .content-section {
  border-top: 1px solid var(--ant-color-border-secondary);
  padding-top: 26px;
}
.content-section-title {
  margin: 0 0 18px;
  color: #102a56;
  font:
    700 21px system-ui,
    sans-serif;
}
:global(html.dark .dictionary-entry .content-section-title),
:global(html.dark .dictionary-entry .section-item-heading h3),
:global(html.dark .dictionary-entry .sense-section-title) {
  color: #93c5fd;
}
.part-of-speech-tag {
  margin: 0 0 14px;
  border-color: #102a56;
  background: #102a56;
  color: white;
  font-size: 15px;
  font-weight: 700;
  font-style: italic;
}
.section-item + .section-item {
  margin-top: 20px;
  border-top: 1px dashed var(--ant-color-border-secondary);
  padding-top: 20px;
}
.section-item-heading {
  display: flex;
  align-items: center;
  flex-wrap: wrap;
  gap: 8px;
  margin-bottom: 10px;
}
.section-item-heading h3 {
  margin: 0;
  color: #102a56;
  font-size: 20px;
}
.subentry-part-of-speech-tag {
  margin: 0;
  border-color: #102a56;
  background: #102a56;
  color: #fff;
  font-style: italic;
}
.subentry-pronunciations {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
  margin-bottom: 10px;
}
.section-text {
  margin: 8px 0;
  color: var(--ant-color-text-secondary);
  font:
    15px/1.65 system-ui,
    sans-serif;
  white-space: pre-line;
}
.sense {
  padding: 16px 0;
  border-bottom: 1px solid var(--ant-color-border-secondary);
}
.sense:last-child {
  border-bottom: 0;
}
.sense-section-title {
  margin: 4px 0 10px;
  color: #102a56;
  font:
    700 18px system-ui,
    sans-serif;
}
.definition {
  font-size: 18px;
  line-height: 1.4;
}
.sense-number {
  margin-right: 7px;
}
.grammar-tags {
  display: inline-flex;
  flex-wrap: wrap;
  gap: 4px;
  margin-right: 7px;
  vertical-align: 2px;
}
.grammar-tag {
  margin: 0;
  color: var(--ant-color-text-secondary);
  font-family: system-ui, sans-serif;
  font-size: 12px;
}
.definition-text {
  font-weight: 600;
}
.chinese {
  margin-left: 5px;
}
.example {
  margin: 10px 0 0 16px;
}
.sense-notes {
  margin: 10px 0;
  border-left: 3px solid var(--ant-color-primary);
  padding: 8px 12px;
  color: var(--ant-color-text-secondary);
  background: var(--ant-color-fill-quaternary);
  font:
    14px/1.55 system-ui,
    sans-serif;
}
.example-english {
  display: flex;
  align-items: baseline;
  flex-wrap: wrap;
  gap: 3px;
  font-size: 17px;
  font-style: italic;
}
.example-marker {
  flex: 0 0 auto;
  font-size: 0.55em;
  line-height: 1;
}
.example-chinese {
  margin: 5px 0 0 16px;
  color: var(--ant-color-text-secondary);
  font-family: system-ui, sans-serif;
}
.audio-icon-button {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  gap: 2px;
  border: 0;
  padding: 3px 5px;
  background: transparent;
  color: var(--ant-color-primary);
  cursor: pointer;
  line-height: 1;
}
.audio-region {
  font-size: 10px;
  font-family: system-ui, sans-serif;
  line-height: 1;
}
:deep(.pronunciation .anticon),
:deep(.audio-icon-button .anticon) {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  line-height: 1;
  vertical-align: 0;
}
:deep(.pronunciation .anticon > svg),
:deep(.audio-icon-button .anticon > svg) {
  display: block;
}
.playing {
  animation: audio-pulse 0.8s ease-in-out infinite alternate;
}
.bold {
  font-weight: 700;
}
.italic {
  font-style: italic;
}
@keyframes audio-pulse {
  to {
    opacity: 0.35;
    transform: scale(0.9);
  }
}
@media (max-width: 640px) {
  .definition {
    font-size: 16px;
  }
  .example {
    margin-left: 5px;
  }
  :deep(.ant-tabs-nav-wrap) {
    overflow-x: auto;
  }
}
@media (prefers-reduced-motion: reduce) {
  .playing {
    animation: none;
  }
}
</style>
