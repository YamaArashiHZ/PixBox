<script setup lang="ts">
import { ref, watch, onMounted, onUnmounted, computed } from "vue";
import { NButton, NIcon, NSpin } from "naive-ui";
import { ChevronBackOutline, ChevronForwardOutline, CloseOutline } from "@vicons/ionicons5";
import { getImageData } from "../api";

const props = defineProps<{
  url: string;
  alt: string;
  allItems: { key: string; large_url: string }[];
  currentKey: string;
}>();

const emit = defineEmits<{
  close: [];
  navigate: [direction: -1 | 1];
}>();

const imageData = ref("");
const loading = ref(false);
const zoomed = ref(false);

async function loadImage() {
  if (!props.url) return;
  loading.value = true;
  try {
    const b64 = await getImageData(props.url);
    imageData.value = `data:image/jpeg;base64,${b64}`;
  } catch {
    imageData.value = "";
  } finally {
    loading.value = false;
  }
}

watch(() => props.url, loadImage, { immediate: true });

function onKeydown(e: KeyboardEvent) {
  if (e.key === "Escape") emit("close");
  if (e.key === "ArrowLeft") emit("navigate", -1);
  if (e.key === "ArrowRight") emit("navigate", 1);
}

function toggleZoom() {
  zoomed.value = !zoomed.value;
}

function onBackdropClick(e: MouseEvent) {
  if ((e.target as HTMLElement).classList.contains("lightbox-backdrop")) {
    emit("close");
  }
}

onMounted(() => {
  document.addEventListener("keydown", onKeydown);
});

onUnmounted(() => {
  document.removeEventListener("keydown", onKeydown);
});

const currentIdx = computed(() => props.allItems.findIndex((i) => i.key === props.currentKey));
</script>

<template>
  <div class="lightbox-backdrop" @click="onBackdropClick">
    <n-button circle class="lightbox-close" @click="emit('close')">
      <n-icon :component="CloseOutline" :size="20" />
    </n-button>

    <n-button
      v-if="currentIdx > 0"
      circle
      class="lightbox-nav lightbox-prev"
      @click.stop="emit('navigate', -1)"
    >
      <n-icon :component="ChevronBackOutline" :size="24" />
    </n-button>

    <n-button
      v-if="currentIdx < allItems.length - 1"
      circle
      class="lightbox-nav lightbox-next"
      @click.stop="emit('navigate', 1)"
    >
      <n-icon :component="ChevronForwardOutline" :size="24" />
    </n-button>

    <div class="lightbox-content" :class="{ zoomed }">
      <div v-if="loading" class="lightbox-loading">
        <n-spin size="large" />
      </div>
      <img
        v-else-if="imageData"
        :src="imageData"
        :alt="alt"
        @click="toggleZoom"
        :class="{ 'cursor-zoom-in': !zoomed, 'cursor-zoom-out': zoomed }"
      />
      <div v-else class="lightbox-error">图片加载失败</div>
    </div>

    <div class="lightbox-counter">
      {{ currentIdx + 1 }} / {{ allItems.length }}
    </div>
  </div>
</template>

<style scoped>
.lightbox-backdrop {
  position: fixed;
  inset: 0;
  z-index: 1000;
  background: rgba(0, 0, 0, 0.92);
  display: flex;
  align-items: center;
  justify-content: center;
}

.lightbox-close {
  position: absolute;
  top: 16px;
  right: 16px;
  z-index: 10;
  opacity: 0.7;
}

.lightbox-close:hover {
  opacity: 1;
}

.lightbox-nav {
  position: absolute;
  top: 50%;
  transform: translateY(-50%);
  z-index: 10;
  opacity: 0.7;
}

.lightbox-nav:hover {
  opacity: 1;
}

.lightbox-prev {
  left: 16px;
}

.lightbox-next {
  right: 16px;
}

.lightbox-content {
  max-width: 90vw;
  max-height: 90vh;
  display: flex;
  align-items: center;
  justify-content: center;
}

.lightbox-content.zoomed {
  max-width: none;
  max-height: none;
  overflow: auto;
}

.lightbox-content img {
  max-width: 100%;
  max-height: 90vh;
  object-fit: contain;
  border-radius: 4px;
}

.lightbox-content.zoomed img {
  max-width: none;
  max-height: none;
}

.cursor-zoom-in {
  cursor: zoom-in;
}

.cursor-zoom-out {
  cursor: zoom-out;
}

.lightbox-loading,
.lightbox-error {
  color: rgba(255, 255, 255, 0.5);
  font-size: 16px;
  padding: 48px;
}

.lightbox-counter {
  position: absolute;
  bottom: 16px;
  left: 50%;
  transform: translateX(-50%);
  color: rgba(255, 255, 255, 0.5);
  font-size: 13px;
}
</style>
