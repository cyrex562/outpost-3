<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref, watch } from "vue";
import { RouterLink } from "vue-router";
import { useConnectionStore } from "../stores/connection";
import { useWorldStore } from "../stores/world";
import { useLayoutStore } from "../stores/layout";
import { useWebSocket } from "../composables/useWebSocket";
import WorldManager from "../components/WorldManager.vue";
import WindowManager from "../components/WindowManager.vue";
import PanelWindow from "../components/PanelWindow.vue";
import ChatPanel from "../components/ChatPanel.vue";
import SquareMap from "../components/SquareMap.vue";
import TownMap from "../components/TownMap.vue";
import StatusSidebar from "../components/StatusSidebar.vue";
import ReferencePanel from "../components/ReferencePanel.vue";
import InventoryPanel from "../components/InventoryPanel.vue";
import CombatPanel from "../components/CombatPanel.vue";
import EncounterWindow from "../components/EncounterWindow.vue";
import ActionBar from "../components/ActionBar.vue";
import { useTownStore } from "../stores/town";
import { useGameStore } from "../stores/game";
import { useLootStore } from "../stores/loot";
import LootPanel from "../components/LootPanel.vue";

const connectionStore = useConnectionStore();
const worldStore = useWorldStore();
const townStore = useTownStore();
const layoutStore = useLayoutStore();
const gameStore = useGameStore();
const lootStore = useLootStore();
const { send } = useWebSocket();

// Show the encounter panel during an active combat OR when a pre-combat menu
// is staged (phase "pre_combat" = encounter happening, not yet in combat).
const showEncounter = computed(
    () =>
        gameStore.currentScene === "combat" ||
        gameStore.combatPhase === "pre_combat",
);

const worldLoaded = computed(() => worldStore.activeWorld !== null);
const showWorldManager = ref(false);

// Build stamp (git short-hash + build time), injected by vite.config.ts. Shown in
// the header so a stale build is obvious: the hash must match the deployed commit.
const buildStamp = __BUILD_STAMP__;

function toggleWorldManager() {
    showWorldManager.value = !showWorldManager.value;
}

// Bring town map to front when entering a town
watch(
    () => townStore.active,
    (active) => {
        if (active) {
            layoutStore.updatePanel("townmap", { visible: true });
            layoutStore.bringToFront("townmap");
        }
    },
);

// HR-786: auto-show the loot panel while any loot source is revealed; hide it
// once everything is taken or the player leaves the cell.
watch(
    () => lootStore.sources.length,
    (count) => {
        if (count > 0) {
            layoutStore.updatePanel("loot", { visible: true });
            layoutStore.bringToFront("loot");
        } else {
            layoutStore.updatePanel("loot", { visible: false });
        }
    },
);

// Bring combat panel and encounter window to front when entering combat.
watch(
    () => gameStore.currentScene,
    (scene) => {
        if (scene === "combat") {
            layoutStore.updatePanel("combat", { visible: true });
            layoutStore.bringToFront("combat");
            layoutStore.updatePanel("encounter", { visible: true });
            layoutStore.bringToFront("encounter");
        }
    },
);

// Also bring the encounter window to front when a pre-combat encounter is staged.
watch(
    () => gameStore.combatPhase,
    (phase) => {
        if (phase === "pre_combat") {
            layoutStore.updatePanel("encounter", { visible: true });
            layoutStore.bringToFront("encounter");
        }
    },
);

onMounted(async () => {
    // Try to restore saved layout from server
    layoutStore.loadLayout();
});

// --- WASD movement keys (active when text input is not focused) -----------
const MOVE_KEYS: Record<string, string> = {
    w: "n",
    a: "w",
    s: "s",
    d: "e",
    q: "nw",
    e: "ne",
    z: "sw",
    c: "se",
};

function onGlobalKeydown(ev: KeyboardEvent) {
    // Ignore when typing in an input/textarea or when modifiers are held
    const tag = (ev.target as HTMLElement)?.tagName;
    if (tag === "INPUT" || tag === "TEXTAREA" || tag === "SELECT") return;
    if (ev.ctrlKey || ev.metaKey || ev.altKey) return;

    const dir = MOVE_KEYS[ev.key.toLowerCase()];
    if (dir && worldLoaded.value) {
        ev.preventDefault();
        send(dir);
    }
}

document.addEventListener("keydown", onGlobalKeydown);
onUnmounted(() => document.removeEventListener("keydown", onGlobalKeydown));

/** Send the tile-action command when the user clicks an exploration action button. */
function onExplorationAction(command: string): void {
  send(command);
}

const statusLabel: Record<string, string> = {
    connecting: "Connecting...",
    connected: "Connected",
    disconnected: "Disconnected",
    reconnecting: "Reconnecting...",
};

const statusColor: Record<string, string> = {
    connecting: "text-yellow-400",
    connected: "text-green-400",
    disconnected: "text-red-500",
    reconnecting: "text-yellow-400",
};
</script>

<template>
    <div class="flex flex-col h-screen bg-gray-950 text-gray-100">
        <!-- Fixed header bar -->
        <header
            class="flex items-center justify-between px-4 py-2 border-b border-gray-800 bg-gray-900 shrink-0 z-50 relative"
        >
            <div class="flex items-center gap-3">
                <h1
                    class="text-lg font-bold tracking-widest uppercase text-gray-100"
                >
                    Harsh Realm
                </h1>
                <span
                    v-if="worldStore.activeName"
                    class="text-xs text-gray-500 font-mono"
                >
                    {{ worldStore.activeName }}
                </span>
                <span
                    data-testid="build-stamp"
                    class="text-[10px] text-gray-600 font-mono"
                    title="Frontend build stamp (git short-hash + build time). If this doesn't match the deployed commit, the build didn't take."
                >
                    build {{ buildStamp }}
                </span>
            </div>
            <div class="flex items-center gap-3 text-sm font-mono">
                <!-- Worlds button -->
                <button
                    class="px-2 py-1 text-xs font-mono rounded border transition-colors select-none"
                    :class="
                        showWorldManager
                            ? 'border-blue-600 text-blue-400 bg-blue-900/30'
                            : 'border-gray-600 text-gray-400 hover:border-gray-400 hover:text-gray-200'
                    "
                    @click="toggleWorldManager"
                >
                    Worlds
                </button>
                <!-- Admin link -->
                <RouterLink
                    to="/admin"
                    class="px-2 py-1 text-xs font-mono rounded border border-gray-600 text-gray-400 hover:border-gray-400 hover:text-gray-200 transition-colors select-none"
                >
                    Admin
                </RouterLink>
                <!-- Content authoring link -->
                <RouterLink
                    to="/content"
                    class="px-2 py-1 text-xs font-mono rounded border border-gray-600 text-gray-400 hover:border-gray-400 hover:text-gray-200 transition-colors select-none"
                >
                    Content
                </RouterLink>
                <!-- Difficulty settings link -->
                <RouterLink
                    to="/difficulty"
                    data-testid="difficulty-nav"
                    class="px-2 py-1 text-xs font-mono rounded border border-gray-600 text-gray-400 hover:border-gray-400 hover:text-gray-200 transition-colors select-none"
                >
                    Difficulty
                </RouterLink>
                <!-- Reference panel toggle -->
                <button
                    class="px-2 py-1 text-xs font-mono rounded border transition-colors select-none"
                    :class="
                        layoutStore.panels['ref']?.visible
                            ? 'border-green-600 text-green-400 bg-green-900/30'
                            : 'border-gray-600 text-gray-400 hover:border-gray-400 hover:text-gray-200'
                    "
                    title="Toggle Reference panel (skills, commands)"
                    @click="
                        () => {
                            const show = !layoutStore.panels['ref']?.visible;
                            layoutStore.updatePanel('ref', { visible: show });
                            if (show) layoutStore.bringToFront('ref');
                        }
                    "
                >
                    ? Ref
                </button>
                <!-- Inventory panel toggle -->
                <button
                    class="px-2 py-1 text-xs font-mono rounded border transition-colors select-none"
                    :class="
                        layoutStore.panels['inv']?.visible
                            ? 'border-amber-600 text-amber-400 bg-amber-900/30'
                            : 'border-gray-600 text-gray-400 hover:border-gray-400 hover:text-gray-200'
                    "
                    title="Toggle Inventory panel"
                    @click="
                        () => {
                            const show = !layoutStore.panels['inv']?.visible;
                            layoutStore.updatePanel('inv', { visible: show });
                            if (show) layoutStore.bringToFront('inv');
                        }
                    "
                >
                    Inv
                </button>
                <!-- Connection status -->
                <span
                    class="w-2 h-2 rounded-full"
                    :class="{
                        'bg-green-400': connectionStore.status === 'connected',
                        'bg-yellow-400':
                            connectionStore.status === 'connecting' ||
                            connectionStore.status === 'reconnecting',
                        'bg-red-500': connectionStore.status === 'disconnected',
                    }"
                />
                <span
                    data-testid="connection-status"
                    :class="statusColor[connectionStore.status]"
                >
                    {{ statusLabel[connectionStore.status] }}
                </span>
            </div>
        </header>

        <!-- Window manager fills area below header -->
        <div class="flex-1 relative overflow-hidden">
            <WindowManager>
                <!-- Chat panel -->
                <PanelWindow
                    panel-id="chat"
                    title="Chat / Log"
                    :min-width="300"
                    :min-height="200"
                >
                    <ChatPanel :send="send" />
                </PanelWindow>

                <!-- Map panel -->
                <PanelWindow
                    panel-id="map"
                    title="Map"
                    :min-width="250"
                    :min-height="200"
                >
                    <div class="flex flex-col h-full">
                        <SquareMap class="flex-1 min-h-0" />
                        <!-- Exploration tile-action bar (HR-112): shown only in
                             exploration scene outside of pre-combat. -->
                        <ActionBar
                            v-if="
                                gameStore.currentScene === 'exploration' &&
                                gameStore.combatPhase !== 'pre_combat' &&
                                gameStore.explorationActions.length > 0
                            "
                            :actions="gameStore.explorationActions"
                            @action="onExplorationAction"
                        />
                    </div>
                </PanelWindow>

                <!-- Town map panel (auto-shows when in town) -->
                <PanelWindow
                    v-if="townStore.active"
                    panel-id="townmap"
                    title="Town Map"
                    :min-width="250"
                    :min-height="250"
                    :closable="true"
                >
                    <TownMap />
                </PanelWindow>

                <!-- Status panel -->
                <PanelWindow
                    panel-id="status"
                    title="Status"
                    :min-width="180"
                    :min-height="150"
                >
                    <StatusSidebar />
                </PanelWindow>

                <!-- Reference panel (hidden by default, toggled from header) -->
                <PanelWindow
                    panel-id="ref"
                    title="Reference"
                    :min-width="300"
                    :min-height="300"
                    :closable="true"
                >
                    <ReferencePanel />
                </PanelWindow>

                <!-- Inventory panel (hidden by default, toggled from header) -->
                <PanelWindow
                    panel-id="inv"
                    title="Inventory"
                    :min-width="200"
                    :min-height="200"
                    :closable="true"
                >
                    <InventoryPanel />
                </PanelWindow>

                <!-- Combat panel (auto-shows during combat) -->
                <PanelWindow
                    v-if="gameStore.currentScene === 'combat'"
                    panel-id="combat"
                    title="Combat"
                    :min-width="200"
                    :min-height="150"
                    :closable="true"
                >
                    <CombatPanel />
                </PanelWindow>

                <!-- Encounter window — shown during active combat AND pre-combat
                     (when an encounter is staged but the player hasn't chosen yet) -->
                <PanelWindow
                    v-if="showEncounter"
                    panel-id="encounter"
                    title="Encounter"
                    :min-width="200"
                    :min-height="200"
                    :closable="true"
                >
                    <EncounterWindow />
                </PanelWindow>

                <!-- Loot panel (HR-786) — shown while a searchable source is revealed -->
                <PanelWindow
                    v-if="lootStore.sources.length > 0"
                    panel-id="loot"
                    title="Loot"
                    :min-width="200"
                    :min-height="160"
                    :closable="true"
                >
                    <LootPanel />
                </PanelWindow>
            </WindowManager>
        </div>

        <!-- World manager overlay — shown when no world is loaded or toggled from header -->
        <WorldManager
            v-if="!worldLoaded || showWorldManager"
            @close="showWorldManager = false"
        />
    </div>
</template>
