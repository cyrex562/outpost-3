<script setup lang="ts">
import { ref, watch, onMounted } from "vue";
import { useAdminStore } from "../../stores/admin";
import type { DispositionOutcome } from "../../types/api";
import ImportExportBar from "./ImportExportBar.vue";

const store = useAdminStore();
const editDisposition = ref<DispositionOutcome[]>([]);

function sync() {
    editDisposition.value = store.dispositionOutcomes.map((d) => ({ ...d }));
}
watch(() => store.dispositionOutcomes, sync, { deep: true });
onMounted(sync);

async function handleSaveDisposition(idx: number) {
    await store.saveDispositionOutcome(editDisposition.value[idx]);
}
</script>

<template>
    <div>
                <div class="mb-4 text-sm text-gray-400 max-w-4xl">
                    <p class="mb-2">
                        <strong class="text-gray-200"
                            >Disposition Outcomes</strong
                        >
                        define how specific social events affect an NPC's mood
                        (disposition).
                    </p>
                    <ul class="grid grid-cols-2 gap-x-6 gap-y-1 list-disc pl-5">
                        <li>
                            <strong>Outcome Key:</strong> The internal event ID
                            (e.g., <code class="text-blue-400">success</code>,
                            <code class="text-red-400">failure</code>) from a
                            skill check.
                        </li>
                        <li>
                            <strong>Delta:</strong> The amount added to or
                            subtracted from the NPC's raw disposition score.
                        </li>
                    </ul>
                </div>
                <ImportExportBar
                    table="disposition_outcomes"
                    @imported="store.loadAllData()"
                />
                <table class="w-full text-sm">
                    <thead>
                        <tr
                            class="text-left text-gray-500 border-b border-gray-800"
                        >
                            <th class="px-2 py-1">Outcome Key</th>
                            <th class="px-2 py-1">Delta</th>
                            <th class="px-2 py-1">Description</th>
                            <th class="px-2 py-1">Actions</th>
                        </tr>
                    </thead>
                    <tbody>
                        <tr
                            v-for="(d, idx) in editDisposition"
                            :key="d.outcome_key"
                            class="border-b border-gray-800/50 hover:bg-gray-900/50"
                        >
                            <td class="px-2 py-1 font-mono text-gray-300">
                                {{ d.outcome_key }}
                            </td>
                            <td class="px-2 py-1">
                                <input
                                    v-model.number="d.delta"
                                    type="number"
                                    class="bg-gray-800 border border-gray-700 text-gray-200 rounded px-1 py-0.5 w-16 text-xs focus:outline-none focus:border-gray-500"
                                />
                            </td>
                            <td class="px-2 py-1">
                                <input
                                    v-model="d.description"
                                    class="bg-gray-800 border border-gray-700 text-gray-200 rounded px-1 py-0.5 w-full text-xs focus:outline-none focus:border-gray-500"
                                />
                            </td>
                            <td class="px-2 py-1 whitespace-nowrap">
                                <button
                                    class="px-2 py-0.5 text-xs rounded border border-green-700 text-green-400 hover:bg-green-900/30"
                                    @click="handleSaveDisposition(idx)"
                                >
                                    Save
                                </button>
                            </td>
                        </tr>
                    </tbody>
                </table>
                <p
                    v-if="editDisposition.length === 0 && !store.loading"
                    class="text-gray-600 text-sm mt-4"
                >
                    No disposition outcomes found.
                </p>
    </div>
</template>
