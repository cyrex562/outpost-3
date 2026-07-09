<script setup lang="ts">
import { ref } from "vue";

const props = defineProps<{
  modelValue: string[];
}>();

const emit = defineEmits<{
  "update:modelValue": [value: string[]];
}>();

const input = ref("");

function addTag() {
  const tag = input.value.trim();
  if (tag && !props.modelValue.includes(tag)) {
    emit("update:modelValue", [...props.modelValue, tag]);
  }
  input.value = "";
}

function removeTag(idx: number) {
  const next = [...props.modelValue];
  next.splice(idx, 1);
  emit("update:modelValue", next);
}

function onKeydown(e: KeyboardEvent) {
  if (e.key === "Enter") {
    e.preventDefault();
    addTag();
  } else if (e.key === "Backspace" && input.value === "" && props.modelValue.length > 0) {
    removeTag(props.modelValue.length - 1);
  }
}
</script>

<template>
  <div class="flex flex-wrap items-center gap-1 bg-gray-800 border border-gray-700 rounded px-1.5 py-0.5 min-h-[26px]">
    <span
      v-for="(tag, idx) in modelValue"
      :key="tag"
      class="inline-flex items-center gap-0.5 bg-gray-700 text-gray-300 rounded px-1.5 py-0 text-xs"
    >
      {{ tag }}
      <button
        type="button"
        class="text-gray-500 hover:text-gray-200 ml-0.5 text-[10px] leading-none"
        @click="removeTag(idx)"
      >&times;</button>
    </span>
    <input
      v-model="input"
      class="bg-transparent text-gray-200 text-xs outline-none flex-1 min-w-[60px] py-0.5"
      placeholder="Add..."
      @keydown="onKeydown"
      @blur="addTag"
    />
  </div>
</template>
