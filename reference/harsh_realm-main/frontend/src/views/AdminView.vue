<script setup lang="ts">
import { ref, watch, onMounted } from "vue";
import { RouterLink } from "vue-router";
import { useAdminStore } from "../stores/admin";
import { useConfigStore } from "../stores/config";
import MapEditorTab from "../components/admin/MapEditorTab.vue";
import CharacterEditorTab from "../components/admin/CharacterEditorTab.vue";
import FactionEditorTab from "../components/admin/FactionEditorTab.vue";
import DungeonEditorTab from "../components/admin/DungeonEditorTab.vue";
import WorldsTab from "../components/admin/WorldsTab.vue";
import YAMLFilesTab from "../components/admin/YAMLFilesTab.vue";
import WorldMetaTab from "../components/admin/WorldMetaTab.vue";
import CreatureEditorTab from "../components/admin/CreatureEditorTab.vue";
import ItemEditorTab from "../components/admin/ItemEditorTab.vue";
import TableEditorTab from "../components/admin/TableEditorTab.vue";
import OracleTab from "../components/admin/OracleTab.vue";
import GMCommandPanel from "../components/admin/GMCommandPanel.vue";
import LiveEventFeed from "../components/admin/LiveEventFeed.vue";
import SkillMappingsTab from "../components/admin/SkillMappingsTab.vue";
import DifficultyTargetsTab from "../components/admin/DifficultyTargetsTab.vue";
import DispositionOutcomesTab from "../components/admin/DispositionOutcomesTab.vue";
import EncounterWeightsTab from "../components/admin/EncounterWeightsTab.vue";
import FactionAssetStatsTab from "../components/admin/FactionAssetStatsTab.vue";
import ProgressionTab from "../components/admin/ProgressionTab.vue";

const store = useAdminStore();
const configStore = useConfigStore();
const activeTab = ref<string>("skills");
const exportMessage = ref<string>("");

const tabs = [
    { id: "skills", label: "Skill Mappings" },
    { id: "difficulties", label: "Difficulties" },
    { id: "disposition", label: "Disposition" },
    { id: "encounters", label: "Encounter Weights" },
    { id: "faction", label: "Faction Assets" },
    { id: "map", label: "Map" },
    { id: "characters", label: "Characters" },
    { id: "factions-world", label: "Factions (World)" },
    { id: "dungeons", label: "Dungeons" },
    { id: "worlds", label: "Worlds" },
    { id: "yaml-files", label: "YAML Files" },
    { id: "world-meta", label: "World Meta" },
    { id: "progression", label: "XP Progression" },
    { id: "creatures", label: "Creatures" },
    { id: "items", label: "Items" },
    { id: "tables", label: "Random Tables" },
    { id: "oracle", label: "Oracle & Threads" },
    { id: "gm-commands", label: "GM Commands" },
    { id: "live-events", label: "Live Feed" },
];


watch(
    () => store.activeWorldPath,
    async () => {
        await store.loadAllData();
    },
);

onMounted(async () => {
    await configStore.load();
    await store.loadWorlds();
    await store.loadAllData();
});

async function handleExport() {
    try {
        const result = await store.exportConfig();
        exportMessage.value = result.path
            ? `Exported to ${result.path}`
            : JSON.stringify(result);
        setTimeout(() => {
            exportMessage.value = "";
        }, 5000);
    } catch (e) {
        exportMessage.value = `Export failed: ${e}`;
    }
}
</script>

<template>
    <div class="flex flex-col h-screen bg-gray-950 text-gray-100">
        <!-- Header -->
        <header
            class="flex items-center justify-between px-4 py-2 border-b border-gray-800 bg-gray-900 shrink-0"
        >
            <div class="flex items-center gap-3">
                <h1
                    class="text-lg font-bold tracking-widest uppercase text-gray-100"
                >
                    Admin Panel
                </h1>
                <!-- World selector -->
                <select
                    v-model="store.activeWorldPath"
                    class="bg-gray-800 border border-gray-700 text-gray-200 text-xs rounded px-2 py-1 focus:outline-none focus:border-gray-500"
                >
                    <option value="">Default (no world)</option>
                    <option
                        v-for="w in store.availableWorlds"
                        :key="w.file"
                        :value="w.file"
                    >
                        {{ w.name }} ({{ w.file }})
                    </option>
                </select>
                <span
                    data-testid="admin-config-chip"
                    class="text-[10px] font-mono px-2 py-0.5 rounded border"
                    :class="configStore.adminMode
                        ? 'border-emerald-700 text-emerald-400'
                        : 'border-gray-700 text-gray-500'"
                    :title="configStore.adminMode
                        ? 'Admin mode on — all tools enabled.'
                        : 'Admin mode off — procedure runner and gated tools are disabled.'"
                >
                    {{ configStore.runMode }} · admin {{ configStore.adminMode ? "on" : "off" }}
                </span>
            </div>
            <div class="flex items-center gap-3 text-sm font-mono">
                <span v-if="exportMessage" class="text-xs text-green-400">{{
                    exportMessage
                }}</span>
                <button
                    data-testid="admin-export-btn"
                    class="px-2 py-1 text-xs font-mono rounded border border-blue-600 text-blue-400 hover:bg-blue-900/30 transition-colors select-none"
                    @click="handleExport"
                >
                    Export Config
                </button>
                <button
                    data-testid="admin-refresh-btn"
                    class="px-2 py-1 text-xs font-mono rounded border border-gray-600 text-gray-400 hover:border-gray-400 hover:text-gray-200 transition-colors select-none"
                    @click="store.loadAllData()"
                >
                    Refresh
                </button>
                <RouterLink
                    to="/"
                    class="px-2 py-1 text-xs font-mono rounded border border-gray-600 text-gray-400 hover:border-gray-400 hover:text-gray-200 transition-colors select-none"
                >
                    Back to Game
                </RouterLink>
            </div>
        </header>

        <!-- Loading / Error -->
        <div
            v-if="store.loading"
            class="px-4 py-2 text-sm text-yellow-400 bg-gray-900"
        >
            Loading...
        </div>
        <div
            v-if="store.error"
            class="px-4 py-2 text-sm text-red-400 bg-gray-900"
        >
            Error: {{ store.error }}
        </div>

        <!-- Tab bar -->
        <div
            class="flex overflow-x-auto border-b border-gray-800 bg-gray-900 px-4 shrink-0"
        >
            <button
                v-for="tab in tabs"
                :key="tab.id"
                :data-testid="`admin-tab-${tab.id}`"
                class="px-4 py-2 text-sm font-mono border-b-2 transition-colors whitespace-nowrap"
                :class="
                    activeTab === tab.id
                        ? 'border-blue-500 text-blue-400'
                        : 'border-transparent text-gray-500 hover:text-gray-300'
                "
                @click="activeTab = tab.id"
            >
                {{ tab.label }}
            </button>
        </div>

        <!-- Tab content -->
        <div class="flex-1 overflow-auto p-4">
            <!-- Config tabs -->
            <SkillMappingsTab v-if="activeTab === 'skills'" />
            <DifficultyTargetsTab v-if="activeTab === 'difficulties'" />
            <DispositionOutcomesTab v-if="activeTab === 'disposition'" />
            <EncounterWeightsTab v-if="activeTab === 'encounters'" />
            <FactionAssetStatsTab v-if="activeTab === 'faction'" />
            <ProgressionTab v-if="activeTab === 'progression'" />


            <!-- M4.5 Editor Tabs -->
            <MapEditorTab v-if="activeTab === 'map'" />
            <CharacterEditorTab v-if="activeTab === 'characters'" />
            <FactionEditorTab v-if="activeTab === 'factions-world'" />
            <DungeonEditorTab v-if="activeTab === 'dungeons'" />
            <WorldsTab v-if="activeTab === 'worlds'" />
            <YAMLFilesTab v-if="activeTab === 'yaml-files'" />
            <WorldMetaTab v-if="activeTab === 'world-meta'" />
            <CreatureEditorTab v-if="activeTab === 'creatures'" />
            <ItemEditorTab v-if="activeTab === 'items'" />
            <TableEditorTab v-if="activeTab === 'tables'" />
            <OracleTab v-if="activeTab === 'oracle'" />
            <GMCommandPanel v-if="activeTab === 'gm-commands'" />
            <LiveEventFeed v-if="activeTab === 'live-events'" />
        </div>
    </div>
</template>
