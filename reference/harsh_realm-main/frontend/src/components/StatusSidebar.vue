<script setup lang="ts">
import { computed } from "vue";
import { useGameStore } from "../stores/game";

const gameStore = useGameStore();
const char = computed(() => gameStore.character);
const currentScene = computed(() => gameStore.currentScene);
const chaosFactor = computed(() => gameStore.chaosFactor);
const questActive = computed(() => gameStore.questActive);
const statusEffects = computed(() => char.value?.statusEffects ?? []);

// HP color: >50% green, 25-50% yellow, <25% red
function hpColor(hp: number, max: number): string {
    const pct = max > 0 ? hp / max : 0;
    if (pct > 0.5) return "#4ade80"; // green
    if (pct > 0.25) return "#facc15"; // yellow
    return "#f87171"; // red
}

function hpBarWidth(hp: number, max: number): string {
    const pct = max > 0 ? Math.max(0, Math.min(1, hp / max)) : 0;
    return `${Math.round(pct * 100)}%`;
}

const sceneLabel: Record<string, string> = {
    exploration: "Exploring",
    social: "Social",
    shopping: "Shopping",
    combat: "Combat",
    char_create: "Character Creation",
    respawn: "Respawn",
};

const sceneColor: Record<string, string> = {
    exploration: "bg-emerald-900/50 border-emerald-700 text-emerald-300",
    social: "bg-sky-900/50 border-sky-700 text-sky-300",
    shopping: "bg-amber-900/50 border-amber-700 text-amber-300",
    combat: "bg-red-900/50 border-red-700 text-red-300",
    char_create: "bg-purple-900/50 border-purple-700 text-purple-300",
    respawn: "bg-gray-800/50 border-gray-600 text-gray-300",
};

function chaosColor(chaos: number): string {
    if (chaos <= 3) return "text-emerald-400";
    if (chaos <= 6) return "text-amber-400";
    return "text-red-400";
}

function effectLabel(effectId: string): string {
    const parts = effectId.split(".");
    return parts[parts.length - 1] ?? effectId;
}

function expiryLabel(expiresAtTick: number | null): string {
    return expiresAtTick === null ? "permanent" : `until tick ${expiresAtTick}`;
}
</script>

<template>
    <div
        class="flex flex-col h-full overflow-y-auto p-3 font-mono text-xs text-gray-300 space-y-3"
    >
        <!-- No character state -->
        <div v-if="!char" class="text-gray-600 italic">
            No character loaded.
        </div>

        <!-- Character loaded -->
        <template v-else>
            <!-- Name + class + scene badge -->
            <div class="border-b border-gray-700 pb-2">
                <div class="text-gray-100 text-sm font-semibold">
                    {{ char.name }}
                </div>
                <div class="text-gray-500 capitalize mb-1">
                    {{ char.characterClass }} {{ char.level }}
                </div>
                <div class="flex flex-col gap-1 items-start">
                    <span
                        class="px-1.5 py-0.5 rounded text-[10px] font-bold border uppercase"
                        :class="
                            sceneColor[currentScene] ??
                            'bg-gray-800 border-gray-600 text-gray-400'
                        "
                    >
                        {{ sceneLabel[currentScene] ?? currentScene }}
                    </span>
                    <span
                        v-if="currentScene === 'dungeon' && char.features?.[0]"
                        class="text-[10px] text-gray-400 font-mono truncate max-w-full"
                    >
                        &raquo; {{ char.features[0] }}
                    </span>
                </div>
            </div>

            <!-- Vitals -->
            <div>
                <div
                    class="text-gray-600 uppercase tracking-wider text-[10px] mb-1"
                >
                    Vitals
                </div>

                <!-- HP bar -->
                <div class="flex items-center gap-2 mb-1">
                    <span class="text-gray-500 w-4">HP</span>
                    <div
                        class="flex-1 bg-gray-800 rounded-full h-1.5 overflow-hidden"
                    >
                        <div
                            class="h-full rounded-full transition-all"
                            :style="{
                                width: hpBarWidth(char.hp, char.maxHp),
                                backgroundColor: hpColor(char.hp, char.maxHp),
                            }"
                        />
                    </div>
                    <span class="text-gray-300 text-[10px] tabular-nums"
                        >{{ char.hp }}/{{ char.maxHp }}</span
                    >
                </div>

                <div class="flex gap-4 text-[10px]">
                    <span
                        ><span class="text-gray-500">AC</span>
                        {{ char.ac }}</span
                    >
                    <span
                        ><span class="text-gray-500">XP</span> {{ char.xp
                        }}<span class="text-gray-600"
                            >/{{ char.xpNext }}</span
                        ></span
                    >
                </div>

                <!-- Gold -->
                <div class="flex items-center gap-1 text-[10px] mt-1.5">
                    <span class="text-amber-500 text-xs">&#x29BF;</span>
                    <span class="text-amber-300 tabular-nums font-semibold"
                        >{{ char.gold }} gp</span
                    >
                </div>
            </div>

            <!-- Oracle -->
            <div>
                <div
                    class="text-gray-600 uppercase tracking-wider text-[10px] mb-1"
                >
                    Oracle
                </div>
                <div class="text-[10px] flex items-center gap-1.5">
                    <span class="text-gray-500">Chaos:</span>
                    <span
                        class="tabular-nums font-bold text-sm"
                        :class="chaosColor(chaosFactor)"
                    >
                        {{ chaosFactor }}
                    </span>
                </div>
                <!-- Active quests (HR-19) -->
                <div
                    v-if="questActive > 0"
                    data-testid="quest-badge"
                    class="text-[10px] flex items-center gap-1.5 mt-1"
                >
                    <span class="text-gray-500">Quests:</span>
                    <span class="tabular-nums font-bold text-sm text-amber-300">
                        {{ questActive }}
                    </span>
                </div>
            </div>

            <!-- Location -->
            <div v-if="char.locationQ !== null">
                <div
                    class="text-gray-600 uppercase tracking-wider text-[10px] mb-1"
                >
                    Location
                </div>
                <div class="text-gray-400">
                    ({{ char.locationQ }}, {{ char.locationR }})
                    <span
                        v-if="char.terrain"
                        class="text-gray-500 ml-1 capitalize"
                        >· {{ char.terrain }}</span
                    >
                </div>
                <div
                    v-if="char.weather"
                    class="text-emerald-500/80 text-[10px] mt-0.5"
                >
                    Weather: {{ char.weather.condition }} ({{
                        char.weather.temperature
                    }})
                </div>
                <div
                    v-if="char.features.length"
                    class="text-gray-600 text-[10px] mt-0.5"
                >
                    {{ char.features.join(", ") }}
                </div>
            </div>

            <!-- Conditions -->
            <div>
                <div
                    class="text-gray-600 uppercase tracking-wider text-[10px] mb-1"
                >
                    Conditions
                </div>
                <div v-if="statusEffects.length" class="space-y-1">
                    <span
                        v-for="effect in statusEffects"
                        :key="effect.id"
                        class="flex items-center justify-between gap-2 px-1.5 py-1 bg-red-950/40 border border-red-900 text-red-200 text-[10px]"
                        :title="effect.description ?? ''"
                    >
                        <span class="truncate">
                            <span v-if="effect.icon" class="text-red-300 mr-1">
                                {{ effect.icon }}
                            </span>
                            {{ effect.name ?? effectLabel(effect.effect_id) }}
                        </span>
                        <span
                            class="text-red-400 tabular-nums whitespace-nowrap"
                        >
                            {{ expiryLabel(effect.expires_at_tick) }}
                        </span>
                    </span>
                </div>
                <div v-else class="text-gray-700 italic">(none)</div>
            </div>
        </template>
    </div>
</template>
