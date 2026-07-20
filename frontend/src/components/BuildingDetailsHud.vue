<script setup lang="ts">
/**
 * Building details (issue #182) — a single building type's full production
 * picture: category, recipe flows, maintenance upkeep, and the outcome of
 * its most recent production attempt (scale + shortfalls, including a
 * dedicated MaintenanceShort indicator surfacing #180's per-sol upkeep
 * gate). Renders either as a modal overlay (default) or, with `asPage`, as
 * plain inline content for `FacilityView.vue`'s routed facility page
 * (navigation rework #7 phase 2 — promoted from modal-only so a facility
 * has its own back-stack position instead of only existing as long as a
 * parent component's local ref stays non-null).
 *
 * `ownerType` selects colony- or outpost-scoped detail/recipe-selection
 * calls (navigation rework #7 phase 4 — outpost drill-down parity with
 * colonies); the response shape and rendering are identical either way.
 *
 * Fetches fresh detail every time `buildingType` changes, since scale/
 * shortfalls are per-sol and stale data would misrepresent the building's
 * current state.
 */

import { ref, watch } from 'vue'
import {
  getBuildingDetail,
  setActiveRecipe,
  getOutpostBuildingDetail,
  setOutpostActiveRecipe,
  type BuildingDetail,
} from '@/services/tauriBridge'

const props = withDefaults(
  defineProps<{
    /** Which owner kind `ownerId` refers to. Defaults to `'colony'` for pre-#7-phase-4 callers. */
    ownerType?: 'colony' | 'outpost'
    ownerId: string | null
    /** `null` closes the HUD (modal mode only — ignored in page mode). */
    buildingType: string | null
    /** Render as plain inline content (no backdrop/overlay) for a routed page. */
    asPage?: boolean
  }>(),
  { ownerType: 'colony', asPage: false },
)

const emit = defineEmits<{
  (e: 'close'): void
}>()

const detail = ref<BuildingDetail | null>(null)
const loading = ref(false)
const error = ref<string | null>(null)
const switching = ref(false)

async function load(): Promise<void> {
  if (!props.ownerId || !props.buildingType) {
    detail.value = null
    return
  }
  loading.value = true
  error.value = null
  try {
    detail.value =
      props.ownerType === 'outpost'
        ? await getOutpostBuildingDetail(props.ownerId, props.buildingType)
        : await getBuildingDetail(props.ownerId, props.buildingType)
  } catch (e) {
    error.value = e instanceof Error ? e.message : String(e)
    detail.value = null
  } finally {
    loading.value = false
  }
}

watch(() => [props.ownerType, props.ownerId, props.buildingType], load, { immediate: true })

async function switchRecipe(recipeId: string): Promise<void> {
  if (!props.ownerId || !props.buildingType || switching.value) return
  switching.value = true
  try {
    if (props.ownerType === 'outpost') {
      await setOutpostActiveRecipe(props.ownerId, props.buildingType, recipeId)
    } else {
      await setActiveRecipe(props.ownerId, props.buildingType, recipeId)
    }
    await load()
  } catch (e) {
    error.value = e instanceof Error ? e.message : String(e)
  } finally {
    switching.value = false
  }
}

function hasMaintenanceShort(d: BuildingDetail): boolean {
  return d.last_run?.shortfalls.some((s) => s.kind === 'maintenance_short') ?? false
}

const SHORTFALL_LABELS: Record<string, string> = {
  input_short: 'Input shortage',
  power_brownout: 'Power brownout',
  labor_short: 'Labour shortage',
  maintenance_short: 'Maintenance shortage',
}

function shortfallLabel(kind: string): string {
  return SHORTFALL_LABELS[kind] ?? kind
}

/** Backdrop click-outside-to-close only makes sense in modal mode — a page
 * has no overlay to click "outside" of. */
function onBackdropClick(): void {
  if (!props.asPage) emit('close')
}
</script>

<template>
  <div
    v-if="asPage || props.buildingType !== null"
    :class="asPage ? 'facility-page' : 'hud-backdrop'"
    :data-testid="asPage ? 'facility-page' : 'building-details-hud'"
    @click.self="onBackdropClick"
  >
    <div :class="['hud-panel', { 'hud-panel--page': asPage }]">
      <div class="hud-header">
        <button v-if="asPage" class="btn-back" data-testid="facility-back" @click="emit('close')">
          ← Back
        </button>
        <h4 class="hud-title">{{ detail?.name ?? props.buildingType }}</h4>
        <button v-if="!asPage" class="btn-close" data-testid="close-details-hud" @click="emit('close')">×</button>
      </div>

      <div v-if="loading" class="hint">Loading…</div>
      <div v-else-if="error" class="error" data-testid="building-details-error">{{ error }}</div>

      <template v-else-if="detail">
        <p v-if="detail.description" class="description">{{ detail.description }}</p>

        <div class="meta-row">
          <span class="meta-item">Category: {{ detail.category }}</span>
          <span class="meta-item">Slots: {{ detail.slot_cost }}</span>
          <span class="meta-item">Power: {{ detail.power_delta >= 0 ? '-' : '+' }}{{ Math.abs(detail.power_delta) }} kW</span>
        </div>

        <div
          v-if="detail.last_run && hasMaintenanceShort(detail)"
          class="maintenance-short-banner"
          data-testid="maintenance-short-indicator"
        >
          MAINTENANCE SHORT
        </div>

        <section v-if="detail.last_run" class="section" data-testid="last-run-section">
          <h5>Last Run</h5>
          <p>Scale: {{ (detail.last_run.scale * 100).toFixed(0) }}%
            <span v-if="detail.last_run.is_full_production" class="tag-full">Full production</span>
          </p>
          <ul v-if="detail.last_run.shortfalls.length > 0" class="shortfall-list">
            <li
              v-for="(s, i) in detail.last_run.shortfalls"
              :key="i"
              class="shortfall-item"
              :class="{ 'shortfall-maintenance': s.kind === 'maintenance_short' }"
              :data-testid="`shortfall-${s.kind}`"
            >
              {{ shortfallLabel(s.kind) }}<span v-if="s.commodity_id"> ({{ s.commodity_id }})</span>
            </li>
          </ul>
        </section>
        <p v-else class="hint">No production data yet — this building hasn't run a sol.</p>

        <section v-if="detail.recipe" class="section" data-testid="recipe-section">
          <h5>Recipe: {{ detail.recipe.name }}</h5>
          <div v-if="detail.available_recipes.length > 1" class="recipe-selector" data-testid="recipe-selector">
            <label class="hint" for="recipe-select">Switch recipe:</label>
            <select
              id="recipe-select"
              class="recipe-select"
              data-testid="recipe-select"
              :value="detail.recipe.recipe_id"
              :disabled="switching"
              @change="switchRecipe(($event.target as HTMLSelectElement).value)"
            >
              <option v-for="r in detail.available_recipes" :key="r.recipe_id" :value="r.recipe_id">
                {{ r.name }}
              </option>
            </select>
          </div>
          <p class="flow-row">
            <span class="flow-label">In:</span>
            <span v-for="i in detail.recipe.inputs" :key="i.commodity_id" class="flow-item">
              {{ i.commodity_id }} ×{{ i.quantity }}
            </span>
            <span v-if="detail.recipe.inputs.length === 0" class="hint">none</span>
          </p>
          <p class="flow-row">
            <span class="flow-label">Out:</span>
            <span v-for="i in detail.recipe.outputs" :key="i.commodity_id" class="flow-item">
              {{ i.commodity_id }} ×{{ i.quantity }}
            </span>
          </p>
          <p class="flow-row">Cycle: {{ detail.recipe.cycle_sols }} sol{{ detail.recipe.cycle_sols !== 1 ? 's' : '' }}</p>
        </section>

        <section
          v-if="detail.concurrent_recipes.length > 0"
          class="section"
          data-testid="concurrent-recipes-section"
        >
          <h5>Always-on recipes</h5>
          <div v-for="r in detail.concurrent_recipes" :key="r.recipe_id" class="concurrent-recipe">
            <p class="flow-row concurrent-recipe-name">{{ r.name }}</p>
            <p class="flow-row">
              <span class="flow-label">In:</span>
              <span v-for="i in r.inputs" :key="i.commodity_id" class="flow-item">
                {{ i.commodity_id }} ×{{ i.quantity }}
              </span>
              <span v-if="r.inputs.length === 0" class="hint">none</span>
            </p>
            <p class="flow-row">
              <span class="flow-label">Out:</span>
              <span v-for="i in r.outputs" :key="i.commodity_id" class="flow-item">
                {{ i.commodity_id }} ×{{ i.quantity }}
              </span>
            </p>
          </div>
        </section>

        <p v-if="!detail.recipe && detail.concurrent_recipes.length === 0" class="hint">
          No recipe (storage/habitat building).
        </p>

        <section v-if="detail.maintenance.length > 0" class="section" data-testid="maintenance-section">
          <h5>Maintenance (per sol)</h5>
          <p class="flow-row">
            <span v-for="i in detail.maintenance" :key="i.commodity_id" class="flow-item">
              {{ i.commodity_id }} ×{{ i.quantity }}
            </span>
          </p>
        </section>
      </template>
    </div>
  </div>
</template>

<style scoped>
.hud-backdrop {
  position: fixed;
  inset: 0;
  background: rgba(0, 0, 0, 0.55);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 100;
}

.hud-panel {
  background: #0d0d15;
  border: 1px solid #334;
  border-radius: 4px;
  padding: 1rem 1.25rem;
  width: min(480px, 90vw);
  max-height: 80vh;
  overflow-y: auto;
  font-family: monospace;
  color: #aab;
}

.facility-page { padding: 1rem; }
.hud-panel--page {
  width: min(640px, 100%);
  max-height: none;
  margin: 0 auto;
}

/* Modal mode: (title, close-button) — spread to opposite edges.
   Page mode: (back-button, title) — sit together at the start instead. */
.hud-header { display: flex; justify-content: space-between; align-items: center; gap: 0.6rem; margin-bottom: 0.5rem; }
.facility-page .hud-header { justify-content: flex-start; }
.hud-title { color: #8cf; font-size: 1rem; margin: 0; }
.btn-back {
  background: none;
  border: 1px solid #334;
  border-radius: 3px;
  color: #aab;
  font-size: 0.78rem;
  padding: 0.2rem 0.5rem;
  cursor: pointer;
  margin-right: 0.6rem;
}
.btn-back:hover { color: #cdd; border-color: #446; }
.btn-close {
  background: none;
  border: none;
  color: #889;
  font-size: 1.2rem;
  cursor: pointer;
  line-height: 1;
}
.btn-close:hover { color: #cdd; }

.hint { font-size: 0.78rem; color: #446; font-style: italic; }
.error { font-size: 0.8rem; color: #e77; }
.description { font-size: 0.8rem; color: #99a; margin: 0.25rem 0 0.75rem; }

.meta-row { display: flex; gap: 0.75rem; flex-wrap: wrap; font-size: 0.75rem; color: #668; margin-bottom: 0.6rem; }

.maintenance-short-banner {
  background: #3a1a1a;
  border: 1px solid #a55;
  color: #f89;
  font-size: 0.75rem;
  font-weight: 700;
  letter-spacing: 0.04em;
  padding: 0.3rem 0.5rem;
  border-radius: 3px;
  margin-bottom: 0.6rem;
}

.section { margin-top: 0.75rem; }
.section h5 { color: #789; font-size: 0.8rem; margin: 0 0 0.3rem; }

.recipe-selector { display: flex; align-items: center; gap: 0.4rem; margin: 0.3rem 0 0.5rem; }
.recipe-select {
  background: #13131e;
  border: 1px solid #334;
  border-radius: 3px;
  color: #cdd;
  padding: 0.15rem 0.35rem;
  font-family: monospace;
  font-size: 0.75rem;
}
.recipe-select:disabled { opacity: 0.5; }

.flow-row { font-size: 0.78rem; margin: 0.2rem 0; display: flex; gap: 0.4rem; flex-wrap: wrap; align-items: baseline; }
.flow-label { color: #668; }
.flow-item { color: #aab; background: #13131e; border: 1px solid #223; border-radius: 3px; padding: 0.1rem 0.35rem; }

.concurrent-recipe { margin-bottom: 0.5rem; padding-left: 0.5rem; border-left: 2px solid #223; }
.concurrent-recipe-name { color: #789; }

.tag-full { color: #6adba5; font-size: 0.72rem; margin-left: 0.4rem; }

.shortfall-list { list-style: none; margin: 0.3rem 0 0; padding: 0; display: flex; flex-direction: column; gap: 0.25rem; }
.shortfall-item { font-size: 0.75rem; color: #eab764; }
.shortfall-item.shortfall-maintenance { color: #f89; }
</style>
