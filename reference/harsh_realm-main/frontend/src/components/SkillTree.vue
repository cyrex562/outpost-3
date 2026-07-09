<script setup lang="ts">
import { computed, ref } from "vue";
import type { SkillOption } from "../types/api";
import {
  evaluateSkillLock,
  type SkillDraftContext,
} from "../utils/skillRequirements";

const props = withDefaults(
  defineProps<{
    skills: SkillOption[];
    levels: Record<string, number>;
    context: SkillDraftContext;
    remainingPoints: number;
    maxLevel?: number;
  }>(),
  { maxLevel: 4 },
);

const emit = defineEmits<{ set: [skillId: string, level: number] }>();

const search = ref("");
const collapsed = ref<Record<string, boolean>>({});

const filtered = computed<SkillOption[]>(() => {
  const q = search.value.trim().toLowerCase();
  if (!q) return props.skills;
  return props.skills.filter(
    (s) =>
      s.name.toLowerCase().includes(q) ||
      s.id.toLowerCase().includes(q) ||
      s.description.toLowerCase().includes(q) ||
      s.category.toLowerCase().includes(q) ||
      (s.subcategory ?? "").toLowerCase().includes(q),
  );
});

const grouped = computed<{ category: string; skills: SkillOption[] }[]>(() => {
  const byCat = new Map<string, SkillOption[]>();
  for (const s of filtered.value) {
    const list = byCat.get(s.category) ?? [];
    list.push(s);
    byCat.set(s.category, list);
  }
  return [...byCat.entries()]
    .sort((a, b) => a[0].localeCompare(b[0]))
    .map(([category, skills]) => ({
      category,
      skills: [...skills].sort((a, b) => a.name.localeCompare(b.name)),
    }));
});

// While searching, force every matching branch open.
const searching = computed(() => search.value.trim().length > 0);

function isOpen(category: string): boolean {
  return searching.value || !collapsed.value[category];
}

function toggle(category: string): void {
  collapsed.value = {
    ...collapsed.value,
    [category]: !collapsed.value[category],
  };
}

function levelOf(id: string): number {
  return props.levels[id] ?? -1;
}

function lockOf(skill: SkillOption) {
  return evaluateSkillLock(skill, props.context);
}

function canRaise(skill: SkillOption): boolean {
  return (
    lockOf(skill).takeable &&
    levelOf(skill.id) < props.maxLevel &&
    props.remainingPoints >= 1
  );
}

function raise(skill: SkillOption): void {
  if (canRaise(skill)) emit("set", skill.id, levelOf(skill.id) + 1);
}

function lower(skill: SkillOption): void {
  if (levelOf(skill.id) >= 0) emit("set", skill.id, levelOf(skill.id) - 1);
}

function levelLabel(id: string): string {
  const lvl = levelOf(id);
  return lvl < 0 ? "—" : String(lvl);
}
</script>

<template>
  <div class="flex flex-col gap-2" data-testid="cc-skilltree">
    <input
      v-model="search"
      type="text"
      placeholder="Search skills…"
      class="w-full bg-gray-950 border border-gray-700 rounded px-2 py-1 text-xs focus:border-amber-600 focus:outline-none"
      data-testid="cc-skilltree-search"
    />

    <div
      class="border border-gray-800 rounded bg-gray-900 max-h-72 overflow-y-auto"
    >
      <p
        v-if="grouped.length === 0"
        class="text-[11px] text-gray-500 italic p-3"
        data-testid="cc-skilltree-empty"
      >
        No skills match “{{ search }}”.
      </p>

      <div
        v-for="group in grouped"
        :key="group.category"
        class="border-b border-gray-800 last:border-b-0"
      >
        <button
          type="button"
          class="w-full flex items-center justify-between px-3 py-1.5 text-xs uppercase tracking-wide text-amber-200/80 hover:bg-gray-800/50"
          :data-testid="`cc-skill-category-${group.category}`"
          @click="toggle(group.category)"
        >
          <span>{{ group.category }}</span>
          <span class="text-gray-500"
            >{{ group.skills.length }} {{ isOpen(group.category) ? "▾" : "▸" }}</span
          >
        </button>

        <div v-show="isOpen(group.category)" class="pb-1">
          <div
            v-for="skill in group.skills"
            :key="skill.id"
            class="flex items-center gap-2 px-3 py-1 text-xs"
            :data-testid="`cc-skill-row-${skill.id}`"
          >
            <div class="flex-1 min-w-0">
              <div
                class="truncate"
                :class="lockOf(skill).takeable ? 'text-gray-200' : 'text-gray-600'"
                :title="skill.description"
              >
                {{ skill.name }}
                <span class="text-gray-600">· {{ skill.default_attribute.toUpperCase() }}</span>
              </div>
              <div
                v-if="!lockOf(skill).takeable"
                class="text-[10px] text-red-400/80 truncate"
                :data-testid="`cc-skill-lock-${skill.id}`"
              >
                {{ lockOf(skill).reasons.join(" ") }}
              </div>
            </div>

            <template v-if="lockOf(skill).takeable">
              <button
                type="button"
                class="w-5 h-5 rounded border border-gray-600 text-gray-300 hover:border-amber-600 disabled:opacity-30 disabled:cursor-not-allowed"
                :disabled="levelOf(skill.id) < 0"
                :data-testid="`cc-skill-dec-${skill.id}`"
                @click="lower(skill)"
              >
                −
              </button>
              <span
                class="font-mono w-5 text-center"
                :class="levelOf(skill.id) >= 0 ? 'text-amber-100' : 'text-gray-600'"
                :data-testid="`cc-skill-level-${skill.id}`"
                >{{ levelLabel(skill.id) }}</span
              >
              <button
                type="button"
                class="w-5 h-5 rounded border border-gray-600 text-gray-300 hover:border-amber-600 disabled:opacity-30 disabled:cursor-not-allowed"
                :disabled="!canRaise(skill)"
                :data-testid="`cc-skill-inc-${skill.id}`"
                @click="raise(skill)"
              >
                +
              </button>
            </template>
            <span v-else class="text-[10px] text-gray-600 italic">locked</span>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>
