<template>
  <div class="book-list-container h-full">
    <div v-if="loading" class="flex h-full min-h-[400px] items-center justify-center">
      <a-spin :tip="t('app.loading')" />
    </div>

    <div
      v-else-if="groupedBooks.length === 0"
      class="flex h-full min-h-[400px] items-center justify-center text-gray-400"
    >
      暂无书籍数据
    </div>

    <a-collapse
      v-else
      v-model:activeKey="activeKeys"
      class="book-list-collapse p-6"
      ghost
      expand-icon-position="right"
    >
      <a-collapse-panel v-for="group in groupedBooks" :key="group.id">
        <template #header>
          <div class="flex items-center gap-2">
            <div class="h-4 w-1 rounded-full bg-blue-500"></div>
            <span class="text-base font-bold tracking-tight text-gray-800 dark:text-gray-200">
              {{ getGroupName(group.id) }}
            </span>
          </div>
        </template>
        <div
          class="grid grid-cols-2 gap-x-6 gap-y-10 py-6 sm:grid-cols-3 md:grid-cols-4 lg:grid-cols-5 xl:grid-cols-6 2xl:grid-cols-8"
        >
          <div
            v-for="book in group.books"
            :key="book.id"
            class="book-item group flex cursor-pointer flex-col items-center"
            @click="openBook(book)"
          >
            <!-- Book Cover -->
            <div
              class="relative mb-4 flex aspect-[3/4] w-full max-w-[180px] items-center justify-center overflow-hidden rounded-xl bg-gray-100 shadow-sm transition-all duration-500 ease-out group-hover:-translate-y-2 group-hover:shadow-2xl group-hover:shadow-blue-500/20 dark:bg-gray-800"
            >
              <template v-if="covers[book.id]">
                <img
                  :src="covers[book.id]"
                  :alt="book.title"
                  class="h-full w-full object-cover transition-transform duration-700 group-hover:scale-105"
                />
              </template>
              <template v-else>
                <div
                  class="absolute inset-0 bg-gradient-to-br from-gray-50 to-gray-200 opacity-50 dark:from-gray-800 dark:to-gray-900"
                ></div>
                <span
                  class="relative z-10 select-none text-[10px] font-bold uppercase tracking-widest text-gray-400"
                  >{{ book.product_code }}</span
                >
              </template>

              <!-- Subtle overlay on hover -->
              <div
                class="absolute inset-0 bg-gradient-to-t from-black/20 to-transparent opacity-0 transition-opacity duration-300 group-hover:opacity-100"
              ></div>
            </div>

            <!-- Book Title -->
            <div class="w-full px-2">
              <a-tooltip :title="book.title" placement="bottom">
                <h3
                  class="book-title line-clamp-2 w-full text-center text-xs font-semibold leading-relaxed text-gray-700 transition-colors duration-300 group-hover:text-blue-600 dark:text-gray-300 dark:group-hover:text-blue-400"
                >
                  {{ book.title }}
                </h3>
              </a-tooltip>
              <div
                class="mt-1 flex justify-center opacity-0 transition-opacity duration-300 group-hover:opacity-100"
              >
                <span class="text-[10px] font-medium uppercase tracking-tighter text-blue-500"
                  >Click to Open</span
                >
              </div>
            </div>
          </div>
        </div>
      </a-collapse-panel>
    </a-collapse>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, computed, onUnmounted } from 'vue'
import { useI18n } from 'vue-i18n'
import { getBooks, getBookCover, bytesToImageUrl } from '../lib/api'
import { Book, BookGroup } from '../types'
import { useAppStore } from '../stores/app'

const { t } = useI18n()
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
.book-item {
  transition: transform 0.2s;
}
.book-item:hover {
  transform: translateY(-4px);
}
</style>
