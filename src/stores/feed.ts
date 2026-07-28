import { ref, computed } from 'vue'
import { defineStore } from 'pinia'
import { fetchFeed, loadCachedFeed, toggleBookmark, type FeedItem, type ThumbProgress } from '../api'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'

export type FeedKind = 'following' | 'recommended'

export const useFeedStore = defineStore('feed', () => {
  const allItems = ref<FeedItem[]>([])
  const kind = ref<FeedKind>('following')
  const nextUrl = ref<string | null>(null)
  const loading = ref(false)
  const refreshing = ref(false)
  const error = ref<string | null>(null)
  const selectedKeys = ref<Set<string>>(new Set())
  const expandedIds = ref<Set<number>>(new Set())
  const contentMode = ref('all')
  const pendingThumbs = new Map<string, string>()
  let thumbUnlisten: UnlistenFn | null = null

  function mergeThumbnails(incoming: FeedItem[]): FeedItem[] {
    const existing = new Map(
      allItems.value
        .filter((item) => item.thumb_b64)
        .map((item) => [item.key, item.thumb_b64]),
    )

    return incoming.map((item) => {
      const thumb = item.thumb_b64 || pendingThumbs.get(item.key) || existing.get(item.key) || ''
      pendingThumbs.delete(item.key)
      return thumb === item.thumb_b64 ? item : { ...item, thumb_b64: thumb }
    })
  }

  async function startThumbListener() {
    if (thumbUnlisten) return
    thumbUnlisten = await listen<ThumbProgress>('thumbnail-event', (event) => {
      const p = event.payload
      const idx = allItems.value.findIndex((i) => i.key === p.key)
      if (idx !== -1) {
        allItems.value[idx] = { ...allItems.value[idx], thumb_b64: p.thumb_b64 }
      } else {
        pendingThumbs.set(p.key, p.thumb_b64)
      }
    })
  }

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
    const oldBookmarks = new Map<number, boolean>()
    for (const item of allItems.value) {
      oldBookmarks.set(item.illust_id, item.is_bookmarked)
    }
    allItems.value = []
    nextUrl.value = null
    error.value = null
    expandedIds.value = new Set()
    loading.value = true
    refreshing.value = true
    try {
      const apiKind = k === 'recommended' ? 'recommended' : 'following'
      const freshPromise = fetchFeed(apiKind, undefined, apiKind === 'recommended' ? contentMode.value : undefined)
      if (apiKind === 'following') {
        try {
          const cached = await loadCachedFeed(apiKind)
          let items = mergeThumbnails(cached.items)
          if (oldBookmarks.size > 0) {
            items = items.map((item) => {
              const bm = oldBookmarks.get(item.illust_id)
              if (bm !== undefined && bm !== item.is_bookmarked) {
                return { ...item, is_bookmarked: bm }
              }
              return item
            })
          }
          allItems.value = items
          nextUrl.value = cached.next_url
        } catch {
          // No cache yet; skeleton cards remain until fresh metadata arrives.
        }
      }
      const page = await freshPromise
      allItems.value = mergeThumbnails(page.items)
      nextUrl.value = page.next_url
    } catch (e) {
      error.value = String(e)
    } finally {
      loading.value = false
      refreshing.value = false
    }
  }

  async function loadMore() {
    if (loading.value || !nextUrl.value) return
    loading.value = true
    try {
      const page = await fetchFeed(kind.value, nextUrl.value, kind.value === 'recommended' ? contentMode.value : undefined)
      allItems.value.push(...mergeThumbnails(page.items))
      nextUrl.value = page.next_url
    } catch (e) {
      error.value = String(e)
    } finally {
      loading.value = false
    }
  }

  function clearItems() {
    allItems.value = []
    pendingThumbs.clear()
  }

  async function doToggleBookmark(illustId: number, isBookmarked: boolean) {
    const idx = allItems.value.findIndex((i) => i.illust_id === illustId)
    if (idx === -1) return

    const newItems = [...allItems.value]
    newItems[idx] = { ...newItems[idx], is_bookmarked: !isBookmarked }
    allItems.value = newItems

    try {
      await toggleBookmark(illustId, isBookmarked)
    } catch {
      const rollback = [...allItems.value]
      rollback[idx] = { ...rollback[idx], is_bookmarked }
      allItems.value = rollback
    }
  }

  return {
    items,
    kind,
    nextUrl,
    loading,
    refreshing,
    error,
    contentMode,
    selectedKeys,
    isEmpty,
    toggleSelect,
    isSelected,
    clearSelection,
    toggleExpand,
    isExpanded,
    clearItems,
    startThumbListener,
    doToggleBookmark,
    load,
    loadMore,
  }
})
