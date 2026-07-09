<script setup lang="ts">
import { useLootStore } from "../stores/loot";
import { useWebSocket } from "../composables/useWebSocket";
import { lootTier, LOOT_COLORS, type LootTier } from "../utils/mapGraphics";
import type { LootSourceView } from "../worldModel/model";

const lootStore = useLootStore();
const { send } = useWebSocket();

interface LootItemRow {
  name: string;
  value: number;
  tier: LootTier;
  color: string;
}

function itemRows(items: Array<Record<string, unknown>>): LootItemRow[] {
  return items.map((it) => {
    const name = typeof it.name === "string" ? it.name : "item";
    const value = typeof it.value === "number" ? it.value : 0;
    const tier = lootTier(value);
    return { name, value, tier, color: LOOT_COLORS[tier] };
  });
}

function take(name: string): void {
  send(`take ${name}`);
}

function takeAll(source: LootSourceView): void {
  for (const it of source.items) {
    const name = typeof it.name === "string" ? it.name : "";
    if (name) send(`take ${name}`);
  }
}
</script>

<template>
  <div data-testid="loot-panel" class="p-2 text-xs font-mono text-gray-200">
    <div
      v-if="lootStore.sources.length === 0"
      data-testid="loot-panel-empty"
      class="text-gray-500"
    >
      Search a container or corpse to reveal its contents.
    </div>

    <div
      v-for="source in lootStore.sources"
      :key="source.id"
      :data-testid="`loot-source-${source.id}`"
      class="mb-3 border border-gray-700 rounded p-2 bg-gray-900/40"
    >
      <div class="flex items-center justify-between mb-1">
        <span class="font-semibold text-gray-100 capitalize">{{ source.name }}</span>
        <button
          v-if="source.items.length"
          :data-testid="`loot-take-all-${source.id}`"
          class="px-1.5 py-0.5 text-[10px] rounded border border-gray-600 hover:border-green-500 hover:text-green-300 transition-colors"
          @click="takeAll(source)"
        >
          Take all
        </button>
      </div>

      <div v-if="source.gold > 0" class="text-yellow-400 mb-1">
        {{ source.gold }} gold
      </div>

      <ul>
        <li
          v-for="row in itemRows(source.items)"
          :key="row.name"
          class="flex items-center justify-between py-0.5"
        >
          <span :style="{ color: row.color }" :data-loot-tier="row.tier">
            {{ row.name }} <span class="text-gray-500">({{ row.value }})</span>
          </span>
          <button
            :data-testid="`loot-take-${source.id}-${row.name}`"
            class="px-1.5 py-0.5 text-[10px] rounded border border-gray-600 hover:border-green-500 hover:text-green-300 transition-colors"
            @click="take(row.name)"
          >
            Take
          </button>
        </li>
      </ul>
    </div>
  </div>
</template>
