<script setup lang="ts">
import { onMounted } from "vue";
import { useGameStore } from "../stores/game";
import { useCharacterCreationStore } from "../stores/characterCreation";
import SkillTree from "./SkillTree.vue";
import AttrPoolAssign from "./AttrPoolAssign.vue";
import CharacterCreationSidebar from "./CharacterCreationSidebar.vue";

const store = useCharacterCreationStore();
const gameStore = useGameStore();

onMounted(() => {
  void store.loadOptions();
});

async function create(): Promise<void> {
  const ok = await store.submit();
  if (ok) {
    // Eagerly refresh character so the form unmounts on success.
    await gameStore.loadCharacter();
  }
}

function switchToChat(): void {
  gameStore.setCreationMode("chat");
}
</script>

<template>
  <Teleport to="body">
    <div
      class="fixed inset-x-0 bottom-0 top-12 z-[55] flex flex-col bg-gray-950 text-gray-200"
      data-testid="character-creation-form"
    >
      <!-- ── Header ─────────────────────────────────────── -->
      <div
        class="flex shrink-0 items-center justify-between border-b border-gray-800 px-5 py-3"
      >
        <h1 class="text-lg font-bold uppercase tracking-wide text-amber-200">
          Create Your Character
        </h1>
        <button
          class="rounded border border-gray-600 px-3 py-1 text-xs text-gray-400 hover:border-amber-700 hover:text-amber-200"
          data-testid="cc-use-chat"
          @click="switchToChat"
        >
          Do it in chat instead
        </button>
      </div>

      <!-- ── Loading / error state ─────────────────────── -->
      <div
        v-if="store.loading"
        class="flex flex-1 items-center justify-center"
      >
        <p class="text-sm text-gray-500">Loading options…</p>
      </div>
      <div
        v-else-if="!store.options && store.error"
        class="flex flex-1 items-center justify-center"
      >
        <p class="text-sm text-red-400">{{ store.error }}</p>
      </div>

      <!-- ── Two-column content ─────────────────────────── -->
      <template v-else-if="store.options">
        <div class="grid min-h-0 flex-1 lg:grid-cols-[1fr_300px]">

          <!-- LEFT: scrollable form -->
          <div class="overflow-y-auto p-5">

            <!-- Name -->
            <section class="mb-6">
              <label class="mb-1 block text-xs uppercase text-gray-500">Name</label>
              <input
                v-model="store.name"
                type="text"
                maxlength="64"
                placeholder="e.g. Kira Vance"
                class="w-full rounded border border-gray-700 bg-gray-900 px-3 py-2 text-sm focus:border-amber-600 focus:outline-none"
                data-testid="cc-name"
              />
            </section>

            <!-- Class -->
            <section class="mb-6">
              <h2 class="mb-2 text-xs uppercase text-gray-500">Class</h2>
              <p
                v-if="store.options.classes.length === 0"
                class="text-xs italic text-gray-500"
              >
                No classes available — check your content packs.
              </p>
              <div v-else class="grid gap-2 sm:grid-cols-3">
                <button
                  v-for="cls in store.options.classes"
                  :key="cls.id"
                  type="button"
                  class="rounded border p-3 text-left transition-colors"
                  :class="
                    store.characterClass === cls.id
                      ? 'border-amber-600 bg-amber-900/20'
                      : 'border-gray-700 bg-gray-900 hover:border-gray-500'
                  "
                  :data-testid="`cc-class-${cls.id}`"
                  @click="store.setClass(cls.id)"
                >
                  <div class="text-sm font-semibold text-amber-100">{{ cls.name }}</div>
                  <div class="mt-1 text-[11px] leading-snug text-gray-400">
                    {{ cls.description }}
                  </div>
                  <div class="mt-1 text-[11px] text-gray-500">
                    HD d{{ cls.hit_die }} · {{ cls.skill_points_start }} skill pts
                  </div>
                </button>
              </div>
            </section>

            <!-- Attributes (method switcher + pool or point-buy) -->
            <section class="mb-6">
              <AttrPoolAssign />
            </section>

            <!-- Skills -->
            <section v-if="store.characterClass" class="mb-6">
              <div class="mb-2 flex items-center justify-between gap-2">
                <h2 class="text-xs uppercase text-gray-500">Skills</h2>
                <div class="flex items-center gap-3">
                  <button
                    type="button"
                    class="rounded border border-gray-600 px-2 py-0.5 text-[11px] text-gray-400 hover:border-amber-700 hover:text-amber-200"
                    data-testid="cc-auto-pick"
                    @click="store.autoPickSkills()"
                  >
                    Auto-pick for {{ store.selectedClass?.name ?? "class" }}
                  </button>
                  <span class="text-[11px] text-gray-400">
                    Points remaining:
                    <span
                      class="font-mono"
                      :class="store.skillRemaining < 0 ? 'text-red-400' : 'text-amber-200'"
                      data-testid="cc-skill-remaining"
                    >{{ store.skillRemaining }}</span>
                    / {{ store.skillBudget }}
                  </span>
                </div>
              </div>
              <SkillTree
                :skills="store.options.skills"
                :levels="store.skills"
                :context="store.skillContext"
                :remaining-points="store.skillRemaining"
                @set="store.setSkill"
              />
            </section>

            <!-- Starting kit -->
            <section v-if="store.availableKits.length" class="mb-6">
              <h2 class="mb-2 text-xs uppercase text-gray-500">Starting kit</h2>
              <div class="grid gap-2 sm:grid-cols-2">
                <button
                  v-for="kit in store.availableKits"
                  :key="kit.id"
                  type="button"
                  class="rounded border p-3 text-left transition-colors"
                  :class="
                    store.kitId === kit.id
                      ? 'border-amber-600 bg-amber-900/20'
                      : 'border-gray-700 bg-gray-900 hover:border-gray-500'
                  "
                  :data-testid="`cc-kit-${kit.id}`"
                  @click="store.setKit(kit.id)"
                >
                  <div class="text-sm font-semibold text-amber-100">{{ kit.name }}</div>
                  <div class="mt-1 text-[11px] text-gray-400">{{ kit.description }}</div>
                  <div class="mt-1 text-[11px] text-gray-500">
                    {{ kit.items.map((i) => i.name).join(", ") }}
                  </div>
                </button>
              </div>
            </section>

          </div>
          <!-- /LEFT -->

          <!-- RIGHT: live summary sidebar (large screens only) -->
          <div class="hidden overflow-y-auto border-l border-gray-800 lg:block">
            <CharacterCreationSidebar />
          </div>

        </div>

        <!-- ── Footer ── -->
        <div class="shrink-0 border-t border-gray-800 bg-gray-950 px-5 py-3">
          <div class="mx-auto max-w-3xl">
            <div
              v-if="store.error || store.validationError"
              class="mb-2 text-xs text-red-400"
              data-testid="cc-error"
            >
              {{ store.error ?? store.validationError }}
            </div>
            <button
              type="button"
              class="w-full rounded border py-2.5 text-sm font-semibold transition-colors
                border-green-700 text-green-300 hover:bg-green-900/30
                disabled:cursor-not-allowed disabled:border-gray-700 disabled:text-gray-600"
              :disabled="!store.canSubmit"
              data-testid="cc-create"
              @click="create"
            >
              {{ store.submitting ? "Creating…" : "Create Character" }}
            </button>
          </div>
        </div>

      </template>
    </div>
  </Teleport>
</template>
