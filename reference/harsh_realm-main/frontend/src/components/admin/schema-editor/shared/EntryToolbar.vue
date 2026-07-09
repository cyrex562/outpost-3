<script setup lang="ts">
defineProps<{
  canMoveUp?: boolean;
  canMoveDown?: boolean;
  canDelete?: boolean;
  canClone?: boolean;
}>();

defineEmits<{
  add: [];
  delete: [];
  clone: [];
  moveUp: [];
  moveDown: [];
}>();
</script>

<template>
  <div class="flex items-center gap-1">
    <button
      type="button"
      class="px-1.5 py-0.5 text-[10px] rounded border border-green-700 text-green-400 hover:bg-green-900/30"
      title="Add entry"
      @click="$emit('add')"
    >+</button>
    <button
      v-if="canClone !== false"
      type="button"
      class="px-1.5 py-0.5 text-[10px] rounded border border-blue-700 text-blue-400 hover:bg-blue-900/30"
      title="Clone entry"
      @click="$emit('clone')"
    >&#x2398;</button>
    <button
      v-if="canMoveUp"
      type="button"
      class="px-1.5 py-0.5 text-[10px] rounded border border-gray-600 text-gray-400 hover:bg-gray-800"
      title="Move up"
      @click="$emit('moveUp')"
    >&uarr;</button>
    <button
      v-if="canMoveDown"
      type="button"
      class="px-1.5 py-0.5 text-[10px] rounded border border-gray-600 text-gray-400 hover:bg-gray-800"
      title="Move down"
      @click="$emit('moveDown')"
    >&darr;</button>
    <button
      v-if="canDelete !== false"
      type="button"
      class="px-1.5 py-0.5 text-[10px] rounded border border-red-700 text-red-400 hover:bg-red-900/30"
      title="Delete entry"
      @click="$emit('delete')"
    >&times;</button>
  </div>
</template>
