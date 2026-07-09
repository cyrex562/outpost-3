<script setup lang="ts">
import { useAdminStore } from "../../stores/admin";

const store = useAdminStore();
</script>

<template>
    <div>
                <div class="mb-4 text-sm text-gray-400 max-w-4xl">
                    <p class="mb-2">
                        <strong class="text-gray-200">XP Progression</strong>
                        defines the total XP needed to reach each level.
                    </p>
                    <ul class="list-disc pl-5">
                        <li>
                            <strong>Level:</strong> The level reached (1-10).
                            Level 1 is always 0 XP.
                        </li>
                        <li>
                            <strong>XP Threshold:</strong> The total cumulative
                            XP required.
                        </li>
                    </ul>
                </div>
                <div class="flex items-center gap-2 mb-2">
                    <button
                        class="px-2 py-1 text-xs rounded border border-red-800 text-red-400 hover:bg-red-900/30"
                        @click="store.resetAllXpProgression()"
                    >
                        Reset All to Default
                    </button>
                </div>
                <div class="overflow-x-auto">
                    <table class="w-full text-sm">
                        <thead>
                            <tr
                                class="text-left text-gray-500 border-b border-gray-800"
                            >
                                <th class="px-2 py-1">Level</th>
                                <th class="px-2 py-1">XP Threshold (Total)</th>
                                <th class="px-2 py-1">Actions</th>
                            </tr>
                        </thead>
                        <tbody>
                            <tr
                                v-for="xp in store.xpProgression"
                                :key="xp.level"
                                class="border-b border-gray-800/50 hover:bg-gray-900/50"
                            >
                                <td class="px-2 py-1 font-mono text-gray-100">
                                    {{ xp.level }}
                                </td>
                                <td class="px-2 py-1">
                                    <input
                                        v-model.number="xp.xp_needed"
                                        type="number"
                                        :disabled="xp.level === 1"
                                        class="bg-gray-800 border border-gray-700 text-gray-200 rounded px-1 py-0.5 w-32 text-xs focus:outline-none focus:border-gray-500"
                                        @change="
                                            store.saveXpProgression(
                                                xp.level,
                                                xp.xp_needed,
                                            )
                                        "
                                    />
                                </td>
                                <td class="px-2 py-1">
                                    <button
                                        class="text-[10px] text-gray-500 hover:text-gray-300"
                                        @click="
                                            store.resetXpProgression(xp.level)
                                        "
                                    >
                                        Reset
                                    </button>
                                </td>
                            </tr>
                        </tbody>
                    </table>
                </div>
    </div>
</template>
