<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted } from "vue";
import { NButton, NIcon } from "naive-ui";
import { ChevronBackOutline, ChevronForwardOutline, CloseOutline } from "@vicons/ionicons5";
import LightboxImage from "./LightboxImage.vue";

const props = defineProps<{
  url: string;
  alt: string;
  ready: boolean;
  aspect: number;
  allItems: { key: string; large_url: string }[];
  currentKey: string;
}>();

const emit = defineEmits<{
  close: [];
  navigate: [direction: -1 | 1];
}>();

// ── slide animation state ──
const lastNavDir = ref<"next" | "prev" | "">("");

const currentIdx = computed(() =>
  props.allItems.findIndex((i) => i.key === props.currentKey)
);

const slideName = computed(() => (lastNavDir.value ? `slide-${lastNavDir.value}` : ""));

// ── navigation ──
function navigate(dir: -1 | 1) {
  lastNavDir.value = dir > 0 ? "next" : "prev";
  emit("navigate", dir);
}

function onKeydown(e: KeyboardEvent) {
  if (e.key === "Escape") emit("close");
  if (e.key === "ArrowLeft") navigate(-1);
  if (e.key === "ArrowRight") navigate(1);
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
      @click.stop="navigate(-1)"
    >
      <n-icon :component="ChevronBackOutline" :size="24" />
    </n-button>

    <n-button
      v-if="currentIdx < allItems.length - 1"
      circle
      class="lightbox-nav lightbox-next"
      @click.stop="navigate(1)"
    >
      <n-icon :component="ChevronForwardOutline" :size="24" />
    </n-button>

    <div class="lightbox-content" :class="{ visible: ready }">
      <Transition :name="slideName">
        <LightboxImage :key="currentKey" :url="url" :alt="alt" :aspect="aspect" />
      </Transition>
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
  position: relative;
  width: 100vw;
  height: 100vh;
  display: flex;
  align-items: center;
  justify-content: center;
  opacity: 0;
  transition: opacity 0.12s ease;
}

.lightbox-content.visible {
  opacity: 1;
}

.lightbox-counter {
  position: absolute;
  bottom: 16px;
  left: 50%;
  transform: translateX(-50%);
  color: rgba(255, 255, 255, 0.5);
  font-size: 13px;
}

/* ── slide: forward (next) ── */
.slide-next-enter-active,
.slide-next-leave-active {
  transition: opacity 0.2s ease, transform 0.25s cubic-bezier(0.4, 0, 0.2, 1);
}

.slide-next-enter-from {
  opacity: 0;
  transform: translateX(80px);
}

.slide-next-leave-to {
  opacity: 0;
  transform: translateX(-80px);
}

/* ── slide: backward (prev) ── */
.slide-prev-enter-active,
.slide-prev-leave-active {
  transition: opacity 0.2s ease, transform 0.25s cubic-bezier(0.4, 0, 0.2, 1);
}

.slide-prev-enter-from {
  opacity: 0;
  transform: translateX(-80px);
}

.slide-prev-leave-to {
  opacity: 0;
  transform: translateX(80px);
}

/* leave 元素脱离 flex 文档流并在容器内居中，避免与新元素并排挤压 */
.slide-next-leave-active,
.slide-prev-leave-active {
  position: absolute;
  inset: 0;
  margin: auto;
  pointer-events: none;
}
</style>

<style>
.lightbox-enter-active,
.lightbox-leave-active {
  transition: opacity 0.3s ease;
}

.lightbox-enter-from,
.lightbox-leave-to {
  opacity: 0;
}
</style>
