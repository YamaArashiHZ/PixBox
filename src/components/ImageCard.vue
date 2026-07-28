<script setup lang="ts">
import { computed } from "vue";
import { NCheckbox, NIcon, NTooltip } from "naive-ui";
import { openUrl } from "@tauri-apps/plugin-opener";

const props = defineProps<{
  itemKey: string;
  illustId: number;
  thumbB64: string;
  title: string;
  artist: string;
  isBookmarked: boolean;
  selected: boolean;
  page: number;
  pageCount: number;
  expanded: boolean;
}>();

const emit = defineEmits<{
  toggle: [];
  preview: [];
  expand: [];
  toggleBookmark: [];
}>();

const thumbSrc = computed(() =>
  props.thumbB64 ? `data:image/jpeg;base64,${props.thumbB64}` : "",
);

const showHeart = computed(() => props.page === 0);

// 用系统默认浏览器打开 Pixiv 帖子页
function openPost() {
  openUrl(`https://www.pixiv.net/artworks/${props.illustId}`).catch(() => {});
}
</script>

<template>
  <div class="image-card" :class="{ selected }" :data-key="itemKey">
    <button class="image-wrap" @click="emit('preview')">
      <img v-if="thumbSrc" :src="thumbSrc" :alt="title" loading="lazy" />
      <div v-else class="img-placeholder"></div>
    </button>

    <NCheckbox
      :checked="selected"
      class="card-checkbox"
      @update:checked="emit('toggle')"
      @click.stop
    />

    <NTooltip v-if="pageCount > 1 && page === 0" :delay="300">
      <template #trigger>
        <button class="page-badge" @click.stop="emit('expand')">
          <svg width="14" height="14" viewBox="0 0 24 24" fill="currentColor">
            <path d="M8 4a2 2 0 0 0-2 2v12a2 2 0 0 0 2 2h8a2 2 0 0 0 2-2V6a2 2 0 0 0-2-2H8z" opacity="0.5"/>
            <path d="M4 2a2 2 0 0 0-2 2v12a2 2 0 0 0 2 2h8a2 2 0 0 0 2-2V4a2 2 0 0 0-2-2H4z"/>
          </svg>
          <span>{{ expanded ? "▾" : "" }}{{ pageCount }}</span>
        </button>
      </template>
      {{ expanded ? "收起" : `展开全部 ${pageCount} 页` }}
    </NTooltip>

    <NTooltip :delay="300">
      <template #trigger>
        <button class="preview-btn" @click.stop="emit('preview')">
          <svg width="22" height="22" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round">
            <circle cx="11" cy="11" r="8" />
            <line x1="21" y1="21" x2="16.65" y2="16.65" />
          </svg>
        </button>
      </template>
      预览
    </NTooltip>

    <NTooltip v-if="showHeart" :delay="300">
      <template #trigger>
        <button class="heart-btn" :class="{ active: isBookmarked }" @click.stop="emit('toggleBookmark')">
          <svg v-if="isBookmarked" width="24" height="24" viewBox="0 0 24 24" fill="currentColor">
            <path d="M20.84 4.61a5.5 5.5 0 0 0-7.78 0L12 5.67l-1.06-1.06a5.5 5.5 0 0 0-7.78 7.78l1.06 1.06L12 21.23l7.78-7.78 1.06-1.06a5.5 5.5 0 0 0 0-7.78z" />
          </svg>
          <svg v-else width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round">
            <path d="M20.84 4.61a5.5 5.5 0 0 0-7.78 0L12 5.67l-1.06-1.06a5.5 5.5 0 0 0-7.78 7.78l1.06 1.06L12 21.23l7.78-7.78 1.06-1.06a5.5 5.5 0 0 0 0-7.78z" />
          </svg>
        </button>
      </template>
      {{ isBookmarked ? "已收藏" : "收藏" }}
    </NTooltip>

    <div class="card-info">
      <div class="card-text">
        <span class="artist">{{ artist }}</span>
        <span class="card-title" :title="title">{{ title }}</span>
      </div>
      <NTooltip :delay="300">
        <template #trigger>
          <button class="link-btn" @click.stop="openPost">
            <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round">
              <path d="M18 13v6a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V8a2 2 0 0 1 2-2h6" />
              <polyline points="15 3 21 3 21 9" />
              <line x1="10" y1="14" x2="21" y2="3" />
            </svg>
          </button>
        </template>
        在浏览器中打开
      </NTooltip>
    </div>
  </div>
</template>

<style scoped>
.image-card {
  position: relative;
  height: 100%;
  max-height: 100%;
  aspect-ratio: 2/3;
  flex-shrink: 0;
  border-radius: 12px;
  overflow: hidden;
  transition: box-shadow 0.18s ease;
  cursor: pointer;
  border: 1px solid var(--border-color);
  background: var(--preview-bg);
}

.image-card:hover {
  box-shadow: 0 0 0 2px var(--primary-soft);
}

.image-card.selected {
  box-shadow: 0 0 0 2px var(--primary-soft), 0 0 12px rgba(91, 124, 250, 0.25);
}

.image-wrap {
  width: 100%;
  height: calc(100% - 52px);
  overflow: hidden;
  border: none;
  background: none;
  cursor: pointer;
  padding: 0;
  display: block;
}

.image-wrap img {
  width: 100%;
  height: 100%;
  object-fit: cover;
  display: block;
}

.img-placeholder {
  width: 100%;
  height: 100%;
  background: linear-gradient(
    100deg,
    rgba(128, 128, 128, 0.06) 20%,
    rgba(128, 128, 128, 0.16) 40%,
    rgba(128, 128, 128, 0.06) 60%
  );
  background-size: 240% 100%;
  animation: skeleton-shimmer 1.4s ease-in-out infinite;
}

@keyframes skeleton-shimmer {
  from { background-position: 120% 0; }
  to { background-position: -120% 0; }
}

.card-checkbox {
  position: absolute;
  top: 8px;
  left: 8px;
  --n-checkbox-size: 22px;
}

.page-badge {
  position: absolute;
  top: 6px;
  right: 6px;
  height: 24px;
  padding: 0 6px;
  border: none;
  border-radius: 6px;
  background: rgba(0, 0, 0, 0.45);
  color: #fff;
  font-size: 12px;
  font-weight: 600;
  cursor: pointer;
  display: flex;
  align-items: center;
  gap: 3px;
  transition: background 0.18s ease;
  backdrop-filter: blur(4px);
}

.page-badge:hover {
  background: rgba(0, 0, 0, 0.65);
}

.page-badge svg {
  flex-shrink: 0;
  opacity: 0.9;
}

.preview-btn {
  position: absolute;
  bottom: 58px;
  left: 8px;
  width: 36px;
  height: 36px;
  border: none;
  border-radius: 50%;
  background: transparent;
  color: rgba(255, 255, 255, 0.85);
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  opacity: 0;
  transition: opacity 0.18s ease;
  padding: 0;
  filter: drop-shadow(0 1px 4px rgba(0, 0, 0, 0.5));
}

.preview-btn:hover {
  opacity: 1;
}

.image-card:hover .preview-btn {
  opacity: 1;
}

.heart-btn {
  position: absolute;
  bottom: 58px;
  right: 8px;
  width: 36px;
  height: 36px;
  border: none;
  border-radius: 50%;
  background: transparent;
  color: rgba(255, 255, 255, 0.85);
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  transition: all 0.18s ease;
  padding: 0;
  filter: drop-shadow(0 1px 4px rgba(0, 0, 0, 0.5));
}

.heart-btn svg {
  filter: drop-shadow(0 0 2px rgba(0, 0, 0, 0.5));
}

.image-card:hover .heart-btn {
  opacity: 1;
}

.heart-btn.active {
  color: #ff4757;
  opacity: 1;
}

.card-info {
  height: 52px;
  padding: 8px 10px;
  display: flex;
  align-items: center;
  gap: 6px;
}

.card-text {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
  gap: 2px;
  justify-content: center;
}

.link-btn {
  flex-shrink: 0;
  width: 28px;
  height: 28px;
  border: none;
  border-radius: 6px;
  background: transparent;
  color: currentColor;
  opacity: 0.45;
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  transition: opacity 0.18s ease, background 0.18s ease;
  padding: 0;
}

.link-btn:hover {
  opacity: 0.9;
  background: rgba(128, 128, 128, 0.15);
}

.artist {
  font-size: 12px;
  opacity: 0.65;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.card-title {
  font-size: 13px;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
</style>
