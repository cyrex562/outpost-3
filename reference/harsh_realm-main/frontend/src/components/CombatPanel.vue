<script setup lang="ts">
import { computed } from "vue";
import { useGameStore, type CombatEnemy } from "../stores/game";

// TODO: In a future milestone, add per-enemy armor/body-part damage display
// (e.g. limb wounds, damaged gear) once the Rust backend tracks that state.

const gameStore = useGameStore();
const enemies = computed(() => gameStore.combatEnemies);

function hpCondition(enemy: CombatEnemy): string {
  if (enemy.defeated || enemy.hp <= 0) return "Defeated";
  if (enemy.maxHp <= 0) return "Unknown";
  const ratio = enemy.hp / enemy.maxHp;
  if (ratio >= 1.0) return "Unharmed";
  if (ratio >= 0.75) return "Scratched";
  if (ratio >= 0.5) return "Bloodied";
  if (ratio >= 0.25) return "Badly Hurt";
  return "Near Death";
}

function conditionClass(enemy: CombatEnemy): string {
  const cond = hpCondition(enemy);
  switch (cond) {
    case "Unharmed":   return "text-green-400 bg-green-900/30 border-green-700";
    case "Scratched":  return "text-yellow-300 bg-yellow-900/30 border-yellow-700";
    case "Bloodied":   return "text-orange-400 bg-orange-900/30 border-orange-700";
    case "Badly Hurt": return "text-red-400 bg-red-900/30 border-red-700";
    case "Near Death": return "text-red-300 bg-red-950/50 border-red-600";
    case "Defeated":   return "text-gray-500 bg-gray-900/30 border-gray-700";
    default:           return "text-gray-400 bg-gray-900/30 border-gray-700";
  }
}

function hpBarWidth(enemy: CombatEnemy): string {
  if (enemy.maxHp <= 0 || enemy.defeated) return "0%";
  const pct = Math.max(0, Math.min(100, (enemy.hp / enemy.maxHp) * 100));
  return `${pct}%`;
}

function hpBarColor(enemy: CombatEnemy): string {
  const cond = hpCondition(enemy);
  switch (cond) {
    case "Unharmed":   return "bg-green-500";
    case "Scratched":  return "bg-yellow-400";
    case "Bloodied":   return "bg-orange-500";
    case "Badly Hurt": return "bg-red-500";
    case "Near Death": return "bg-red-700";
    default:           return "bg-gray-600";
  }
}
</script>

<template>
  <div class="flex flex-col gap-2 p-2 font-mono text-sm" data-testid="combat-panel">
    <div
      v-if="enemies.length === 0"
      class="text-gray-500 text-xs text-center py-4"
      data-testid="combat-panel-empty"
    >
      No combatants
    </div>

    <div
      v-for="enemy in enemies"
      :key="enemy.entityId"
      class="flex flex-col gap-1 p-2 rounded border"
      :class="enemy.defeated ? 'border-gray-700 opacity-50' : 'border-gray-600'"
      :data-testid="`combat-enemy-${enemy.entityId}`"
    >
      <!-- Name row -->
      <div class="flex items-center justify-between gap-2">
        <span
          class="font-semibold truncate"
          :class="enemy.defeated ? 'text-gray-500 line-through' : 'text-gray-100'"
          :data-testid="`combat-enemy-name-${enemy.entityId}`"
        >
          {{ enemy.name }}
        </span>
        <!-- Condition badge -->
        <span
          class="text-xs px-1.5 py-0.5 rounded border shrink-0"
          :class="conditionClass(enemy)"
          :data-testid="`combat-enemy-condition-${enemy.entityId}`"
        >
          {{ hpCondition(enemy) }}
        </span>
      </div>

      <!-- HP bar + exact value -->
      <div class="flex items-center gap-2">
        <div class="flex-1 h-1.5 bg-gray-800 rounded overflow-hidden">
          <div
            class="h-full rounded transition-all duration-300"
            :class="hpBarColor(enemy)"
            :style="{ width: hpBarWidth(enemy) }"
            :data-testid="`combat-enemy-hpbar-${enemy.entityId}`"
          />
        </div>
        <span
          class="text-xs text-gray-400 shrink-0 tabular-nums"
          :class="enemy.defeated ? 'line-through text-gray-600' : ''"
          :data-testid="`combat-enemy-hp-${enemy.entityId}`"
        >
          {{ enemy.defeated ? "0" : enemy.hp }}/{{ enemy.maxHp }}
        </span>
      </div>
    </div>
  </div>
</template>
