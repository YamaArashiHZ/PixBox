<script setup lang="ts">
import { NCheckbox, NTag, NButton, NIcon, NTooltip } from "naive-ui";
import { SearchOutline, ImageOutline } from "@vicons/ionicons5";

const props = defineProps<{
  thumbB64: string;
  title: string;
  artist: string;
  isBookmarked: boolean;
  selected: boolean;
}>();

const emit = defineEmits<{
  toggle: [];
  preview: [];
}>();

const thumbSrc = props.thumbB64 ? `data:image/jpeg;base64,${props.thumbB64}` : "";
</script>

<template>
  <div class="image-card" :class="{ selected }">
    <button class="image-wrap" @click="emit('preview')">
      <img v-if="thumbSrc" :src="thumbSrc" :alt="title" loading="lazy" />
      <div v-else class="img-placeholder">
        <NIcon :component="ImageOutline" :size="32" color="rgba(128,128,128,0.35)" />
      </div>
    </button>

    <NCheckbox
      :checked="selected"
      class="card-checkbox"
      @update:checked="emit('toggle')"
      @click.stop
    />

    <NTag v-if="isBookmarked" type="success" size="small" class="saved-badge">已存</NTag>

    <NTooltip :delay="300">
      <template #trigger>
        <NButton circle size="tiny" class="preview-btn" @click.stop="emit('preview')">
          <NIcon :component="SearchOutline" :size="14" />
        </NButton>
      </template>
      预览
    </NTooltip>

    <div class="card-info">
      <span class="artist">{{ artist }}</span>
      <span class="card-title" :title="title">{{ title }}</span>
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
  display: flex;
  align-items: center;
  justify-content: center;
}

.card-checkbox {
  position: absolute;
  top: 8px;
  left: 8px;
  --n-checkbox-size: 22px;
}

.saved-badge {
  position: absolute;
  top: 8px;
  right: 8px;
}

.preview-btn {
  position: absolute;
  bottom: 60px;
  right: 8px;
  opacity: 0;
  transition: opacity 0.18s ease;
}

.image-card:hover .preview-btn {
  opacity: 1;
}

.card-info {
  height: 52px;
  padding: 8px 10px;
  display: flex;
  flex-direction: column;
  gap: 2px;
  justify-content: center;
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
