import { ref, computed } from 'vue'
import { defineStore } from 'pinia'
import { fetchFeed, type FeedItem } from '../api'

export type FeedKind = 'following' | 'recommended'

export const useFeedStore = defineStore('feed', () => {
  const items = ref<FeedItem[]>([])
  const kind = ref<FeedKind>('following')
  const nextUrl = ref<string | null>(null)
  const loading = ref(false)
  const error = ref<string | null>(null)
  const selectedKeys = ref<Set<string>>(new Set())

  const isEmpty = computed(() => items.value.length === 0 && !loading.value)

  function toggleSelect(key: string) {
    const s = new Set(selectedKeys.value)
    if (s.has(key)) {
      s.delete(key)
    } else {
      s.add(key)
    }
    selectedKeys.value = s
  }

  function isSelected(key: string): boolean {
    return selectedKeys.value.has(key)
  }

  function clearSelection() {
    selectedKeys.value = new Set()
  }

  async function load(k: FeedKind) {
    kind.value = k
    items.value = []
    nextUrl.value = null
    error.value = null
    loading.value = true
    try {
      const page = await fetchFeed(k === 'recommended' ? 'recommended' : 'following')
      items.value = page.items
      nextUrl.value = page.next_url
    } catch (e) {
      error.value = String(e)
    } finally {
      loading.value = false
    }
  }

  async function loadMore() {
    if (loading.value || !nextUrl.value) return
    loading.value = true
    try {
      const page = await fetchFeed(kind.value, nextUrl.value)
      items.value.push(...page.items)
      nextUrl.value = page.next_url
    } catch (e) {
      error.value = String(e)
    } finally {
      loading.value = false
    }
  }

  return {
    items,
    kind,
    nextUrl,
    loading,
    error,
    selectedKeys,
    isEmpty,
    toggleSelect,
    isSelected,
    clearSelection,
    load,
    loadMore,
  }
})
