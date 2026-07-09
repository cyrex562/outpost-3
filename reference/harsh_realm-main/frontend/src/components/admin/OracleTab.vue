<script setup lang="ts">
import { ref, onMounted } from "vue";
import { useAdminStore } from "../../stores/admin";
import type { OracleNpc, OracleThread } from "../../types/api";

const store = useAdminStore();

const newThreadTitle = ref("");
const newThreadType = ref("story");
const newNpcName = ref("");
const newNpcNotes = ref("");
const editingThreadId = ref<string | null>(null);
const editingNpcId = ref<string | null>(null);
const statusMessage = ref("");

onMounted(async () => {
  await Promise.all([
    store.loadThreads(),
    store.loadOracleNpcs(),
    store.loadOracleState(),
  ]);
});

async function addThread() {
  if (!newThreadTitle.value.trim()) return;
  await store.createThread({ title: newThreadTitle.value.trim(), type: newThreadType.value });
  newThreadTitle.value = "";
  statusMessage.value = "Thread created";
  setTimeout(() => { statusMessage.value = ""; }, 2000);
}

async function saveThread(thread: OracleThread) {
  await store.updateThread(thread.id, {
    title: thread.title,
    type: thread.type,
    status: thread.status,
    progress: thread.progress,
  });
  editingThreadId.value = null;
  statusMessage.value = "Thread saved";
  setTimeout(() => { statusMessage.value = ""; }, 2000);
}

async function removeThread(id: string) {
  await store.deleteThread(id);
  statusMessage.value = "Thread deleted";
  setTimeout(() => { statusMessage.value = ""; }, 2000);
}

async function addNpc() {
  if (!newNpcName.value.trim()) return;
  await store.createOracleNpc({ name: newNpcName.value.trim(), notes: newNpcNotes.value });
  newNpcName.value = "";
  newNpcNotes.value = "";
  statusMessage.value = "Oracle NPC created";
  setTimeout(() => { statusMessage.value = ""; }, 2000);
}

async function saveNpc(npc: OracleNpc) {
  await store.updateOracleNpc(npc.id, {
    name: npc.name,
    status: npc.status,
    notes: npc.notes,
  });
  editingNpcId.value = null;
  statusMessage.value = "NPC saved";
  setTimeout(() => { statusMessage.value = ""; }, 2000);
}

async function removeNpc(id: string) {
  await store.deleteOracleNpc(id);
  statusMessage.value = "Oracle NPC deleted";
  setTimeout(() => { statusMessage.value = ""; }, 2000);
}

async function setChaos(val: number) {
  await store.updateOracleState({ chaos_factor: val });
  statusMessage.value = `Chaos factor set to ${val}`;
  setTimeout(() => { statusMessage.value = ""; }, 2000);
}
</script>

<template>
  <div class="space-y-6">
    <p v-if="statusMessage" class="text-xs text-green-400">{{ statusMessage }}</p>

    <!-- Chaos Factor -->
    <div class="p-3 border border-gray-800 rounded bg-gray-900/50">
      <h3 class="text-sm font-mono text-gray-300 mb-2">Chaos Factor</h3>
      <div class="flex items-center gap-3">
        <input
          type="range"
          :value="store.oracleState.chaos_factor"
          min="1" max="9" step="1"
          class="w-48 accent-red-500"
          @change="setChaos(Number(($event.target as HTMLInputElement).value))"
        />
        <span
          class="text-lg font-bold font-mono"
          :class="{
            'text-green-400': store.oracleState.chaos_factor <= 3,
            'text-yellow-400': store.oracleState.chaos_factor >= 4 && store.oracleState.chaos_factor <= 6,
            'text-red-400': store.oracleState.chaos_factor >= 7,
          }"
        >
          {{ store.oracleState.chaos_factor }}
        </span>
        <span class="text-xs text-gray-500">
          (1 = ordered, 9 = chaotic)
        </span>
      </div>
    </div>

    <!-- Threads -->
    <div>
      <h3 class="text-sm font-mono text-gray-300 mb-2">Threads & Plotlines</h3>
      <div class="flex gap-2 mb-3">
        <input
          v-model="newThreadTitle"
          placeholder="New thread title..."
          class="flex-1 bg-gray-800 border border-gray-700 text-gray-200 rounded px-2 py-1 text-xs focus:outline-none focus:border-gray-500"
          @keydown.enter="addThread"
        />
        <select
          v-model="newThreadType"
          class="bg-gray-800 border border-gray-700 text-gray-200 rounded px-2 py-1 text-xs"
        >
          <option value="story">Story</option>
          <option value="character">Character</option>
        </select>
        <button
          class="px-3 py-1 text-xs rounded border border-green-700 text-green-400 hover:bg-green-900/30"
          @click="addThread"
        >
          Add
        </button>
      </div>
      <table class="w-full text-sm">
        <thead>
          <tr class="text-left text-gray-500 border-b border-gray-800">
            <th class="px-2 py-1">ID</th>
            <th class="px-2 py-1">Title</th>
            <th class="px-2 py-1">Type</th>
            <th class="px-2 py-1">Status</th>
            <th class="px-2 py-1">Progress</th>
            <th class="px-2 py-1">Actions</th>
          </tr>
        </thead>
        <tbody>
          <tr
            v-for="t in store.threads"
            :key="t.id"
            class="border-b border-gray-800/50 hover:bg-gray-900/50"
          >
            <td class="px-2 py-1 font-mono text-gray-500 text-xs">{{ t.id }}</td>
            <td class="px-2 py-1">
              <input
                v-if="editingThreadId === t.id"
                v-model="t.title"
                class="bg-gray-800 border border-gray-700 text-gray-200 rounded px-1 py-0.5 w-full text-xs"
              />
              <span v-else class="text-gray-300">{{ t.title }}</span>
            </td>
            <td class="px-2 py-1">
              <select
                v-if="editingThreadId === t.id"
                v-model="t.type"
                class="bg-gray-800 border border-gray-700 text-gray-200 rounded px-1 py-0.5 text-xs"
              >
                <option value="story">story</option>
                <option value="character">character</option>
              </select>
              <span v-else class="text-gray-400 text-xs">{{ t.type }}</span>
            </td>
            <td class="px-2 py-1">
              <select
                v-if="editingThreadId === t.id"
                v-model="t.status"
                class="bg-gray-800 border border-gray-700 text-gray-200 rounded px-1 py-0.5 text-xs"
              >
                <option value="active">active</option>
                <option value="resolved">resolved</option>
                <option value="abandoned">abandoned</option>
              </select>
              <span
                v-else
                class="text-xs px-1.5 py-0.5 rounded"
                :class="{
                  'bg-green-900/40 text-green-400': t.status === 'active',
                  'bg-blue-900/40 text-blue-400': t.status === 'resolved',
                  'bg-gray-800 text-gray-500': t.status === 'abandoned',
                }"
              >
                {{ t.status }}
              </span>
            </td>
            <td class="px-2 py-1">
              <input
                v-if="editingThreadId === t.id"
                v-model.number="t.progress"
                type="number"
                class="bg-gray-800 border border-gray-700 text-gray-200 rounded px-1 py-0.5 w-12 text-xs"
              />
              <span v-else class="text-gray-400">{{ t.progress }}</span>
            </td>
            <td class="px-2 py-1 whitespace-nowrap">
              <template v-if="editingThreadId === t.id">
                <button
                  class="px-2 py-0.5 text-xs rounded border border-green-700 text-green-400 hover:bg-green-900/30 mr-1"
                  @click="saveThread(t)"
                >
                  Save
                </button>
                <button
                  class="px-2 py-0.5 text-xs rounded border border-gray-600 text-gray-400 hover:bg-gray-800"
                  @click="editingThreadId = null"
                >
                  Cancel
                </button>
              </template>
              <template v-else>
                <button
                  class="px-2 py-0.5 text-xs rounded border border-blue-700 text-blue-400 hover:bg-blue-900/30 mr-1"
                  @click="editingThreadId = t.id"
                >
                  Edit
                </button>
                <button
                  class="px-2 py-0.5 text-xs rounded border border-red-700 text-red-400 hover:bg-red-900/30"
                  @click="removeThread(t.id)"
                >
                  Delete
                </button>
              </template>
            </td>
          </tr>
        </tbody>
      </table>
      <p v-if="store.threads.length === 0" class="text-gray-600 text-sm mt-2">
        No threads. Add one above.
      </p>
    </div>

    <!-- Oracle NPCs -->
    <div>
      <h3 class="text-sm font-mono text-gray-300 mb-2">Oracle NPC Cast List</h3>
      <div class="flex gap-2 mb-3">
        <input
          v-model="newNpcName"
          placeholder="NPC name..."
          class="bg-gray-800 border border-gray-700 text-gray-200 rounded px-2 py-1 text-xs focus:outline-none focus:border-gray-500"
          @keydown.enter="addNpc"
        />
        <input
          v-model="newNpcNotes"
          placeholder="Notes..."
          class="flex-1 bg-gray-800 border border-gray-700 text-gray-200 rounded px-2 py-1 text-xs focus:outline-none focus:border-gray-500"
        />
        <button
          class="px-3 py-1 text-xs rounded border border-green-700 text-green-400 hover:bg-green-900/30"
          @click="addNpc"
        >
          Add
        </button>
      </div>
      <table class="w-full text-sm">
        <thead>
          <tr class="text-left text-gray-500 border-b border-gray-800">
            <th class="px-2 py-1">ID</th>
            <th class="px-2 py-1">Name</th>
            <th class="px-2 py-1">Status</th>
            <th class="px-2 py-1">Notes</th>
            <th class="px-2 py-1">Actions</th>
          </tr>
        </thead>
        <tbody>
          <tr
            v-for="npc in store.oracleNpcs"
            :key="npc.id"
            class="border-b border-gray-800/50 hover:bg-gray-900/50"
          >
            <td class="px-2 py-1 font-mono text-gray-500 text-xs">{{ npc.id }}</td>
            <td class="px-2 py-1">
              <input
                v-if="editingNpcId === npc.id"
                v-model="npc.name"
                class="bg-gray-800 border border-gray-700 text-gray-200 rounded px-1 py-0.5 w-full text-xs"
              />
              <span v-else class="text-gray-300">{{ npc.name }}</span>
            </td>
            <td class="px-2 py-1">
              <select
                v-if="editingNpcId === npc.id"
                v-model="npc.status"
                class="bg-gray-800 border border-gray-700 text-gray-200 rounded px-1 py-0.5 text-xs"
              >
                <option value="active">active</option>
                <option value="inactive">inactive</option>
              </select>
              <span v-else class="text-xs text-gray-400">{{ npc.status }}</span>
            </td>
            <td class="px-2 py-1">
              <input
                v-if="editingNpcId === npc.id"
                v-model="npc.notes"
                class="bg-gray-800 border border-gray-700 text-gray-200 rounded px-1 py-0.5 w-full text-xs"
              />
              <span v-else class="text-gray-500 text-xs">{{ npc.notes }}</span>
            </td>
            <td class="px-2 py-1 whitespace-nowrap">
              <template v-if="editingNpcId === npc.id">
                <button
                  class="px-2 py-0.5 text-xs rounded border border-green-700 text-green-400 hover:bg-green-900/30 mr-1"
                  @click="saveNpc(npc)"
                >
                  Save
                </button>
                <button
                  class="px-2 py-0.5 text-xs rounded border border-gray-600 text-gray-400 hover:bg-gray-800"
                  @click="editingNpcId = null"
                >
                  Cancel
                </button>
              </template>
              <template v-else>
                <button
                  class="px-2 py-0.5 text-xs rounded border border-blue-700 text-blue-400 hover:bg-blue-900/30 mr-1"
                  @click="editingNpcId = npc.id"
                >
                  Edit
                </button>
                <button
                  class="px-2 py-0.5 text-xs rounded border border-red-700 text-red-400 hover:bg-red-900/30"
                  @click="removeNpc(npc.id)"
                >
                  Delete
                </button>
              </template>
            </td>
          </tr>
        </tbody>
      </table>
      <p v-if="store.oracleNpcs.length === 0" class="text-gray-600 text-sm mt-2">
        No oracle NPCs. Add one above.
      </p>
    </div>
  </div>
</template>
