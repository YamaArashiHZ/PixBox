import { ref, computed } from 'vue'
import { defineStore } from 'pinia'
import { fetchFeed, type FeedItem } from '../api'

export type FeedKind = 'following' | 'recommended'

export const useFeedStore = defineStore('feed', () => {
  const allItems = ref<FeedItem[]>([])
  const kind = ref<FeedKind>('following')
  const nextUrl = ref<string | null>(null)
  const loading = ref(false)
  const error = ref<string | null>(null)
  const selectedKeys = ref<Set<string>>(new Set())
  const expandedIds = ref<Set<number>>(new Set())

  const items = computed<FeedItem[]>(() => {
    const seen = new Set<number>()
    return allItems.value.filter((item) => {
      if (seen.has(item.illust_id)) {
        return expandedIds.value.has(item.illust_id)
      }
      seen.add(item.illust_id)
      return true
    })
  })

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

  function toggleExpand(illustId: number) {
    const s = new Set(expandedIds.value)
    if (s.has(illustId)) {
      s.delete(illustId)
    } else {
      s.add(illustId)
    }
    expandedIds.value = s
  }

  function isExpanded(illustId: number): boolean {
    return expandedIds.value.has(illustId)
  }

  async function load(k: FeedKind) {
    kind.value = k
    allItems.value = []
    nextUrl.value = null
    error.value = null
    expandedIds.value = new Set()
    loading.value = true
    try {
      const page = await fetchFeed(k === 'recommended' ? 'recommended' : 'following')
      allItems.value = page.items
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
      allItems.value.push(...page.items)
      nextUrl.value = page.next_url
    } catch (e) {
      error.value = String(e)
    } finally {
      loading.value = false
    }
  }

  function clearItems() {
    allItems.value = []
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
    toggleExpand,
    isExpanded,
    clearItems,
    load,
    loadMore,
  }
})
