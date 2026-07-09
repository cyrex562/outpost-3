<script setup lang="ts">
defineProps<{
  overridden?: boolean;
}>();

const emit = defineEmits<{
  revert: [];
}>();
</script>

<template>
  <span v-if="overridden" class="inline-flex items-center gap-1">
    <span
      class="rounded border border-amber-700 px-1 py-0.5 text-[10px] uppercase tracking-wide text-amber-300"
      title="World override"
      data-testid="override-indicator"
    >
      Override
    </span>
    <button
      class="text-yellow-500 hover:text-yellow-300 px-1"
      title="Revert to pack default"
      data-testid="override-revert"
      @click.stop="emit('revert')"
    >
      ↺
    </button>
  </span>
</template>
