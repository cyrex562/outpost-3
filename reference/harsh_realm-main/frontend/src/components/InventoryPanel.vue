<script setup lang="ts">
import { computed } from "vue";
import { useGameStore, type EquipmentItem } from "../stores/game";

const gameStore = useGameStore();
const char = computed(() => gameStore.character);

const equippedTypes = new Set([
  "melee_weapon",
  "ranged_weapon",
  "weapon",
  "armor",
  "shield",
]);

const equipped = computed<EquipmentItem[]>(() => {
  if (!char.value) return [];
  return char.value.equipment.filter((i) => equippedTypes.has(i.type));
});

const stowed = computed<EquipmentItem[]>(() => {
  if (!char.value) return [];
  return char.value.equipment.filter((i) => !equippedTypes.has(i.type));
});

const totalEnc = computed(() => {
  if (!char.value) return 0;
  return char.value.equipment.reduce((sum, i) => sum + (i.enc ?? 0), 0);
});

const encCapacity = computed(() => {
  if (!char.value) return 10;
  return char.value.str;
});

function itemIcon(item: EquipmentItem): string {
  switch (item.type) {
    case "melee_weapon":
    case "ranged_weapon":
    case "weapon":
      return "\u2694"; // crossed swords
    case "armor":
      return "\u{1F6E1}"; // shield emoji
    case "shield":
      return "\u{1F6E1}";
    case "ammo":
      return "\u25C6"; // diamond
    case "consumable":
      return "\u25CB"; // circle
    default:
      return "\u25AA"; // small square
  }
}

function itemLabel(item: EquipmentItem): string {
  const qty = item.quantity && item.quantity > 1 ? ` \u00D7 ${item.quantity}` : "";
  return `${item.name}${qty}`;
}

function itemSublabel(item: EquipmentItem): string {
  const parts: string[] = [];
  if (item.weapon_damage || item.damage) parts.push(item.weapon_damage || item.damage || "");
  if (item.shock_damage) parts.push(`Shock ${item.shock_damage}/AC ${item.shock_ac_threshold}`);
  if (item.ac_bonus) parts.push(`+${item.ac_bonus} AC`);
  return parts.join(", ");
}

function encLabel(item: EquipmentItem): string {
  const enc = item.enc ?? 0;
  if (enc === 0) return "";
  return `${enc} enc`;
}
</script>

<template>
  <div
    class="flex flex-col h-full overflow-y-auto p-3 font-mono text-xs text-gray-300 space-y-3"
  >
    <div v-if="!char" class="text-gray-600 italic">No character loaded.</div>

    <template v-else>
      <!-- Equipped -->
      <div>
        <div class="text-gray-600 uppercase tracking-wider text-[10px] mb-1">
          Equipped
        </div>
        <div v-if="equipped.length === 0" class="text-gray-700 italic">
          (nothing equipped)
        </div>
        <div
          v-for="(item, idx) in equipped"
          :key="idx"
          class="flex flex-col py-1 border-b border-gray-800/50"
        >
          <div class="flex items-center justify-between">
            <span class="text-gray-300 truncate">
              <span class="text-gray-500 mr-1">{{ itemIcon(item) }}</span>
              {{ itemLabel(item) }}
            </span>
            <span v-if="encLabel(item)" class="text-gray-600 text-[10px] ml-2 shrink-0">
              {{ encLabel(item) }}
            </span>
          </div>
          <div v-if="itemSublabel(item)" class="text-[10px] text-gray-500 ml-5 italic">
            {{ itemSublabel(item) }}
          </div>
        </div>
      </div>

      <!-- Stowed -->
      <div>
        <div class="text-gray-600 uppercase tracking-wider text-[10px] mb-1">
          Stowed
        </div>
        <div v-if="stowed.length === 0" class="text-gray-700 italic">
          (nothing stowed)
        </div>
        <div
          v-for="(item, idx) in stowed"
          :key="idx"
          class="flex flex-col py-1 border-b border-gray-800/50"
        >
          <div class="flex items-center justify-between">
            <span class="text-gray-300 truncate">
              <span class="text-gray-500 mr-1">{{ itemIcon(item) }}</span>
              {{ itemLabel(item) }}
            </span>
            <span v-if="encLabel(item)" class="text-gray-600 text-[10px] ml-2 shrink-0">
              {{ encLabel(item) }}
            </span>
          </div>
          <div v-if="itemSublabel(item)" class="text-[10px] text-gray-500 ml-5 italic">
            {{ itemSublabel(item) }}
          </div>
        </div>
      </div>

      <!-- Enc total -->
      <div class="border-t border-gray-700 pt-2">
        <div class="flex justify-between text-[10px]">
          <span class="text-gray-500">Encumbrance</span>
          <span
            class="tabular-nums"
            :class="totalEnc > encCapacity ? 'text-red-400' : 'text-gray-300'"
          >
            {{ totalEnc }} / {{ encCapacity }}
          </span>
        </div>
      </div>
    </template>
  </div>
</template>
