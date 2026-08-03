<template>
  <div class="book-list-container h-full">
    <LoadingBlock v-if="loading" min-height="400px" :message="t('app.loading')" />

    <div v-else-if="groupedBooks.length === 0" class="book-empty-state">
      <div class="book-empty-icon" aria-hidden="true">
        <BookOutlined />
      </div>
      <p class="book-empty-title">暂无书籍数据</p>
    </div>

    <div v-else class="book-list-shell">
      <a-collapse
        v-model:activeKey="activeKeys"
        class="book-list-collapse p-4 sm:p-5"
        ghost
        expand-icon-position="right"
      >
        <a-collapse-panel v-for="group in groupedBooks" :key="group.id">
          <template #header>
            <div class="group-header">
              <div class="group-header-main">
                <span class="group-marker" aria-hidden="true"></span>
                <span class="group-name">{{ getGroupName(group.id) }}</span>
              </div>
            </div>
          </template>

          <div class="book-grid">
            <button
              v-for="book in group.books"
              :key="book.id"
              type="button"
              class="book-item group"
              :aria-label="book.title"
              @click="openBook(book)"
            >
              <div class="book-cover">
                <template v-if="covers[book.id]">
                  <img
                    :src="covers[book.id]"
                    :alt="book.title"
                    class="h-full w-full object-cover transition-transform duration-300 group-hover:scale-[1.02]"
                  />
                </template>
                <template v-else>
                  <div class="book-cover-placeholder"></div>
                </template>
              </div>

              <div class="book-meta">
                <a-tooltip :title="book.title" placement="bottom">
                  <h3 class="book-title line-clamp-2">{{ book.title }}</h3>
                </a-tooltip>
              </div>
            </button>
          </div>
        </a-collapse-panel>
      </a-collapse>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, computed, onUnmounted } from 'vue'
import { useI18n } from 'vue-i18n'
import { theme } from 'ant-design-vue'
import { BookOutlined } from '@ant-design/icons-vue'
import { getBooks, getBookCover, bytesToImageUrl } from '../lib/api'
import { Book, BookGroup } from '../types'
import { useAppStore } from '../stores/app'
import LoadingBlock from './common/loading/LoadingBlock.vue'

const { t } = useI18n()
const { useToken } = theme
const { token } = useToken()
const appStore = useAppStore()
const loading = ref(true)
const books = ref<Book[]>([])
const covers = ref<Record<number, string>>({})
const activeKeys = ref<number[]>([BookGroup.Vocabulary, BookGroup.Grammar])
let fetchGeneration = 0
let disposed = false

const openBook = (book: Book) => {
  appStore.currentBook = book
}

const groupedBooks = computed(() => {
  const groups: Record<number, Book[]> = {}

  books.value.forEach((book) => {
    if (!groups[book.book_group]) {
      groups[book.book_group] = []
    }
    groups[book.book_group].push(book)
  })

  // 按 sort_num 对每个分组内的图书排序。
  Object.values(groups).forEach((groupBooks) => {
    groupBooks.sort((a, b) => a.sort_num - b.sort_num)
  })

  // 转换为数组后按分组标识排序。
  return Object.keys(groups)
    .map((key) => ({
      id: parseInt(key) as BookGroup,
      books: groups[parseInt(key)],
    }))
    .sort((a, b) => a.id - b.id)
})

const getGroupName = (groupId: BookGroup) => {
  switch (groupId) {
    case BookGroup.Vocabulary:
      return t('app.bookGroups.vocabulary')
    case BookGroup.Grammar:
      return t('app.bookGroups.grammar')
    default:
      return `Group ${groupId}`
  }
}

/** 推断封面资源的 MIME 类型。 */
const getCoverMimeType = (coverPath: string) => {
  const normalizedPath = coverPath.toLowerCase()
  if (normalizedPath.endsWith('.png')) return 'image/png'
  if (normalizedPath.endsWith('.webp')) return 'image/webp'
  if (normalizedPath.endsWith('.svg')) return 'image/svg+xml'
  return 'image/jpeg'
}

/** 使用固定数量的工作协程加载封面，避免一次创建过多请求。 */
const loadCovers = async (items: Book[], generation: number) => {
  let nextIndex = 0
  const workerCount = Math.min(4, items.length)
  const worker = async () => {
    while (nextIndex < items.length) {
      const book = items[nextIndex++]
      if (!book.cover) continue
      try {
        const bytes = await getBookCover(book)
        if (!bytes || bytes.length === 0) continue

        const url = bytesToImageUrl(bytes, getCoverMimeType(book.cover))
        if (disposed || generation !== fetchGeneration) {
          URL.revokeObjectURL(url)
          return
        }
        covers.value[book.id] = url
      } catch (err) {
        console.error(`Failed to load cover for book ${book.title}:`, err)
      }
    }
  }

  await Promise.all(Array.from({ length: workerCount }, worker))
}

/** 加载书籍列表；封面在列表显示后于后台继续加载。 */
const fetchBooks = async () => {
  const generation = ++fetchGeneration
  loading.value = true
  try {
    const loadedBooks = await getBooks()
    if (disposed || generation !== fetchGeneration) return
    books.value = loadedBooks
    loading.value = false
    void loadCovers(loadedBooks, generation)
  } catch (error) {
    console.error('Failed to fetch books:', error)
  } finally {
    if (!disposed && generation === fetchGeneration) {
      loading.value = false
    }
  }
}

onMounted(() => {
  disposed = false
  fetchBooks()
})

onUnmounted(() => {
  disposed = true
  fetchGeneration++
  // 释放对象 URL，避免封面缓存造成内存泄漏。
  Object.values(covers.value).forEach((url) => {
    URL.revokeObjectURL(url)
  })
})
</script>

<style scoped>
.book-list-container {
  position: relative;
  width: 100%;
  max-width: 100%;
  min-width: 0;
  min-height: 100%;
  padding: 14px;
  overflow-y: auto;
  overflow-x: hidden;
}

.book-list-shell {
  display: flex;
  flex-direction: column;
  gap: 12px;
  width: 100%;
  max-width: 100%;
  min-width: 0;
}

.book-empty-state {
  min-height: 400px;
  height: 100%;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 10px;
  color: v-bind('token.colorTextTertiary');
}

.book-empty-icon {
  width: 48px;
  height: 48px;
  border-radius: 999px;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  font-size: 20px;
  background: color-mix(in srgb, v-bind('token.colorFillSecondary') 40%, transparent);
}

.book-empty-title {
  margin: 0;
  font-size: 14px;
  font-weight: 600;
}

.group-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
}

.group-header-main {
  display: inline-flex;
  align-items: center;
  gap: 10px;
}

.group-marker {
  width: 6px;
  height: 18px;
  border-radius: 999px;
  background: v-bind('token.colorPrimary');
}

.group-name {
  font-size: 16px;
  font-weight: 800;
  color: v-bind('token.colorTextHeading');
  letter-spacing: -0.01em;
}

.book-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(150px, 1fr));
  min-width: 0;
  gap: 16px;
  padding: 10px 0 12px;
}

.book-item {
  cursor: pointer;
  position: relative;
  border: 1px solid color-mix(in srgb, v-bind('token.colorBorderSecondary') 50%, transparent);
  background: v-bind('token.colorBgContainer');
  border-radius: 12px;
  padding: 10px;
  min-height: 240px;
  text-align: left;
  transition:
    border-color 0.18s ease,
    transform 0.18s ease;
}

.book-item:focus-visible {
  outline: 2px solid v-bind('token.colorPrimary');
  outline-offset: 2px;
}

.book-item:hover {
  border-color: color-mix(in srgb, v-bind('token.colorPrimaryBorder') 50%, transparent);
  transform: translateY(-2px);
}

.book-cover {
  position: relative;
  width: 100%;
  aspect-ratio: 3 / 4;
  overflow: hidden;
  border-radius: 10px;
  background: color-mix(in srgb, v-bind('token.colorFillAlter') 20%, transparent);
  border: 1px solid color-mix(in srgb, v-bind('token.colorBorderSecondary') 50%, transparent);
}

.book-cover-placeholder {
  position: absolute;
  inset: 0;
  background: color-mix(in srgb, v-bind('token.colorFillSecondary') 20%, transparent);
}

.book-meta {
  margin-top: 12px;
  padding: 0 2px;
}

.book-title {
  margin: 0;
  font-size: 14px;
  line-height: 1.35;
  font-weight: 700;
  text-align: left;
  color: v-bind('token.colorText');
  transition: color 0.2s ease;
  letter-spacing: -0.01em;
}

.book-item:hover .book-title {
  color: v-bind('token.colorPrimary');
}

@media (max-width: 768px) {
  .book-list-container {
    padding: 10px;
  }

  .book-grid {
    grid-template-columns: repeat(auto-fill, minmax(128px, 1fr));
    gap: 12px;
  }

  .book-item {
    min-height: 210px;
  }
}
</style>
