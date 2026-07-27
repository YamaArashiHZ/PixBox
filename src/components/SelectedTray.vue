<script setup lang="ts">
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
          v-for="item in items"
          :key="item.key"
          class="tray-thumb"
        >
          <img v-if="item.thumb_b64" :src="thumbSrc(item)" :alt="item.title" />
          <div v-else class="tray-thumb-placeholder"></div>
          <button class="tray-remove" @click="emit('remove', item.key)" :disabled="saving">
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
