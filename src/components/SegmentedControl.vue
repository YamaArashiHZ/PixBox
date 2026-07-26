<script setup lang="ts">
import { computed, ref, watch, nextTick, onMounted } from "vue";

const props = defineProps<{
  options: { label: string; value: string }[];
  modelValue: string;
}>();

const emit = defineEmits<{
  "update:modelValue": [value: string];
}>();

const containerRef = ref<HTMLElement | null>(null);
const activeIndex = computed(() => props.options.findIndex((o) => o.value === props.modelValue));
const indicatorStyle = ref({ left: "0px", width: "0px", opacity: "0" });

async function updateIndicator() {
  await nextTick();
  const el = containerRef.value;
  if (!el) return;
  const buttons = el.querySelectorAll(".segment");
  if (!buttons.length) return;
  const idx = Math.max(0, activeIndex.value);
  const btn = buttons[idx] as HTMLElement;
  indicatorStyle.value = {
    left: `${btn.offsetLeft}px`,
    width: `${btn.offsetWidth}px`,
    opacity: "1",
  };
}

watch(() => props.modelValue, updateIndicator, { immediate: true });

onMounted(updateIndicator);

function select(value: string) {
  emit("update:modelValue", value);
}
</script>

<template>
  <div ref="containerRef" class="segmented-control">
    <div class="indicator" :style="indicatorStyle" />
    <button
      v-for="opt in options"
      :key="opt.value"
      type="button"
      class="segment"
      :class="{ active: modelValue === opt.value }"
      @click="select(opt.value)"
    >
      {{ opt.label }}
    </button>
  </div>
</template>

<style scoped>
.segmented-control {
  position: relative;
  display: inline-flex;
  padding: 3px;
  border-radius: 10px;
  background: rgba(128, 128, 128, 0.1);
  border: 1px solid rgba(128, 128, 128, 0.08);
  gap: 2px;
}

.indicator {
  position: absolute;
  top: 3px;
  height: calc(100% - 6px);
  border-radius: 8px;
  background: var(--primary-soft);
  box-shadow: 0 1px 4px rgba(91, 124, 250, 0.3);
  transition: left 0.25s cubic-bezier(0.4, 0, 0.2, 1), width 0.25s cubic-bezier(0.4, 0, 0.2, 1), opacity 0.2s ease;
  z-index: 0;
  pointer-events: none;
}

.segment {
  position: relative;
  z-index: 1;
  padding: 6px 18px;
  border: none;
  border-radius: 8px;
  background: transparent;
  color: rgba(128, 128, 128, 0.8);
  font-size: 13px;
  font-weight: 500;
  cursor: pointer;
  transition: color 0.18s ease;
}

.segment:hover {
  color: inherit;
}

.segment.active {
  color: #fff;
}
</style>
