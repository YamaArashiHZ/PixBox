<script setup lang="ts">
import { NButton, NIcon, NProgress } from "naive-ui";
import { SaveOutline, CloseOutline } from "@vicons/ionicons5";
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
  <div class="selected-tray" v-if="items.length > 0 || saving">
    <div class="tray-bar">
      <div class="tray-info">
        <span class="tray-count">已选 {{ items.length }} 张</span>
        <n-button
          type="primary"
          :loading="saving"
          @click="emit('save')"
        >
          <n-icon :component="SaveOutline" :size="16" style="margin-right: 6px" />
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
  flex-shrink: 0;
  margin-top: auto;
}

.tray-bar {
  border-radius: 14px;
  padding: 16px 20px;
  border: 1px solid var(--border-color);
  background: var(--preview-bg);
  backdrop-filter: blur(12px);
  box-shadow: 0 -4px 24px rgba(0, 0, 0, 0.12);
  max-height: 220px;
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
