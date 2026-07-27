import { invoke, Channel } from '@tauri-apps/api/core'

export interface PixivUser {
  id: string
  name: string
  account: string
}

export interface FeedItem {
  key: string
  illust_id: number
  page: number
  page_count: number
  thumb_b64: string
  large_url: string
  original_url: string
  title: string
  artist: string
  is_bookmarked: boolean
  width: number
  height: number
}

export interface FeedPage {
  items: FeedItem[]
  next_url: string | null
}

export interface SaveItem {
  key: string
  illust_id: number
  original_url: string
}

export interface ProgressEvent {
  current: number
  total: number
  percent: number
  key: string
  status: string
}

export interface ThumbProgress {
  key: string
  thumb_b64: string
}

export interface SaveReport {
  success: number
  failed: number
  errors: string[]
}

export async function startOauth(): Promise<PixivUser> {
  return invoke('start_oauth')
}

export async function getLoginStatus(): Promise<PixivUser | null> {
  return invoke('get_login_status')
}

export async function logout(): Promise<void> {
  return invoke('logout')
}

export async function fetchFeed(kind: string, nextUrl?: string, contentMode?: string): Promise<FeedPage> {
  return invoke('fetch_feed', { kind, nextUrl, contentMode })
}

export async function loadCachedFeed(kind: string): Promise<FeedPage> {
  return invoke('load_cached_feed', { kind })
}

export async function getImageData(url: string): Promise<string> {
  return invoke('get_image_data', { url })
}

export async function saveImages(
  items: SaveItem[],
  onProgress: (e: ProgressEvent) => void
): Promise<SaveReport> {
  const channel = new Channel<ProgressEvent>()
  channel.onmessage = onProgress
  return invoke('save_images', { items, onProgress: channel })
}

export async function testProxy(proxy: string): Promise<number> {
  return invoke('test_proxy', { proxy })
}

export async function getCacheSize(): Promise<number> {
  return invoke('get_cache_size')
}

export async function clearCache(): Promise<void> {
  return invoke('clear_cache')
}

export async function enforceCacheLimit(): Promise<void> {
  return invoke('enforce_cache_limit')
}
