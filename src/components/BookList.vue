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
        class="book-list-collapse p-4 sm:p-6"
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
              <span class="group-count">{{ group.books.length }}</span>
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
                  <span class="book-code-badge">{{ book.product_code }}</span>
                </template>
              </div>

              <div class="book-meta">
                <a-tooltip :title="book.title" placement="bottom">
                  <h3 class="book-title line-clamp-2">{{ book.title }}</h3>
                </a-tooltip>
                <p class="book-code">{{ book.product_code }}</p>
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

  // Sort books within each group by sort_num
  Object.values(groups).forEach((groupBooks) => {
    groupBooks.sort((a, b) => a.sort_num - b.sort_num)
  })

  // Convert to array and sort by group ID
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

const fetchBooks = async () => {
  loading.value = true
  try {
    books.value = await getBooks()
    // Load covers for each book
    const coverPromises = books.value.map(async (book) => {
      if (book.cover) {
        try {
          const bytes = await getBookCover(book)

          if (!bytes || bytes.length === 0) return

          // Determine mime type from extension
          let mimeType = 'image/jpeg'
          const coverLower = book.cover.toLowerCase()
          if (coverLower.endsWith('.png')) mimeType = 'image/png'
          else if (coverLower.endsWith('.webp')) mimeType = 'image/webp'
          else if (coverLower.endsWith('.svg')) mimeType = 'image/svg+xml'

          covers.value[book.id] = bytesToImageUrl(bytes, mimeType)
        } catch (err) {
          console.error(`Failed to load cover for book ${book.title}:`, err)
        }
      }
    })

    await Promise.all(coverPromises)
  } catch (error) {
    console.error('Failed to fetch books:', error)
  } finally {
    loading.value = false
  }
}

onMounted(() => {
  fetchBooks()
})

onUnmounted(() => {
  // Clean up object URLs to prevent memory leaks
  Object.values(covers.value).forEach((url) => {
    URL.revokeObjectURL(url)
  })
})
</script>

<style scoped>
.book-list-container {
  position: relative;
  min-height: 100%;
}

.book-list-shell {
  min-height: 100%;
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
  background: color-mix(in srgb, v-bind('token.colorFillSecondary') 80%, transparent);
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
  background: linear-gradient(to bottom, v-bind('token.colorPrimary'), v-bind('token.colorInfo'));
}

.group-name {
  font-size: 16px;
  font-weight: 800;
  color: v-bind('token.colorTextHeading');
  letter-spacing: -0.01em;
}

.group-count {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  min-width: 32px;
  height: 24px;
  padding: 0 10px;
  font-size: 12px;
  font-weight: 700;
  border-radius: 999px;
  color: v-bind('token.colorPrimary');
  background: color-mix(in srgb, v-bind('token.colorPrimaryBg') 60%, transparent);
  backdrop-filter: blur(4px);
  border: 1px solid color-mix(in srgb, v-bind('token.colorPrimaryBorder') 40%, transparent);
}

.book-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(140px, 1fr));
  gap: 24px;
  padding: 16px 0 12px;
}

.book-item {
  cursor: pointer;
  position: relative;
  border: 1px solid color-mix(in srgb, #ffffff 20%, transparent);
  background: color-mix(in srgb, v-bind('token.colorBgContainer') 65%, transparent);
  backdrop-filter: blur(16px);
  border-radius: 20px;
  padding: 12px;
  text-align: left;
  transition: all 0.3s cubic-bezier(0.34, 1.56, 0.64, 1);
  box-shadow:
    0 4px 6px -1px color-mix(in srgb, v-bind('token.colorText') 5%, transparent),
    0 2px 4px -2px color-mix(in srgb, v-bind('token.colorText') 5%, transparent);
}

.book-item:focus-visible {
  outline: 2px solid v-bind('token.colorPrimary');
  outline-offset: 4px;
}

.book-item:hover {
  transform: translateY(-6px) scale(1.02);
  border-color: color-mix(in srgb, #ffffff 40%, transparent);
  background: color-mix(in srgb, v-bind('token.colorBgContainer') 80%, transparent);
  box-shadow:
    0 20px 25px -5px color-mix(in srgb, v-bind('token.colorText') 12%, transparent),
    0 8px 10px -6px color-mix(in srgb, v-bind('token.colorText') 12%, transparent);
}

.book-cover {
  position: relative;
  width: 100%;
  aspect-ratio: 3 / 4;
  overflow: hidden;
  border-radius: 14px;
  background: color-mix(in srgb, v-bind('token.colorFillAlter') 40%, transparent);
  border: 1px solid color-mix(in srgb, #ffffff 15%, transparent);
  box-shadow: inset 0 0 20px rgba(0, 0, 0, 0.05);
}

.book-cover-placeholder {
  position: absolute;
  inset: 0;
  background: linear-gradient(
    155deg,
    color-mix(in srgb, v-bind('token.colorFillSecondary') 40%, transparent),
    color-mix(in srgb, v-bind('token.colorFillTertiary') 30%, transparent)
  );
}

.book-code-badge {
  position: absolute;
  left: 10px;
  bottom: 10px;
  max-width: calc(100% - 20px);
  font-size: 10px;
  line-height: 1;
  font-weight: 800;
  padding: 6px 8px;
  color: v-bind('token.colorTextSecondary');
  background: color-mix(in srgb, #ffffff 70%, transparent);
  backdrop-filter: blur(8px);
  border-radius: 8px;
  letter-spacing: 0.05em;
  text-transform: uppercase;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  border: 1px solid rgba(255, 255, 255, 0.3);
}

.book-meta {
  margin-top: 14px;
  padding: 0 4px;
}

.book-title {
  margin: 0;
  font-size: 13px;
  line-height: 1.5;
  font-weight: 700;
  text-align: left;
  color: v-bind('token.colorText');
  transition: color 0.2s ease;
  letter-spacing: -0.01em;
}

.book-item:hover .book-title {
  color: v-bind('token.colorPrimary');
}

.book-code {
  margin: 6px 0 0;
  font-size: 10px;
  line-height: 1.2;
  font-weight: 700;
  text-transform: uppercase;
  letter-spacing: 0.06em;
  color: v-bind('token.colorTextTertiary');
  opacity: 0.8;
}

@media (max-width: 768px) {
  .book-grid {
    grid-template-columns: repeat(auto-fill, minmax(120px, 1fr));
    gap: 14px;
  }
}
</style>
