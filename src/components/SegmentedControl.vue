<script setup lang="ts">
import { NButton } from "naive-ui";

defineProps<{
  options: { label: string; value: string }[];
  modelValue: string;
}>();

const emit = defineEmits<{
  "update:modelValue": [value: string];
}>();

function select(value: string) {
  emit("update:modelValue", value);
}
</script>

<template>
  <div class="segmented-control">
    <n-button
      v-for="opt in options"
      :key="opt.value"
      size="small"
      :type="modelValue === opt.value ? 'primary' : 'default'"
      :ghost="modelValue !== opt.value"
      @click="select(opt.value)"
    >
      {{ opt.label }}
    </n-button>
  </div>
</template>

<style scoped>
.segmented-control {
  display: inline-flex;
  gap: 0;
}

.segmented-control :deep(.n-button) {
  border-radius: 0;
}

.segmented-control :deep(.n-button:first-child) {
  border-radius: 10px 0 0 10px;
}

.segmented-control :deep(.n-button:last-child) {
  border-radius: 0 10px 10px 0;
}
</style>
