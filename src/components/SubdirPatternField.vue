<script setup lang="ts">
import { NInput, NText } from "naive-ui";

defineProps<{
  modelValue: string;
  label?: string;
}>();

const emit = defineEmits<{
  "update:modelValue": [value: string];
}>();
</script>

<template>
  <div class="pattern-field">
    <n-text depth="3" class="label">{{ label ?? "文件夹命名规则" }}</n-text>
    <n-input
      :value="modelValue"
      placeholder="%yy_%mm%dd_%HH%MM"
      @update:value="emit('update:modelValue', $event)"
    />
    <n-text depth="3" class="help">
      %yyyy/%yy=年 %mm/%m=月 %dd/%d=日 %HH/%H=时 %MM/%M=分 %SS/%S=秒（双写补零，如 07；单写不补零，如 7）
    </n-text>
  </div>
</template>

<style scoped>
.pattern-field {
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.label {
  font-size: 13px;
}

.help {
  font-size: 11px;
}
</style>
