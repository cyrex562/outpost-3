<script setup lang="ts">
import { computed } from "vue";
import type { FieldDef } from "../types";
import TagsInput from "./TagsInput.vue";

const props = defineProps<{
  field: FieldDef;
  modelValue: unknown;
}>();

const emit = defineEmits<{
  "update:modelValue": [value: unknown];
}>();

const widthClass = computed(() => {
  switch (props.field.width) {
    case "sm": return "w-24";
    case "md": return "w-48";
    case "lg": return "w-72";
    case "full": return "w-full";
    default: return "w-full";
  }
});

function onInput(val: string) {
  if (props.field.type === "number") {
    const n = Number(val);
    emit("update:modelValue", isNaN(n) ? 0 : n);
  } else if (props.field.type === "textarea") {
    if (val.trim().startsWith("{") || val.trim().startsWith("[")) {
      try {
        const parsed = JSON.parse(val);
        emit("update:modelValue", parsed);
      } catch {
        // Fallback to string if parsing fails while typing
        emit("update:modelValue", val);
      }
    } else {
      emit("update:modelValue", val);
    }
  } else {
    emit("update:modelValue", val);
  }
}

function onCheck(checked: boolean) {
  emit("update:modelValue", checked);
}

function onTags(tags: string[]) {
  emit("update:modelValue", tags);
}
</script>

<template>
  <div :class="widthClass">
    <select
      v-if="field.type === 'select'"
      :value="String(modelValue ?? '')"
      class="w-full bg-gray-800 border border-gray-700 text-gray-200 rounded px-2 py-0.5 text-xs focus:outline-none focus:border-gray-500"
      @change="emit('update:modelValue', ($event.target as HTMLSelectElement).value)"
    >
      <option v-for="opt in field.options" :key="opt" :value="opt">{{ opt }}</option>
    </select>

    <textarea
      v-else-if="field.type === 'textarea'"
      :value="typeof modelValue === 'object' && modelValue !== null ? JSON.stringify(modelValue, null, 2) : String(modelValue ?? '')"
      rows="3"
      class="w-full bg-gray-800 border border-gray-700 text-gray-200 rounded px-2 py-1 text-xs font-mono focus:outline-none focus:border-gray-500 resize-y"
      @input="onInput(($event.target as HTMLTextAreaElement).value)"
    />

    <label
      v-else-if="field.type === 'boolean'"
      class="inline-flex items-center gap-1.5 cursor-pointer"
    >
      <input
        type="checkbox"
        :checked="Boolean(modelValue)"
        class="accent-blue-500"
        @change="onCheck(($event.target as HTMLInputElement).checked)"
      />
      <span class="text-xs text-gray-400">{{ field.label ?? field.key }}</span>
    </label>

    <TagsInput
      v-else-if="field.type === 'tags'"
      :modelValue="Array.isArray(modelValue) ? (modelValue as string[]) : []"
      @update:modelValue="onTags"
    />

    <input
      v-else
      :type="field.type === 'number' ? 'number' : 'text'"
      :value="modelValue ?? ''"
      class="w-full bg-gray-800 border border-gray-700 text-gray-200 rounded px-2 py-0.5 text-xs focus:outline-none focus:border-gray-500"
      @input="onInput(($event.target as HTMLInputElement).value)"
    />
  </div>
</template>
