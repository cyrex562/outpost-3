<script setup lang="ts">
import { ref, onMounted, computed } from 'vue'
import { useRouter } from 'vue-router'
import { getTechTree, type TechNode, type TechEffect } from '@/services/tauriBridge'
import { useGameStore } from '@/stores/game'

const router = useRouter()
const gameStore = useGameStore()

const nodes = ref<TechNode[]>([])
const selected = ref<TechNode | null>(null)
const error = ref<string | null>(null)
const categoryFilter = ref<string>('')
const tierFilter = ref<string>('')

async function refresh(): Promise<void> {
  try {
    nodes.value = await getTechTree()
    if (selected.value) {
      selected.value = nodes.value.find((n) => n.id === selected.value?.id) ?? null
    }
  } catch (e) {
    error.value = e instanceof Error ? e.message : String(e)
  }
}

onMounted(refresh)

// ── Category / tier browsing (issue #250 — the tree grew from 12 to 81 nodes) ──

const categories = computed<string[]>(() => {
  const set = new Set(nodes.value.map((n) => n.category).filter(Boolean))
  return Array.from(set).sort()
})

const tierOptions = computed<number[]>(() => {
  const set = new Set(nodes.value.map((n) => n.tier))
  return Array.from(set).sort((a, b) => a - b)
})

const filteredNodes = computed<TechNode[]>(() =>
  nodes.value.filter((n) => {
    if (categoryFilter.value && n.category !== categoryFilter.value) return false
    if (tierFilter.value && String(n.tier) !== tierFilter.value) return false
    return true
  }),
)

/** The player's queued-but-not-yet-active research, in actual FIFO drain order. */
const queuedNodes = computed<TechNode[]>(() =>
  nodes.value
    .filter((n) => n.state === 'queued')
    .sort((a, b) => (a.queue_position ?? 0) - (b.queue_position ?? 0)),
)

const activeNode = computed<TechNode | undefined>(() =>
  nodes.value.find((n) => n.state === 'in_progress'),
)

// ── Layered layout: tier = 1 + max(tier(prereqs)) ─────────────────────────

const tiers = computed<TechNode[][]>(() => {
  const byId = new Map<string, TechNode>(nodes.value.map((n) => [n.id, n]))
  const tierOf = new Map<string, number>()

  function resolve(id: string, seen = new Set<string>()): number {
    if (tierOf.has(id)) return tierOf.get(id)!
    if (seen.has(id)) return 0
    seen.add(id)
    const node = byId.get(id)
    if (!node || node.prerequisites.length === 0) {
      tierOf.set(id, 0)
      return 0
    }
    const t = 1 + Math.max(...node.prerequisites.map((p) => resolve(p, seen)))
    tierOf.set(id, t)
    return t
  }

  const buckets: TechNode[][] = []
  for (const n of filteredNodes.value) {
    const t = resolve(n.id)
    while (buckets.length <= t) buckets.push([])
    buckets[t].push(n)
  }
  return buckets
})

// ── SVG geometry ──────────────────────────────────────────────────────────

const COL_W = 200
const ROW_H = 80
const NODE_W = 160
const NODE_H = 52

const totalWidth = computed(() => Math.max(1, tiers.value.length) * COL_W + 40)
const totalHeight = computed(() => {
  const max = Math.max(1, ...tiers.value.map((t) => t.length))
  return max * ROW_H + 40
})

function nodePos(tier: number, idx: number): { x: number; y: number } {
  return {
    x: 20 + tier * COL_W,
    y: 20 + idx * ROW_H,
  }
}

interface Positioned extends TechNode {
  x: number
  y: number
}

const laidOut = computed<Positioned[]>(() => {
  const out: Positioned[] = []
  for (let t = 0; t < tiers.value.length; t++) {
    for (let i = 0; i < tiers.value[t].length; i++) {
      const n = tiers.value[t][i]
      const { x, y } = nodePos(t, i)
      out.push({ ...n, x, y })
    }
  }
  return out
})

const positionById = computed(() => {
  const m = new Map<string, Positioned>()
  for (const n of laidOut.value) m.set(n.id, n)
  return m
})

interface Edge {
  x1: number
  y1: number
  x2: number
  y2: number
  key: string
}

const edges = computed<Edge[]>(() => {
  const list: Edge[] = []
  for (const n of laidOut.value) {
    for (const p of n.prerequisites) {
      const from = positionById.value.get(p)
      if (!from) continue
      list.push({
        key: `${p}-${n.id}`,
        x1: from.x + NODE_W,
        y1: from.y + NODE_H / 2,
        x2: n.x,
        y2: n.y + NODE_H / 2,
      })
    }
  }
  return list
})

function nodeClass(state: string): string {
  return `state-${state}`
}

async function research(node: TechNode): Promise<void> {
  await gameStore.sendCommand({ kind: 'research_tech', tech_id: node.id })
  await refresh()
}

async function enqueue(node: TechNode): Promise<void> {
  await gameStore.sendCommand({ kind: 'enqueue_research', tech_id: node.id })
  await refresh()
}

async function cancelResearch(): Promise<void> {
  await gameStore.sendCommand({ kind: 'cancel_research' })
  await refresh()
}

function effectLabel(effect: TechEffect): string {
  switch (effect.type) {
    case 'unlock_building':
      return `Unlocks building: ${effect.building_id}`
    case 'unlock_commodity':
      return `Unlocks commodity: ${effect.commodity_id}`
    case 'unlock_capability':
      return `Unlocks capability: ${effect.capability_id}`
    case 'bonus':
      return `+${(effect.value * 100).toFixed(0)}% ${effect.category.replace(/_/g, ' ')}`
    case 'mitigate_attribute':
      return `Mitigates ${effect.attribute.replace(/_/g, ' ')} penalty`
    case 'survey_modifier_bonus':
      return `+${(effect.full_reveal_bonus * 100).toFixed(0)}% full-reveal / +${(effect.partial_reveal_bonus * 100).toFixed(0)}% partial-reveal survey odds`
    case 'reduce_transit_time':
      return `-${(effect.fraction * 100).toFixed(0)}% expedition transit time`
    case 'extend_outpost_range':
      return `+${effect.bonus_au.toFixed(1)} AU outpost range`
    default:
      return JSON.stringify(effect)
  }
}
</script>

<template>
  <div class="tech-view" data-testid="tech-tree">
    <header class="head">
      <h2>Tech Tree</h2>
      <div class="legend">
        <span class="legend-item state-researched">researched</span>
        <span class="legend-item state-in_progress">in progress</span>
        <span class="legend-item state-queued">queued</span>
        <span class="legend-item state-available">available</span>
        <span class="legend-item state-locked">locked</span>
      </div>
      <button class="btn" @click="router.push('/system')">Back to System</button>
    </header>

    <div class="filters" data-testid="tech-filters">
      <label>
        Category
        <select v-model="categoryFilter" data-testid="tech-filter-category">
          <option value="">All</option>
          <option v-for="c in categories" :key="c" :value="c">{{ c }}</option>
        </select>
      </label>
      <label>
        Tier
        <select v-model="tierFilter" data-testid="tech-filter-tier">
          <option value="">All</option>
          <option v-for="t in tierOptions" :key="t" :value="String(t)">{{ t }}</option>
        </select>
      </label>
    </div>

    <div v-if="activeNode || queuedNodes.length" class="queue-panel" data-testid="research-queue">
      <div v-if="activeNode" class="queue-active">
        Researching: <strong>{{ activeNode.name }}</strong>
        ({{ (activeNode.progress * 100).toFixed(0) }}%)
        <button class="btn" :disabled="gameStore.busy" @click="cancelResearch">
          {{ queuedNodes.length ? `Cancel (clears ${queuedNodes.length} queued too)` : 'Cancel' }}
        </button>
      </div>
      <ol v-if="queuedNodes.length" class="queue-list">
        <li v-for="n in queuedNodes" :key="n.id">{{ n.name }}</li>
      </ol>
    </div>

    <div class="tech-body">
    <div class="graph-wrap">
      <svg
        :width="totalWidth"
        :height="totalHeight"
        class="graph"
        data-testid="tech-graph-svg"
      >
        <g>
          <path
            v-for="e in edges"
            :key="e.key"
            :d="`M ${e.x1} ${e.y1} C ${(e.x1 + e.x2) / 2} ${e.y1}, ${(e.x1 + e.x2) / 2} ${e.y2}, ${e.x2} ${e.y2}`"
            fill="none"
            class="tech-edge-line"
            stroke-width="1.5"
          />
        </g>
        <g>
          <g
            v-for="n in laidOut"
            :key="n.id"
            :class="['node', nodeClass(n.state), { selected: n.id === selected?.id }]"
            :data-testid="`tech-node-${n.id}`"
            @click="selected = n"
          >
            <rect
              :x="n.x"
              :y="n.y"
              :width="NODE_W"
              :height="NODE_H"
              rx="4"
              ry="4"
            />
            <text
              :x="n.x + NODE_W / 2"
              :y="n.y + 20"
              text-anchor="middle"
              class="node-title"
            >
              {{ n.name }}
            </text>
            <text
              :x="n.x + NODE_W / 2"
              :y="n.y + 38"
              text-anchor="middle"
              class="node-sub"
            >
              {{ n.cost.toFixed(0) }} RP
            </text>
            <rect
              v-if="n.state === 'in_progress'"
              :x="n.x + 4"
              :y="n.y + NODE_H - 6"
              :width="(NODE_W - 8) * Math.min(1, Math.max(0, n.progress))"
              height="4"
              class="tech-node-fill"
            />
          </g>
        </g>
      </svg>
    </div>

    <aside class="detail" data-testid="tech-detail">
      <template v-if="selected">
      <h3>{{ selected.name }}</h3>
      <div class="badges">
        <span class="status" :class="nodeClass(selected.state)">
          {{ selected.state.replace('_', ' ') }}
        </span>
        <span v-if="selected.category" class="badge category">{{ selected.category }}</span>
        <span class="badge tier">tier {{ selected.tier }}</span>
      </div>
      <p class="desc">{{ selected.description || '—' }}</p>
      <dl class="stats">
        <dt>Cost</dt><dd>{{ selected.cost.toFixed(0) }} RP</dd>
        <dt>Prerequisites</dt>
        <dd>
          {{ selected.prerequisites.length ? selected.prerequisites.join(', ') : 'None' }}
        </dd>
      </dl>
      <ul v-if="selected.effects.length" class="effects" data-testid="tech-effects">
        <li v-for="(e, i) in selected.effects" :key="i">{{ effectLabel(e) }}</li>
      </ul>
      <div class="actions">
        <button
          v-if="selected.state === 'available' && !activeNode"
          class="btn primary"
          data-testid="tech-research-btn"
          :disabled="gameStore.busy"
          @click="research(selected)"
        >
          Research
        </button>
        <button
          v-if="selected.state === 'available' && activeNode"
          class="btn primary"
          data-testid="tech-enqueue-btn"
          :disabled="gameStore.busy"
          @click="enqueue(selected)"
        >
          Queue after {{ activeNode.name }}
        </button>
      </div>
      <div v-if="selected.state === 'locked'" class="hint">
        Complete prerequisites to unlock.
      </div>
      </template>
      <p v-else class="hint select-hint" data-testid="tech-detail-empty">
        Click a tech node to see its details and research it.
      </p>
    </aside>
    </div>

    <p v-if="error" class="err">{{ error }}</p>
  </div>
</template>

<style scoped>
/* Inline SVG paint attributes can't read custom properties — these classes
   carry the themed colours for the tech-graph edges and progress bars. */
.tech-edge-line { stroke: var(--border); }
.tech-node-fill { fill: var(--accent); }

.tech-view { display: flex; flex-direction: column; gap: 0.75rem; height: 100%; }

/* Graph + detail sit side by side and fill the remaining height, so
   selecting a node shows its details + Research button without scrolling
   past a tall graph (UI-rework: tech selection was effectively hidden below
   the fold). */
.tech-body { flex: 1; min-height: 0; display: flex; gap: 0.75rem; }
.head { display: flex; align-items: center; gap: 1rem; flex-wrap: wrap; }
.head h2 { color: var(--accent); }
.legend { display: flex; gap: 0.5rem; margin-left: auto; }
.legend-item {
  font-size: 0.72rem;
  padding: 0.2rem 0.5rem;
  border-radius: 3px;
  border: 1px solid var(--border);
}
.legend-item.state-researched { color: var(--good-dim); border-color: var(--good-border); }
.legend-item.state-in_progress { color: var(--accent); border-color: var(--border-accent); }
.legend-item.state-queued { color: var(--warn-dim); border-color: var(--text-dim); }
.legend-item.state-available { color: var(--good-dim); border-color: var(--good-border); }
.legend-item.state-locked { color: var(--text-faint); border-color: var(--border); }

.filters { display: flex; gap: 1rem; align-items: center; font-size: 0.8rem; color: var(--text); }
.filters select {
  background: var(--surface-2);
  border: 1px solid var(--border);
  color: var(--text);
  border-radius: 3px;
  padding: 0.2rem 0.4rem;
  font-family: monospace;
}

.queue-panel {
  background: var(--surface-1);
  border: 1px solid var(--border);
  border-radius: 6px;
  padding: 0.6rem 0.8rem;
  font-size: 0.82rem;
  color: var(--text);
  display: flex;
  flex-direction: column;
  gap: 0.4rem;
}
.queue-active { display: flex; align-items: center; gap: 0.6rem; }
.queue-active strong { color: var(--accent); }
.queue-list { margin: 0; padding-left: 1.2rem; color: var(--warn-dim); }

.actions { display: flex; gap: 0.5rem; margin-bottom: 0.5rem; }

.effects {
  list-style: none;
  margin: 0 0 0.75rem;
  padding: 0;
  display: flex;
  flex-direction: column;
  gap: 0.25rem;
  font-size: 0.78rem;
  color: var(--good);
}
.effects li::before { content: '▸ '; color: var(--text-faint); }

.graph-wrap {
  flex: 1;
  min-height: 0;
  overflow: auto;
  background: var(--map-bg-inner);
  border: 1px solid var(--border-subtle);
  border-radius: 6px;
}
.graph { display: block; }

.node { cursor: pointer; }
.node rect { fill: var(--surface-2); stroke: var(--border); stroke-width: 1; transition: fill 0.1s; }
.node:hover rect { fill: var(--surface-alt); }
.node-title { fill: var(--text); font-family: monospace; font-size: 12px; pointer-events: none; }
.node-sub   { fill: var(--text-faint); font-family: monospace; font-size: 10px; pointer-events: none; }

.node.state-researched rect { fill: var(--good-bg); stroke: var(--good-border); }
.node.state-researched .node-title { fill: var(--good-dim); }
.node.state-in_progress rect { fill: var(--accent-bg); stroke: var(--border-accent); }
.node.state-in_progress .node-title { fill: var(--accent); }
.node.state-queued rect { fill: var(--accent-bg); stroke: var(--text-dim); }
.node.state-queued .node-title { fill: var(--warn-dim); }
.node.state-available rect { fill: var(--warn-bg); stroke: var(--good-border); }
.node.state-available .node-title { fill: var(--good-dim); }
.node.state-locked rect { fill: var(--surface-3); stroke: var(--border-subtle); opacity: 0.65; }
.node.state-locked .node-title { fill: var(--text-faint); }

/* Selected node — a bright outline so a click is obviously registered. */
.node.selected rect { stroke: var(--accent); stroke-width: 2.5; }

.detail {
  flex: none;
  width: 340px;
  overflow-y: auto;
  background: var(--surface-1);
  border: 1px solid var(--border);
  border-radius: 6px;
  padding: 1rem;
  color: var(--text);
}
.select-hint { margin: 0; }
.detail h3 { color: var(--accent); margin-bottom: 0.25rem; }
.status {
  display: inline-block;
  padding: 0.15rem 0.5rem;
  border-radius: 3px;
  font-size: 0.75rem;
  margin-bottom: 0.5rem;
}
.status.state-researched { color: var(--good-dim); border: 1px solid var(--good-border); }
.status.state-in_progress { color: var(--accent); border: 1px solid var(--border-accent); }
.status.state-queued { color: var(--warn-dim); border: 1px solid var(--text-dim); }
.status.state-available { color: var(--good-dim); border: 1px solid var(--good-border); }
.status.state-locked { color: var(--text-faint); border: 1px solid var(--border); }

.badges { display: flex; gap: 0.4rem; margin-bottom: 0.6rem; flex-wrap: wrap; }
.badge {
  display: inline-block;
  padding: 0.15rem 0.5rem;
  border-radius: 3px;
  font-size: 0.7rem;
  border: 1px solid var(--border);
  color: var(--text-muted);
  text-transform: uppercase;
  letter-spacing: 0.04em;
}
.badge.category { color: var(--accent-soft); border-color: var(--border-strong); }
.badge.tier { color: var(--good); border-color: var(--text-faint); }

.desc { color: var(--text); margin-bottom: 0.5rem; font-size: 0.85rem; }
.stats { display: grid; grid-template-columns: 120px 1fr; gap: 0.3rem 0.6rem; font-size: 0.8rem; margin-bottom: 0.75rem; }
.stats dt { color: var(--text-dim); }
.stats dd { color: var(--text); }

.btn {
  background: var(--surface-btn);
  border: 1px solid var(--border-strong);
  border-radius: 3px;
  color: var(--text);
  padding: 0.4rem 0.75rem;
  font-family: monospace;
  font-size: 0.82rem;
  cursor: pointer;
}
.btn:hover:not(:disabled) { background: var(--surface-btn-hover); }
.btn:disabled { opacity: 0.45; cursor: not-allowed; }
.btn.primary { border-color: var(--border-accent); color: var(--accent); }
.hint { color: var(--text-faint); font-style: italic; font-size: 0.8rem; }
.err { color: var(--danger); font-size: 0.85rem; }
</style>
