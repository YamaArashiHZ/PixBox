import { getImageData } from "../api";

// 会话级内存缓存：磁盘缓存命中仍需 读盘+IPC+解码（百毫秒级），内存缓存让本会话内重复打开同步可用
const MEM_CACHE_LIMIT = 30;
const memCache = new Map<string, string>();

export function memGet(url: string): string | undefined {
  const v = memCache.get(url);
  if (v !== undefined) {
    // LRU：提到最新位置
    memCache.delete(url);
    memCache.set(url, v);
  }
  return v;
}

export function memSet(url: string, src: string) {
  memCache.delete(url);
  memCache.set(url, src);
  while (memCache.size > MEM_CACHE_LIMIT) {
    const oldest = memCache.keys().next().value!;
    memCache.delete(oldest);
  }
}

/** 预加载图片到内存缓存，供翻页时秒开；已缓存则跳过，失败静默忽略 */
export async function preloadImage(url: string) {
  if (!url || memGet(url)) return;
  try {
    const b64 = await getImageData(url);
    memSet(url, `data:image/jpeg;base64,${b64}`);
  } catch {
    // preload 失败静默忽略
  }
}
