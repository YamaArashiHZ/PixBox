<script setup lang="ts">
import { ref, watch, onMounted, onUnmounted } from "vue";
import { NButton, NIcon, NProgress } from "naive-ui";
import { CloseOutline } from "@vicons/ionicons5";
import type { FeedItem } from "../api";

const props = defineProps<{
  items: FeedItem[];
  progress: number | null;
  saving: boolean;
}>();

const emit = defineEmits<{
  remove: [key: string];
  save: [];
}>();

const thumbSrc = (item: FeedItem) =>
  item.thumb_b64 ? `data:image/jpeg;base64,${item.thumb_b64}` : "";

// ── 本地列表：延迟移除实现平滑删除动画 ──
// 删除（托盘×或卡片取消勾选）时先标记 leaving，宽度收缩动画播放完毕后才真正移除，
// 右侧元素随 flex 布局被宽度收缩平滑推向左侧（布局动画，不依赖 FLIP）
interface TrayEntry {
  item: FeedItem;
  leaving: boolean;
  entering: boolean;
}

const localItems = ref<TrayEntry[]>([]);
const removalTimers = new Map<string, number>();
let initialized = false;

const ANIM_MS = 260;

function scheduleRemoval(key: string) {
  const t = window.setTimeout(() => {
    removalTimers.delete(key);
    // 只移除仍处于 leaving 的项，防止误删动画期间被重新加回的
    localItems.value = localItems.value.filter((l) => l.item.key !== key || !l.leaving);
  }, ANIM_MS);
  removalTimers.set(key, t);
}

function syncItems(newItems: FeedItem[]) {
  const newKeys = new Set(newItems.map((i) => i.key));

  // 1. 消失的标记 leaving，保持原位收缩
  for (const l of localItems.value) {
    if (!newKeys.has(l.item.key) && !l.leaving) {
      l.leaving = true;
      scheduleRemoval(l.item.key);
    }
  }

  // 2. 活跃项按 props 顺序排列；已存在的复用（取消 leaving），新增的标 entering
  const existing = new Map(localItems.value.map((l) => [l.item.key, l]));
  const activeItems = newItems.map((item) => {
    const e = existing.get(item.key);
    if (e) {
      e.leaving = false;
      const t = removalTimers.get(item.key);
      if (t !== undefined) {
        clearTimeout(t);
        removalTimers.delete(item.key);
      }
      return e;
    }
    return { item, leaving: false, entering: initialized } as TrayEntry;
  });

  // 3. leaving 项插回原相对位置（找原列表中前面最近仍存活的邻居；
  //    找不到说明它在最左侧，保持原位 insertAt = 0，而不是丢到末尾）
  const merged = [...activeItems];
  for (const lv of localItems.value) {
    if (!lv.leaving || newKeys.has(lv.item.key)) continue;
    const origIdx = localItems.value.indexOf(lv);
    let insertAt = 0;
    for (let i = origIdx - 1; i >= 0; i--) {
      const prevKey = localItems.value[i].item.key;
      const idxInMerged = merged.findIndex((m) => m.item.key === prevKey);
      if (idxInMerged !== -1) {
        insertAt = idxInMerged + 1;
        break;
      }
    }
    merged.splice(insertAt, 0, lv);
  }

  localItems.value = merged;

  // 4. entering 项先以缩小态渲染，双 rAF 后清除标记触发过渡弹入
  if (activeItems.some((i) => i.entering)) {
    requestAnimationFrame(() => {
      requestAnimationFrame(() => {
        for (const e of localItems.value) e.entering = false;
      });
    });
  }
}

watch(() => props.items, syncItems, { immediate: true });

onMounted(() => {
  initialized = true;
});

onUnmounted(() => {
  removalTimers.forEach((t) => clearTimeout(t));
  removalTimers.clear();
});
</script>

<template>
  <div class="selected-tray">
    <div class="tray-bar">
      <div class="tray-info">
        <span class="tray-count">已选 {{ items.length }} 张</span>
        <n-button
          type="primary"
          :disabled="items.length === 0"
          :loading="saving"
          @click="emit('save')"
        >
          <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" style="margin-right: 6px">
            <path d="M19 21H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h11l5 5v11a2 2 0 0 1-2 2z" />
            <polyline points="17 21 17 13 7 13 7 21" />
            <polyline points="7 3 7 8 15 8" />
            <circle cx="15" cy="5.5" r="1.5" />
          </svg>
          {{ saving ? "保存中..." : `保存图片 (${items.length})` }}
        </n-button>
      </div>

      <div v-if="progress !== null" class="tray-progress">
        <n-progress
          type="line"
          :percentage="progress"
          :show-indicator="true"
          processing
          status="info"
          :height="4"
        />
      </div>

      <div class="tray-thumbs">
        <div
          v-for="entry in localItems"
          :key="entry.item.key"
          class="tray-thumb"
          :class="{ entering: entry.entering, leaving: entry.leaving }"
        >
          <img v-if="entry.item.thumb_b64" :src="thumbSrc(entry.item)" :alt="entry.item.title" />
          <div v-else class="tray-thumb-placeholder"></div>
          <button class="tray-remove" @click="emit('remove', entry.item.key)" :disabled="saving">
            <n-icon :component="CloseOutline" :size="10" />
          </button>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.selected-tray {
  position: fixed;
  bottom: 0;
  left: var(--nav-width);
  right: 0;
  z-index: 100;
  padding: 0 24px 20px 24px;
  pointer-events: none;
}

.tray-bar {
  border-radius: 14px;
  padding: 16px 20px;
  border: 1px solid var(--border-color);
  background: var(--preview-bg);
  backdrop-filter: blur(12px);
  pointer-events: auto;
  box-shadow: 0 -4px 24px rgba(0, 0, 0, 0.12);
  min-height: 185px;
  display: flex;
  flex-direction: column;
  overflow: hidden;
}

.tray-info {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 12px;
  flex-shrink: 0;
}

.tray-count {
  font-size: 15px;
  font-weight: 600;
}

.tray-progress {
  margin-bottom: 12px;
  flex-shrink: 0;
}

.tray-thumbs {
  display: flex;
  gap: 10px;
  overflow-x: auto;
  overflow-y: hidden;
  padding-bottom: 4px;
  flex: 1;
}

.tray-thumbs::-webkit-scrollbar {
  height: 4px;
}

.tray-thumbs::-webkit-scrollbar-track {
  background: transparent;
}

.tray-thumbs::-webkit-scrollbar-thumb {
  background: rgba(128, 128, 128, 0.2);
  border-radius: 2px;
}

.tray-thumb {
  position: relative;
  flex-shrink: 0;
  width: 100px;
  height: 100px;
  border-radius: 8px;
  overflow: hidden;
  transition:
    width 0.25s cubic-bezier(0.4, 0, 0.2, 1),
    margin-right 0.25s cubic-bezier(0.4, 0, 0.2, 1),
    opacity 0.25s cubic-bezier(0.4, 0, 0.2, 1),
    transform 0.25s cubic-bezier(0.4, 0, 0.2, 1);
}

/* 添加：先以缩小透明态渲染，下一帧过渡到正常 */
.tray-thumb.entering {
  opacity: 0;
  transform: scale(0.5);
}

/* 删除：宽度收缩到 0 并抵消 flex gap，右侧元素被平滑推向左侧 */
.tray-thumb.leaving {
  width: 0;
  margin-right: -10px;
  opacity: 0;
  transform: scale(0.5);
  pointer-events: none;
}

.tray-thumb img {
  width: 100%;
  height: 100%;
  object-fit: cover;
}

.tray-thumb-placeholder {
  width: 100%;
  height: 100%;
  background: rgba(128, 128, 128, 0.15);
}

.tray-remove {
  position: absolute;
  top: 4px;
  right: 4px;
  width: 22px;
  height: 22px;
  border: none;
  border-radius: 50%;
  background: rgba(0, 0, 0, 0.6);
  color: #fff;
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  opacity: 0;
  transition: opacity 0.18s ease;
  padding: 0;
}

.tray-thumb:hover .tray-remove {
  opacity: 1;
}
</style>
