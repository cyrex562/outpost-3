<script setup lang="ts">
import { ref, onMounted } from "vue";
import { useWorldStore } from "../stores/world";
import PackPicker from "./world/PackPicker.vue";

const emit = defineEmits<{ close: [] }>();
const worldStore = useWorldStore();

// Tabs: "load" or "create"
const tab = ref<"load" | "create">("load");
const progressText = ref("");

// Create form
const newName = ref("");
const newWidth = ref(20);
const newHeight = ref(20);
const newSeed = ref("");
const selectedPackIds = ref<string[]>(["xwn-core"]);

onMounted(() => {
  worldStore.fetchWorlds();
});

async function handleCreate() {
  if (!newName.value.trim() || worldStore.loading) return;
  const seed = newSeed.value.trim() ? parseInt(newSeed.value.trim(), 10) : undefined;
  progressText.value = "Creating world database...";
  setTimeout(() => { if (worldStore.loading) progressText.value = "Generating world map..."; }, 1500);
  setTimeout(() => { if (worldStore.loading) progressText.value = "Placing settlements and features..."; }, 3500);
  setTimeout(() => { if (worldStore.loading) progressText.value = "Seeding config tables..."; }, 5500);
  setTimeout(() => { if (worldStore.loading) progressText.value = "Loading world..."; }, 7500);
  const ok = await worldStore.createWorld(
    newName.value.trim(),
    newWidth.value,
    newHeight.value,
    seed,
    selectedPackIds.value
  );
  progressText.value = "";
  if (ok) emit("close");
}

async function handleLoad(file: string) {
  if (worldStore.loading) return;
  const ok = await worldStore.loadWorld(file);
  if (ok) emit("close");
}

function formatDate(iso: string) {
  try {
    return new Date(iso).toLocaleDateString(undefined, {
      month: "short",
      day: "numeric",
      year: "numeric",
    });
  } catch {
    return iso;
  }
}
</script>

<template>
  <div class="fixed inset-0 z-50 flex items-center justify-center bg-gray-950/90 backdrop-blur-sm p-4">
    <div class="w-full max-w-md max-h-[calc(100vh-2rem)] bg-gray-900 border border-gray-700 rounded-lg shadow-2xl overflow-hidden flex flex-col">

      <!-- Header -->
      <div class="px-6 py-4 border-b border-gray-700 flex items-center justify-between shrink-0">
        <div>
          <h2 class="text-lg font-bold tracking-widest uppercase text-gray-100 font-mono">
            Harsh Realm
          </h2>
          <p class="text-sm text-gray-500 font-mono mt-0.5">
            {{ worldStore.activeWorld ? worldStore.activeName : 'No world loaded' }}
          </p>
        </div>
        <button
          v-if="worldStore.activeWorld"
          class="text-gray-500 hover:text-gray-300 text-lg leading-none px-2"
          title="Close"
          @click="emit('close')"
        >&times;</button>
      </div>

      <!-- Tabs -->
      <div class="flex border-b border-gray-700 shrink-0">
        <button
          class="flex-1 py-2 text-sm font-mono font-medium transition-colors"
          :class="tab === 'load'
            ? 'text-amber-300 border-b-2 border-amber-400 bg-gray-800'
            : 'text-gray-500 hover:text-gray-300'"
          @click="tab = 'load'"
        >
          Load World
        </button>
        <button
          class="flex-1 py-2 text-sm font-mono font-medium transition-colors"
          :class="tab === 'create'
            ? 'text-amber-300 border-b-2 border-amber-400 bg-gray-800'
            : 'text-gray-500 hover:text-gray-300'"
          @click="tab = 'create'"
        >
          New World
        </button>
      </div>

      <!-- Error -->
      <div v-if="worldStore.error" class="mx-6 mt-4 px-3 py-2 bg-red-900/40 border border-red-700 rounded text-red-300 text-sm font-mono shrink-0">
        {{ worldStore.error }}
      </div>

      <!-- Load tab -->
      <div v-if="tab === 'load'" class="p-6 overflow-y-auto min-h-0">
        <div v-if="worldStore.available.length === 0" class="text-gray-500 text-sm font-mono text-center py-4">
          No saved worlds found. Create one to begin.
        </div>
        <ul v-else class="space-y-2">
          <li
            v-for="w in worldStore.available"
            :key="w.file"
            class="flex items-center justify-between px-4 py-3 bg-gray-800 rounded border border-gray-700
                   hover:border-amber-600 cursor-pointer transition-colors group"
            @click="handleLoad(w.file)"
          >
            <div>
              <div class="text-gray-100 font-mono text-sm group-hover:text-amber-200 transition-colors">
                {{ w.name }}
              </div>
              <div class="text-gray-600 text-xs font-mono mt-0.5">
                {{ formatDate(w.last_modified) }} · {{ w.file }}
              </div>
            </div>
            <span class="text-gray-600 group-hover:text-amber-400 text-lg leading-none transition-colors">›</span>
          </li>
        </ul>

        <button
          class="mt-4 w-full py-1.5 text-xs text-gray-600 hover:text-gray-400 font-mono transition-colors"
          @click="worldStore.fetchWorlds()"
        >
          Refresh list
        </button>
      </div>

      <!-- Create tab -->
      <div v-if="tab === 'create'" class="p-6 space-y-4 overflow-y-auto min-h-0">
        <div>
          <label class="block text-xs text-gray-500 font-mono mb-1">World name *</label>
          <input
            v-model="newName"
            type="text"
            placeholder="e.g. Ashfall"
            class="w-full bg-gray-800 border border-gray-600 rounded px-3 py-2 text-gray-100
                   font-mono text-sm placeholder-gray-600 outline-none focus:border-amber-500
                   transition-colors"
            @keydown.enter="handleCreate"
          />
        </div>

        <div class="flex gap-3">
          <div class="flex-1">
            <label class="block text-xs text-gray-500 font-mono mb-1">Width (5–100)</label>
            <input
              v-model.number="newWidth"
              type="number"
              min="5"
              max="100"
              class="w-full bg-gray-800 border border-gray-600 rounded px-3 py-2 text-gray-100
                     font-mono text-sm outline-none focus:border-amber-500 transition-colors"
            />
          </div>
          <div class="flex-1">
            <label class="block text-xs text-gray-500 font-mono mb-1">Height (5–100)</label>
            <input
              v-model.number="newHeight"
              type="number"
              min="5"
              max="100"
              class="w-full bg-gray-800 border border-gray-600 rounded px-3 py-2 text-gray-100
                     font-mono text-sm outline-none focus:border-amber-500 transition-colors"
            />
          </div>
        </div>

        <div>
          <label class="block text-xs text-gray-500 font-mono mb-1">Seed (optional — for reproducible maps)</label>
          <input
            v-model="newSeed"
            type="text"
            placeholder="Leave blank for random"
            class="w-full bg-gray-800 border border-gray-600 rounded px-3 py-2 text-gray-100
                   font-mono text-sm placeholder-gray-600 outline-none focus:border-amber-500
                   transition-colors"
            @keydown.enter="handleCreate"
          />
        </div>

        <PackPicker v-model="selectedPackIds" />

        <button
          class="w-full py-2.5 bg-amber-700 hover:bg-amber-600 disabled:bg-gray-700
                 disabled:text-gray-500 text-gray-100 font-mono text-sm font-semibold
                 rounded transition-colors"
          :disabled="worldStore.loading || !newName.trim()"
          @click="handleCreate"
        >
          {{ worldStore.loading ? "Generating…" : "Generate World" }}
        </button>

        <!-- Progress text -->
        <div v-if="worldStore.loading && progressText" class="mt-3 text-center">
          <div class="text-amber-400 text-sm font-mono animate-pulse">{{ progressText }}</div>
        </div>
      </div>

      <!-- Direct access to the content/admin tooling (HR-404) -->
      <div class="px-6 py-3 border-t border-gray-700 shrink-0">
        <p class="text-[11px] text-gray-600 font-mono mb-2 text-center uppercase tracking-wider">
          — or jump to —
        </p>
        <div class="flex gap-2">
          <RouterLink
            to="/content"
            data-testid="launch-content"
            class="flex-1 text-center py-2 text-xs font-mono rounded border border-gray-600
                   text-gray-300 hover:border-emerald-500 hover:text-emerald-300 transition-colors"
          >
            Content Studio
          </RouterLink>
          <RouterLink
            to="/admin"
            data-testid="launch-admin"
            class="flex-1 text-center py-2 text-xs font-mono rounded border border-gray-600
                   text-gray-300 hover:border-sky-500 hover:text-sky-300 transition-colors"
          >
            Admin Panel
          </RouterLink>
        </div>
      </div>

    </div>
  </div>
</template>
