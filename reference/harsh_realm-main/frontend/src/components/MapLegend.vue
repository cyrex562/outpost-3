<script setup lang="ts">
import {
    BIOME_COLORS,
    DESERT_COLORS,
    ENCOUNTER_COLORS,
    LOOT_COLORS,
    MOUNTAIN_COLORS,
    OBJECTIVE_COLORS,
    ROAD_COLORS,
    RUIN_COLORS,
    SETTLEMENT_COLORS,
    WASTELAND_COLORS,
    WATER_COLORS,
} from "../utils/mapGraphics";

defineProps<{
    show: boolean;
}>();

const emit = defineEmits<{
    close: [];
}>();

type LegendIcon =
    | "oak"
    | "pine"
    | "palm"
    | "settlement"
    | "river"
    | "lake"
    | "mountain"
    | "ruins"
    | "desert"
    | "wasteland"
    | "road"
    | "objective"
    | "skull"
    | "loot";

interface LegendItem {
    id: string;
    label: string;
    color?: string;
    symbol?: string;
    isCircle?: boolean;
    icon?: LegendIcon;
}

const COL = BIOME_COLORS;
const SET = SETTLEMENT_COLORS;
const WAT = WATER_COLORS;
const MTN = MOUNTAIN_COLORS;
const RUIN = RUIN_COLORS;
const DES = DESERT_COLORS;
const WAS = WASTELAND_COLORS;
const ROAD = ROAD_COLORS;
const OBJ = OBJECTIVE_COLORS;
const ENC = ENCOUNTER_COLORS;
const LOOT = LOOT_COLORS;

interface LegendGroup {
    name: string;
    items: LegendItem[];
}

const TERRAIN_GROUPS: LegendGroup[] = [
    {
        name: "World Terrain",

        items: [
            { id: "plains", label: "Plains", color: "#4a5d3a" },
            { id: "forest", label: "Forest", color: "#2d4a2d" },
            { id: "hills", label: "Hills", color: "#6b6b4f" },
            { id: "mountains", label: "Mountains", color: "#5a5a5a" },
            { id: "water", label: "Water", color: "#2a3d5a" },
            { id: "swamp", label: "Swamp", color: "#3d4a3a" },
            { id: "desert", label: "Desert", color: "#7a6a4a" },
            { id: "wasteland", label: "Wasteland", color: "#5a4a3a" },
            { id: "ruins", label: "Ruins", color: "#4a4a5a" },
        ],
    },
    {
        name: "Dungeon Terrain",
        items: [
            { id: "corridor", label: "Corridor", color: "#5a5040" },
            { id: "room_floor", label: "Room Floor", color: "#6a6050" },
            { id: "wall", label: "Wall", color: "#2a2a2a" },
            { id: "door", label: "Door", color: "#8a7040" },
            { id: "stairs_up", label: "Stairs Up", color: "#50706a" },
            { id: "stairs_down", label: "Stairs Down", color: "#705060" },
            { id: "trap_floor", label: "Trap Floor", color: "#6a3030" },
        ],
    },
    {
        name: "Town Terrain",
        items: [
            { id: "road", label: "Road", color: "#6a6a5a" },
            { id: "building", label: "Building", color: "#4a4040" },
            { id: "shop", label: "Shop", color: "#5a6a40" },
            { id: "tavern", label: "Tavern", color: "#6a5a30" },
            { id: "temple", label: "Temple", color: "#4a4a6a" },
            { id: "plaza", label: "Plaza", color: "#6a6a6a" },
        ],
    },
    {
        name: "Forests",
        items: [
            { id: "oak", label: "Deciduous (oak)", icon: "oak" },
            { id: "pine", label: "Boreal (pine)", icon: "pine" },
            { id: "palm", label: "Tropical (palm)", icon: "palm" },
        ],
    },
    {
        name: "Water Features",
        items: [
            { id: "river", label: "River", icon: "river" },
            { id: "lake", label: "Lake", icon: "lake" },
        ],
    },
    {
        name: "Terrain Art",
        items: [
            { id: "mountain_art", label: "Mountains", icon: "mountain" },
            { id: "ruins_art", label: "Ruins", icon: "ruins" },
            { id: "desert_art", label: "Desert", icon: "desert" },
            { id: "wasteland_art", label: "Wasteland", icon: "wasteland" },
            { id: "road_art", label: "Road", icon: "road" },
        ],
    },
    {
        name: "Features & Markers",
        items: [
            { id: "settlement", label: "Settlement", icon: "settlement" },
            { id: "objective", label: "Objective", icon: "objective" },
            { id: "encounter", label: "Monster Lair", icon: "skull" },
            { id: "loot", label: "Loot (by rarity)", icon: "loot" },
            { id: "landmark", label: "Landmark", symbol: "★", color: "#ff0" },
        ],
    },
    {
        name: "System",
        items: [
            { id: "player", label: "You", color: "#f0c040", isCircle: true },
            { id: "fog", label: "Unexplored", color: "#1e1e2a" },
        ],
    },
];
</script>

<template>
    <Transition name="fade">
        <div
            v-if="show"
            class="absolute inset-0 bg-black/60 flex items-center justify-center z-50 p-4"
            @click.self="emit('close')"
        >
            <div
                class="bg-gray-900 border border-gray-700 rounded-lg shadow-2xl max-w-sm w-full overflow-hidden flex flex-col max-h-full"
            >
                <div
                    class="px-4 py-3 border-b border-gray-800 flex justify-between items-center bg-gray-900"
                >
                    <h3
                        class="text-sm font-bold uppercase tracking-wider text-gray-300"
                    >
                        Map Legend
                    </h3>
                    <button
                        class="text-gray-500 hover:text-white transition-colors p-1"
                        @click="emit('close')"
                    >
                        <svg
                            xmlns="http://www.w3.org/2000/svg"
                            class="h-5 w-5"
                            viewBox="0 0 20 20"
                            fill="currentColor"
                        >
                            <path
                                fill-rule="evenodd"
                                d="M4.293 4.293a1 1 0 011.414 0L10 8.586l4.293-4.293a1 1 0 111.414 1.414L11.414 10l4.293 4.293a1 1 0 01-1.414 1.414L10 11.414l-4.293 4.293a1 1 0 01-1.414-1.414L8.586 10 4.293 5.707a1 1 0 010-1.414z"
                                clip-rule="evenodd"
                            />
                        </svg>
                    </button>
                </div>

                <div class="p-5 overflow-y-auto space-y-6">
                    <div v-for="group in TERRAIN_GROUPS" :key="group.name">
                        <h4
                            class="text-[10px] font-bold text-gray-500 uppercase tracking-widest mb-3 border-b border-gray-800 pb-1"
                        >
                            {{ group.name }}
                        </h4>
                        <div class="grid grid-cols-2 gap-y-3 gap-x-4">
                            <div
                                v-for="item in group.items"
                                :key="item.id"
                                class="flex items-center gap-3 text-xs"
                            >
                                <!-- Square color -->
                                <div
                                    v-if="
                                        item.color &&
                                        !item.symbol &&
                                        !item.isCircle
                                    "
                                    class="w-4 h-4 rounded border border-black/40 shrink-0"
                                    :style="{ backgroundColor: item.color }"
                                ></div>
                                <!-- Circle color -->
                                <div
                                    v-if="item.color && item.isCircle"
                                    class="w-4 h-4 rounded-full border border-black/60 shrink-0 shadow-sm"
                                    :style="{ backgroundColor: item.color }"
                                ></div>
                                <!-- Symbol -->
                                <div
                                    v-if="item.symbol"
                                    class="w-4 h-4 flex items-center justify-center font-mono font-bold shrink-0"
                                    :style="{ color: item.color }"
                                >
                                    {{ item.symbol }}
                                </div>
                                <!-- Map art preview (trees / settlement) -->
                                <svg
                                    v-if="item.icon"
                                    class="w-4 h-4 shrink-0"
                                    viewBox="-8 -10 16 16"
                                >
                                    <template v-if="item.icon === 'oak'">
                                        <rect x="-0.8" y="0" width="1.6" height="4" :fill="COL.temperate.trunk" />
                                        <circle cx="0" cy="-2.5" r="4" :fill="COL.temperate.canopyDark" />
                                        <circle cx="-1.4" cy="-3.6" r="2.6" :fill="COL.temperate.canopy" />
                                    </template>
                                    <template v-else-if="item.icon === 'pine'">
                                        <rect x="-0.8" y="0" width="1.6" height="4" :fill="COL.boreal.trunk" />
                                        <polygon points="0,-1 -4,5 4,5" :fill="COL.boreal.canopyDark" />
                                        <polygon points="0,-5 -3,1 3,1" :fill="COL.boreal.canopy" />
                                    </template>
                                    <template v-else-if="item.icon === 'palm'">
                                        <rect x="-0.7" y="-1" width="1.4" height="5" :fill="COL.tropical.trunk" />
                                        <g :stroke="COL.tropical.canopy" stroke-width="1.4" stroke-linecap="round" fill="none">
                                            <path d="M0,-3 Q-5,-6 -7,-2" />
                                            <path d="M0,-3 Q5,-6 7,-2" />
                                            <path d="M0,-3 Q-4,-8 -5,-6" />
                                            <path d="M0,-3 Q4,-8 5,-6" />
                                            <path d="M0,-3 Q0,-8 0,-9" />
                                        </g>
                                    </template>
                                    <template v-else-if="item.icon === 'river'">
                                        <path
                                            d="M-7,-7 Q-2,-3 -1,1 Q0,5 6,7"
                                            :stroke="WAT.channel"
                                            stroke-width="2.5"
                                            stroke-linecap="round"
                                            fill="none"
                                        />
                                    </template>
                                    <template v-else-if="item.icon === 'lake'">
                                        <rect x="-7" y="-6" width="14" height="12" rx="3" fill="#2a3d5a" />
                                        <rect
                                            x="-7"
                                            y="-6"
                                            width="14"
                                            height="12"
                                            rx="3"
                                            :stroke="WAT.shore"
                                            stroke-width="1.5"
                                            fill="none"
                                        />
                                        <path d="M-4,0 q2,-1.5 4,0" :stroke="WAT.ripple" stroke-width="1" fill="none" />
                                    </template>
                                    <template v-else-if="item.icon === 'mountain'">
                                        <polygon points="-6,5 -1,-5 4,5" :fill="MTN.rock" />
                                        <polygon points="-1,-5 4,5 -1,5" :fill="MTN.rockDark" />
                                        <polygon points="-1,-5 -2.4,-1.5 0.4,-1.5" :fill="MTN.snow" />
                                        <polygon points="3,0 6,5 0,5" :fill="MTN.rock" />
                                    </template>
                                    <template v-else-if="item.icon === 'ruins'">
                                        <rect x="-6" y="-2" width="2.2" height="7" :fill="RUIN.stone" />
                                        <rect x="-1.6" y="-5" width="2.4" height="10" :fill="RUIN.stone" />
                                        <rect x="3" y="0.5" width="2.2" height="4.5" :fill="RUIN.stoneDark" />
                                        <rect x="-6" y="4.4" width="11" height="1.4" :fill="RUIN.stoneDark" />
                                    </template>
                                    <template v-else-if="item.icon === 'desert'">
                                        <path d="M-7,1 Q0,-3 7,1 L7,6 L-7,6 Z" :fill="DES.dune" opacity="0.85" />
                                        <path d="M-7,4 Q0,1 7,4 L7,6 L-7,6 Z" :fill="DES.duneDark" opacity="0.8" />
                                        <ellipse cx="3" cy="3" rx="1.4" ry="0.9" :fill="DES.rock" />
                                    </template>
                                    <template v-else-if="item.icon === 'wasteland'">
                                        <g :stroke="WAS.crack" stroke-width="0.9" stroke-linecap="round" fill="none">
                                            <path d="M-6,-3 L-1,1" />
                                            <path d="M-1,1 L4,-2" />
                                            <path d="M-1,1 L0,5" />
                                            <path d="M3,3 L6,4" />
                                        </g>
                                        <g :stroke="WAS.shrub" stroke-width="0.8" stroke-linecap="round" fill="none">
                                            <path d="M4,5 L4,1.5" />
                                            <path d="M4,3 L2.6,1.5" />
                                            <path d="M4,3 L5.4,1.5" />
                                        </g>
                                    </template>
                                    <template v-else-if="item.icon === 'road'">
                                        <line x1="-7" y1="4" x2="7" y2="-4" :stroke="ROAD.path" stroke-width="3.2" stroke-linecap="round" />
                                        <line
                                            x1="-7"
                                            y1="4"
                                            x2="7"
                                            y2="-4"
                                            :stroke="ROAD.dash"
                                            stroke-width="0.7"
                                            stroke-dasharray="2 2"
                                            stroke-linecap="round"
                                        />
                                    </template>
                                    <template v-else-if="item.icon === 'objective'">
                                        <circle cx="0" cy="0" r="6" :fill="OBJ.glow" opacity="0.2" />
                                        <line x1="-2" y1="-5" x2="-2" y2="5" :stroke="OBJ.pole" stroke-width="1.2" stroke-linecap="round" />
                                        <polygon points="-2,-5 5,-2.5 -2,0" :fill="OBJ.flag" />
                                    </template>
                                    <template v-else-if="item.icon === 'skull'">
                                        <circle cx="0" cy="-1" r="4.6" :fill="ENC.bone" />
                                        <rect x="-2.6" y="2.6" width="5.2" height="3.4" rx="1" :fill="ENC.bone" />
                                        <circle cx="-1.8" cy="-1.3" r="1.5" :fill="ENC.eye" />
                                        <circle cx="1.8" cy="-1.3" r="1.5" :fill="ENC.eye" />
                                        <polygon points="0,0.4 -1,2.6 1,2.6" :fill="ENC.boneShadow" />
                                    </template>
                                    <template v-else-if="item.icon === 'loot'">
                                        <circle cx="0" cy="0" r="5.5" :fill="LOOT.legendary" opacity="0.25" />
                                        <polygon points="0,-4.6 4.6,0 0,4.6 -4.6,0" :fill="LOOT.epic" stroke="#1a1a1a" stroke-width="0.4" />
                                    </template>
                                    <template v-else>
                                        <rect x="-4" y="-1" width="8" height="5" :fill="SET.wall" />
                                        <polygon points="-5,-1 0,-5 5,-1" :fill="SET.roof" />
                                    </template>
                                </svg>
                                <span class="text-gray-300 font-medium">{{
                                    item.label
                                }}</span>
                            </div>
                        </div>
                    </div>
                </div>

                <div
                    class="p-3 bg-gray-900 border-t border-gray-800 text-center"
                >
                    <button
                        class="w-full py-2 bg-gray-800 hover:bg-gray-700 text-gray-300 text-[10px] font-bold uppercase tracking-widest rounded transition-colors"
                        @click="emit('close')"
                    >
                        Close
                    </button>
                </div>
            </div>
        </div>
    </Transition>
</template>

<style scoped>
.fade-enter-active,
.fade-leave-active {
    transition: opacity 0.2s ease;
}

.fade-enter-from,
.fade-leave-to {
    opacity: 0;
}
</style>
