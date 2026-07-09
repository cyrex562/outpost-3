<script setup lang="ts">
import { computed, onMounted } from "vue";
import { useWorldStore } from "../stores/world";
import { useGameStore } from "../stores/game";
import ChatLog from "./ChatLog.vue";
import HintBar from "./HintBar.vue";
import CommandInput from "./CommandInput.vue";
import AttributeAssignModal from "./AttributeAssignModal.vue";
import CharacterCreationForm from "./CharacterCreationForm.vue";

const props = defineProps<{
  send: (text: string) => void;
}>();

const worldStore = useWorldStore();
const gameStore = useGameStore();

// Character creation is active when a world is loaded, the first character
// fetch has resolved, and no living character exists yet.
const inCreation = computed(
  () =>
    !!worldStore.activeWorld &&
    gameStore.characterChecked &&
    gameStore.character === null,
);
const showForm = computed(
  () => inCreation.value && gameStore.creationMode === "form",
);
const inChatCreation = computed(
  () => inCreation.value && gameStore.creationMode === "chat",
);

onMounted(() => {
  if (worldStore.activeWorld) {
    gameStore.loadCharacter();
  }
});
</script>

<template>
  <div class="flex flex-col h-full">
    <CharacterCreationForm v-if="showForm" />
    <template v-else>
      <ChatLog class="flex-1 min-h-0" />
      <HintBar />
      <div
        v-if="inChatCreation"
        class="px-2 py-1 border-t border-gray-800 bg-gray-950"
      >
        <button
          class="text-[11px] text-gray-500 hover:text-amber-200"
          data-testid="cc-use-form"
          @click="gameStore.setCreationMode('form')"
        >
          ← Use a form instead
        </button>
      </div>
      <CommandInput :disabled="!worldStore.activeWorld" @send="props.send" />
      <AttributeAssignModal :send="props.send" />
    </template>
  </div>
</template>
