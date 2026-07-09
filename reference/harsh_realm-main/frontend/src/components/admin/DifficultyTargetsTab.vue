<script setup lang="ts">
import { ref, watch, onMounted } from "vue";
import { useAdminStore } from "../../stores/admin";
import type { DifficultyTarget } from "../../types/api";
import ImportExportBar from "./ImportExportBar.vue";

const store = useAdminStore();
const editDifficulties = ref<DifficultyTarget[]>([]);

function sync() {
    editDifficulties.value = store.difficultyTargets.map((d) => ({ ...d }));
}
watch(() => store.difficultyTargets, sync, { deep: true });
onMounted(sync);

async function handleSaveDifficulty(idx: number) {
    await store.saveDifficultyTarget(editDifficulties.value[idx]);
}
async function handleResetDifficulty(name: string) {
    await store.resetDifficultyTarget(name);
}
</script>

<template>
    <div>
                <ImportExportBar
                    table="difficulty_targets"
                    @imported="store.loadAllData()"
                />
                <table class="w-full text-sm">
                    <thead>
                        <tr
                            class="text-left text-gray-500 border-b border-gray-800"
                        >
                            <th class="px-2 py-1">Name</th>
                            <th class="px-2 py-1">Target</th>
                            <th class="px-2 py-1">Description</th>
                            <th class="px-2 py-1">Actions</th>
                        </tr>
                    </thead>
                    <tbody>
                        <tr
                            v-for="(dt, idx) in editDifficulties"
                            :key="dt.name"
                            :data-testid="`difficulty-row-${dt.name}`"
                            class="border-b border-gray-800/50 hover:bg-gray-900/50"
                        >
                            <td class="px-2 py-1 font-mono text-gray-300">
                                {{ dt.name }}
                            </td>
                            <td class="px-2 py-1">
                                <input
                                    v-model.number="dt.target"
                                    :data-testid="`difficulty-target-${dt.name}`"
                                    type="number"
                                    class="bg-gray-800 border border-gray-700 text-gray-200 rounded px-1 py-0.5 w-16 text-xs focus:outline-none focus:border-gray-500"
                                />
                            </td>
                            <td class="px-2 py-1">
                                <input
                                    v-model="dt.description"
                                    class="bg-gray-800 border border-gray-700 text-gray-200 rounded px-1 py-0.5 w-full text-xs focus:outline-none focus:border-gray-500"
                                />
                            </td>
                            <td class="px-2 py-1 whitespace-nowrap">
                                <button
                                    :data-testid="`difficulty-save-${dt.name}`"
                                    class="px-2 py-0.5 text-xs rounded border border-green-700 text-green-400 hover:bg-green-900/30 mr-1"
                                    @click="handleSaveDifficulty(idx)"
                                >
                                    Save
                                </button>
                                <button
                                    :data-testid="`difficulty-reset-${dt.name}`"
                                    class="px-2 py-0.5 text-xs rounded border border-yellow-700 text-yellow-400 hover:bg-yellow-900/30"
                                    @click="handleResetDifficulty(dt.name)"
                                >
                                    Reset
                                </button>
                            </td>
                        </tr>
                    </tbody>
                </table>
                <p
                    v-if="editDifficulties.length === 0 && !store.loading"
                    class="text-gray-600 text-sm mt-4"
                >
                    No difficulty targets found.
                </p>
    </div>
</template>
