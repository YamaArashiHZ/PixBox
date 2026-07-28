import { ref, computed } from 'vue'
import { defineStore } from 'pinia'
import { fetchFeed, loadCachedFeed, toggleBookmark, type FeedItem, type FeedPage, type ThumbProgress } from '../api'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'

export type FeedKind = 'following' | 'recommended'

// 封面卡：多图帖展开时第 1 页就地扮演"帖子代表"，cover 标记用于区分展示形态
// selKey：镜像子卡（1/N）的选择绑定到真实页 key，避免托盘/保存出现重复
export type DisplayItem = FeedItem & { cover?: boolean; selKey?: string }

export const useFeedStore = defineStore('feed', () => {
  const allItems = ref<FeedItem[]>([])
  const kind = ref<FeedKind>('following')
  const nextUrl = ref<string | null>(null)
  const loading = ref(false)
  const refreshing = ref(false)
  const error = ref<string | null>(null)
  const selectedKeys = ref<Set<string>>(new Set())
  const selectedItemsList = ref<FeedItem[]>([])
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

  const items = computed<DisplayItem[]>(() => {
    const seen = new Set<number>()
    const result: DisplayItem[] = []
    for (const item of allItems.value) {
      if (seen.has(item.illust_id)) {
        if (expandedIds.value.has(item.illust_id)) result.push(item)
        continue
      }
      seen.add(item.illust_id)
      // 展开的多图帖：第 1 页就地转为封面卡（key 不变、位置不动，避免动画断层），
      // 其子页 1/N 由镜像卡顶替，选择仍绑定真实 key
      if (item.page_count > 1 && expandedIds.value.has(item.illust_id)) {
        result.push({ ...item, cover: true })
        result.push({ ...item, key: `${item.key}-sub`, selKey: item.key })
      } else {
        result.push(item)
      }
    }
    return result
  })

  const isEmpty = computed(() => items.value.length === 0 && !loading.value)

  function addToSelectedList(items: FeedItem[]) {
    const existing = new Set(selectedItemsList.value.map((i) => i.key))
    const toAdd = items.filter((i) => !existing.has(i.key))
    if (toAdd.length > 0) {
      selectedItemsList.value = [...selectedItemsList.value, ...toAdd]
    }
  }

  function removeFromSelectedList(keys: string[]) {
    const keySet = new Set(keys)
    if (selectedItemsList.value.some((i) => keySet.has(i.key))) {
      selectedItemsList.value = selectedItemsList.value.filter((i) => !keySet.has(i.key))
    }
  }

  function toggleSelect(key: string) {
    const s = new Set(selectedKeys.value)
    if (s.has(key)) {
      s.delete(key)
      removeFromSelectedList([key])
    } else {
      s.add(key)
      const item = allItems.value.find((i) => i.key === key)
      if (item) addToSelectedList([item])
    }
    selectedKeys.value = s
  }

  function isSelected(key: string): boolean {
    return selectedKeys.value.has(key)
  }

  // 整帖选择：封面卡勾选时选中/取消该帖全部真实页
  function illustKeys(illustId: number): string[] {
    return allItems.value.filter((i) => i.illust_id === illustId).map((i) => i.key)
  }

  function isAllSelected(illustId: number): boolean {
    const keys = illustKeys(illustId)
    return keys.length > 0 && keys.every((k) => selectedKeys.value.has(k))
  }

  // 部分页已选（封面卡复选框显示半选态）
  function isSomeSelected(illustId: number): boolean {
    const keys = illustKeys(illustId)
    let n = 0
    for (const k of keys) if (selectedKeys.value.has(k)) n++
    return n > 0 && n < keys.length
  }

  function toggleSelectIllust(illustId: number) {
    const keys = illustKeys(illustId)
    const s = new Set(selectedKeys.value)
    if (keys.length > 0 && keys.every((k) => s.has(k))) {
      keys.forEach((k) => s.delete(k))
      removeFromSelectedList(keys)
    } else {
      keys.forEach((k) => s.add(k))
      const items = allItems.value.filter((i) => keys.includes(i.key))
      addToSelectedList(items)
    }
    selectedKeys.value = s
  }

  function clearSelection() {
    selectedKeys.value = new Set()
    selectedItemsList.value = []
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
      if (k === 'recommended' && contentMode.value === 'all') {
        const [recPage, rankPage] = await Promise.all([
          fetchFeed('recommended', undefined, 'all'),
          fetchFeed('ranking', undefined, undefined),
        ])
        const recIds = new Set(recPage.items.map((i) => i.illust_id))
        const extraRank = rankPage.items.filter((i) => !recIds.has(i.illust_id))
        allItems.value = mergeThumbnails([...recPage.items, ...extraRank])
        nextUrl.value = recPage.next_url
      } else if (k === 'recommended' && contentMode.value === 'r18') {
        const [recPage, rankPage] = await Promise.all([
          fetchFeed('recommended', undefined, 'r18'),
          fetchFeed('ranking', undefined, undefined),
        ])
        const recIds = new Set(recPage.items.map((i) => i.illust_id))
        const extraRank = rankPage.items.filter((i) => !recIds.has(i.illust_id))
        allItems.value = mergeThumbnails([...recPage.items, ...extraRank])
        nextUrl.value = recPage.next_url
      } else {
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
      }
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
      let page: FeedPage
        if (kind.value === 'recommended' && contentMode.value === 'r18') {
          page = await fetchFeed('recommended', nextUrl.value, 'r18')
      } else {
        page = await fetchFeed(kind.value, nextUrl.value, kind.value === 'recommended' ? contentMode.value : undefined)
      }
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
      rollback[idx] = { ...rollback[idx], is_bookmarked: isBookmarked }
      allItems.value = rollback
    }
  }

  function markBookmarked(illustIds: Set<number>) {
    if (illustIds.size === 0) return
    allItems.value = allItems.value.map((item) =>
      illustIds.has(item.illust_id)
        ? { ...item, is_bookmarked: true }
        : item
    )
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
    selectedItems: selectedItemsList,
    isEmpty,
    toggleSelect,
    isSelected,
    isAllSelected,
    isSomeSelected,
    toggleSelectIllust,
    clearSelection,
    toggleExpand,
    isExpanded,
    clearItems,
    startThumbListener,
    doToggleBookmark,
    markBookmarked,
    load,
    loadMore,
  }
})
