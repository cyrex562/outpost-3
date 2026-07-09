<script setup lang="ts">
import { ref, watch, onMounted } from "vue";
import { useAdminStore } from "../../stores/admin";
import type { FactionAssetStat } from "../../types/api";
import ImportExportBar from "./ImportExportBar.vue";

const store = useAdminStore();
const editFaction = ref<FactionAssetStat[]>([]);

function sync() {
    editFaction.value = store.factionAssets.map((f) => ({ ...f }));
}
watch(() => store.factionAssets, sync, { deep: true });
onMounted(sync);
</script>

<template>
    <div>
                <div class="mb-4 text-sm text-gray-400 max-w-4xl">
                    <p class="mb-2">
                        <strong class="text-gray-200">Faction Assets</strong>
                        are the units and resources factions use during their
                        turns. These stats are read-only and defined by the core
                        rules.
                    </p>
                    <ul class="grid grid-cols-2 gap-x-6 gap-y-1 list-disc pl-5">
                        <li>
                            <strong>Type:</strong> The unique ID and
                            human-readable name of the asset.
                        </li>
                        <li>
                            <strong>Min Attr:</strong> The minimum faction
                            attribute (Force, Cunning, Wealth) required to buy
                            it.
                        </li>
                        <li>
                            <strong>Attack/Counter:</strong> The attribute and
                            dice used when this asset attacks or defends.
                        </li>
                    </ul>
                </div>
                <ImportExportBar
                    table="faction_asset_stats"
                    @imported="store.loadAllData()"
                />
                <div class="overflow-x-auto">
                    <table class="w-full text-sm">
                        <thead>
                            <tr
                                class="text-left text-gray-500 border-b border-gray-800"
                            >
                                <th class="px-2 py-1">Type</th>
                                <th class="px-2 py-1">Category</th>
                                <th class="px-2 py-1">Min Attr</th>
                                <th class="px-2 py-1">Cost</th>
                                <th class="px-2 py-1">Upkeep</th>
                                <th class="px-2 py-1">Max HP</th>
                                <th class="px-2 py-1">Attack</th>
                                <th class="px-2 py-1">Counter</th>
                                <th class="px-2 py-1">Description</th>
                            </tr>
                        </thead>
                        <tbody>
                            <tr
                                v-for="fa in editFaction"
                                :key="fa.asset_type"
                                class="border-b border-gray-800/50 hover:bg-gray-900/50"
                            >
                                <td class="px-2 py-1 font-mono text-gray-300">
                                    <div class="font-bold text-gray-100">
                                        {{
                                            fa.asset_type
                                                .replace(/_/g, " ")
                                                .replace(/\b\w/g, (l) =>
                                                    l.toUpperCase(),
                                                )
                                        }}
                                    </div>
                                    <div class="text-[10px] text-gray-500">
                                        {{ fa.asset_type }}
                                    </div>
                                </td>
                                <td class="px-2 py-1 text-gray-400">
                                    {{ fa.category }}
                                </td>
                                <td class="px-2 py-1 text-gray-400">
                                    {{ fa.min_attribute }}
                                </td>
                                <td class="px-2 py-1 text-gray-400">
                                    {{ fa.cost }}
                                </td>
                                <td class="px-2 py-1 text-gray-400">
                                    {{ fa.upkeep }}
                                </td>
                                <td class="px-2 py-1 text-gray-400">
                                    {{ fa.max_hp }}
                                </td>
                                <td class="px-2 py-1 text-gray-400 text-xs">
                                    {{ fa.attack_stat }} / {{ fa.attack_roll }}
                                </td>
                                <td class="px-2 py-1 text-gray-400 text-xs">
                                    {{ fa.counter_stat }}
                                </td>
                                <td
                                    class="px-2 py-1 text-gray-400 text-xs max-w-xs truncate"
                                >
                                    {{ fa.description }}
                                </td>
                            </tr>
                        </tbody>
                    </table>
                </div>
                <p
                    v-if="editFaction.length === 0 && !store.loading"
                    class="text-gray-600 text-sm mt-4"
                >
                    No faction assets found.
                </p>
    </div>
</template>
