<script setup lang="ts">
import { ref, watch, onMounted } from "vue";
import { useAdminStore } from "../../stores/admin";
import type { EncounterWeight } from "../../types/api";
import ImportExportBar from "./ImportExportBar.vue";

const store = useAdminStore();
const editEncounters = ref<EncounterWeight[]>([]);

function sync() {
    editEncounters.value = store.encounterWeights.map((e) => ({ ...e }));
}
watch(() => store.encounterWeights, sync, { deep: true });
onMounted(sync);

async function handleSaveEncounter(idx: number) {
    await store.saveEncounterWeight(editEncounters.value[idx]);
}
</script>

<template>
    <div>
                <div class="mb-4 text-sm text-gray-400 max-w-4xl">
                    <p class="mb-2">
                        <strong class="text-gray-200">Encounter Weights</strong>
                        adjust the probability of pulling specific encounter
                        tags based on the player's reputation with a faction.
                    </p>
                    <ul class="grid grid-cols-2 gap-x-6 gap-y-1 list-disc pl-5">
                        <li>
                            <strong>Disposition:</strong> The player's standing
                            level (Hostile to Helpful).
                        </li>
                        <li>
                            <strong>Tag:</strong> The encounter category (e.g.,
                            <code class="text-gray-300">patrol</code>,
                            <code class="text-gray-300">trader</code>).
                        </li>
                        <li>
                            <strong>Modifier:</strong> Multiplier for the base
                            weight.
                            <code class="text-gray-300">2.0</code> doubles the
                            frequency;
                            <code class="text-gray-300">0.0</code> disables it.
                        </li>
                    </ul>
                </div>
                <ImportExportBar
                    table="encounter_weights"
                    @imported="store.loadAllData()"
                />
                <table class="w-full text-sm">
                    <thead>
                        <tr
                            class="text-left text-gray-500 border-b border-gray-800"
                        >
                            <th class="px-2 py-1">Disposition</th>
                            <th class="px-2 py-1">Tag</th>
                            <th class="px-2 py-1">Modifier</th>
                            <th class="px-2 py-1">Actions</th>
                        </tr>
                    </thead>
                    <tbody>
                        <tr
                            v-for="(ew, idx) in editEncounters"
                            :key="`${ew.faction_disposition}-${ew.encounter_tag}`"
                            class="border-b border-gray-800/50 hover:bg-gray-900/50"
                        >
                            <td class="px-2 py-1 font-mono text-gray-300">
                                {{ ew.faction_disposition }}
                            </td>
                            <td class="px-2 py-1 font-mono text-gray-300">
                                {{ ew.encounter_tag }}
                            </td>
                            <td class="px-2 py-1">
                                <input
                                    v-model.number="ew.weight_modifier"
                                    type="number"
                                    step="0.1"
                                    class="bg-gray-800 border border-gray-700 text-gray-200 rounded px-1 py-0.5 w-20 text-xs focus:outline-none focus:border-gray-500"
                                />
                            </td>
                            <td class="px-2 py-1 whitespace-nowrap">
                                <button
                                    class="px-2 py-0.5 text-xs rounded border border-green-700 text-green-400 hover:bg-green-900/30"
                                    @click="handleSaveEncounter(idx)"
                                >
                                    Save
                                </button>
                            </td>
                        </tr>
                    </tbody>
                </table>
                <p
                    v-if="editEncounters.length === 0 && !store.loading"
                    class="text-gray-600 text-sm mt-4"
                >
                    No encounter weights found.
                </p>
    </div>
</template>
