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
          size="small"
          :loading="saving"
          @click="emit('save')"
        >
          <n-icon :component="SaveOutline" :size="14" style="margin-right: 4px" />
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
  position: sticky;
  bottom: 0;
  z-index: 100;
  margin-top: auto;
}

.tray-bar {
  border-radius: 12px;
  padding: 12px 14px;
  border: 1px solid var(--border-color);
  background: var(--preview-bg);
}

.tray-info {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 10px;
}

.tray-count {
  font-size: 14px;
  opacity: 0.65;
}

.tray-progress {
  margin-bottom: 10px;
}

.tray-thumbs {
  display: flex;
  gap: 8px;
  flex-wrap: wrap;
}

.tray-thumb {
  position: relative;
  width: 56px;
  height: 56px;
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
  top: 2px;
  right: 2px;
  width: 16px;
  height: 16px;
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
