<script setup lang="ts">
/**
 * Turn control bar (UI-rework PR3; time controls added in issue #332) — the
 * persistent footer. Shows the current-turn indicator (Sol / Month) on the left
 * and the turn controls pinned bottom-right, so time is reachable from every
 * in-game screen. Also hosts the global event toast.
 *
 * ## Time controls
 *
 * The kernel keeps discrete sols (CLAUDE.md rules 3 and 4 — `apply(Command)`
 * and snapshot-per-turn), so "continuous time" here is a **UI timer** issuing
 * one `fast_forward` per tick. Play/pause and the speed selector only change
 * how often that timer fires; the simulation itself is unchanged, which is what
 * keeps fast-forward byte-identical to stepping.
 *
 * A run stops early on any interrupt at or above the chosen halt threshold. When
 * that happens the timer stops too and the digest panel opens — the player is
 * meant to be handed control back, not have the clock roll on past a crisis.
 */

import { computed, onUnmounted, ref, watch } from 'vue'

import { useWorldStore } from '@/stores/worldStore'
import { useGameStore } from '@/stores/game'
import { getInterruptDigest, type InterruptDigest } from '@/services/tauriBridge'
import type { InterruptTier } from '@/types/commands'
import type { GameEvent } from '@/types/gameEvents'

const world = useWorldStore()
const gameStore = useGameStore()

/** Speed presets: how many sols each timer tick advances, and how often it fires. */
const SPEEDS = [
  { label: '1×', solsPerTick: 1, intervalMs: 1200 },
  { label: '2×', solsPerTick: 1, intervalMs: 600 },
  { label: '5×', solsPerTick: 5, intervalMs: 600 },
] as const

/** Sols a single "Fast Forward" click asks for. */
const FAST_FORWARD_SOLS = 30

/**
 * Halt threshold. `urgent` is the tier documented as "halts fast-forward and
 * hands control back", so it is the default; `blocking` lets the player run
 * through anything short of a decision the sim cannot continue without.
 */
const THRESHOLDS: InterruptTier[] = ['notable', 'urgent', 'blocking']

const speedIndex = ref(0)
const threshold = ref<InterruptTier>('urgent')
const playing = ref(false)
const digest = ref<InterruptDigest | null>(null)
const digestOpen = ref(false)

let timer: ReturnType<typeof setInterval> | null = null

const speed = computed(() => SPEEDS[speedIndex.value])

function stopTimer(): void {
  if (timer !== null) {
    clearInterval(timer)
    timer = null
  }
}

/** Stop the clock and surface what happened. Called on halt and on error. */
async function haltAndShowDigest(): Promise<void> {
  playing.value = false
  stopTimer()
  try {
    digest.value = await getInterruptDigest()
    digestOpen.value = true
  } catch {
    // The digest is a nicety; failing to read it must not leave the UI stuck
    // thinking it is still playing. The halt itself already took effect.
    digest.value = null
  }
}

/** Issue one fast-forward, and stop the clock if it came back halted. */
async function tick(sols: number): Promise<void> {
  if (gameStore.busy) return // let the in-flight command finish first

  // `gameStore.sendCommand` catches its own errors, surfaces a toast, and
  // resolves with `[]` — it does not reject. So a rejection is not the failure
  // signal here; a *missing terminator* is. A successful run always emits
  // exactly one `fast_forward_ended`, so its absence means the command was
  // rejected. Without this the timer would keep re-issuing a failing command
  // every tick, forever, with nothing but a toast to show for it.
  //
  // The try/catch stays only to cover a future store that does throw.
  let events: GameEvent[]
  try {
    events = await gameStore.sendCommand({
      kind: 'fast_forward',
      max_sols: sols,
      threshold: threshold.value,
    })
  } catch {
    playing.value = false
    stopTimer()
    return
  }

  const ended = events.find((e) => e.kind === 'fast_forward_ended')
  if (!ended) {
    // Rejected (or an unrecognised reply). Stop rather than spin.
    playing.value = false
    stopTimer()
    return
  }
  if (ended.kind === 'fast_forward_ended' && ended.halted) await haltAndShowDigest()
}

function togglePlay(): void {
  playing.value = !playing.value
}

async function advanceOne(): Promise<void> {
  await gameStore.sendCommand({ kind: 'advance_sol' })
}

async function fastForward(): Promise<void> {
  await tick(FAST_FORWARD_SOLS)
}

// Restart the timer whenever play state or speed changes, so changing speed
// mid-run takes effect immediately rather than after the current interval.
watch([playing, speedIndex], () => {
  stopTimer()
  if (!playing.value) return
  timer = setInterval(() => {
    void tick(speed.value.solsPerTick)
  }, speed.value.intervalMs)
})

onUnmounted(stopTimer)
</script>

<template>
  <footer class="turn-control-bar" data-testid="turn-control-bar">
    <div class="turn-indicator" data-testid="turn-indicator">
      Sol {{ world.sol }} · Month {{ world.month }}
    </div>

    <div
      v-if="gameStore.toastMessage"
      class="toast"
      data-testid="event-toast"
      @click="gameStore.dismissToast()"
    >
      {{ gameStore.toastMessage }}
    </div>

    <div class="time-controls">
      <button
        class="btn-time"
        :class="{ active: playing }"
        :title="playing ? 'Pause' : 'Play'"
        data-testid="btn-play-pause"
        @click="togglePlay"
      >
        {{ playing ? '❙❙ Pause' : '▶ Play' }}
      </button>

      <div class="speed-group" role="group" aria-label="Speed">
        <button
          v-for="(s, i) in SPEEDS"
          :key="s.label"
          class="btn-speed"
          :class="{ active: i === speedIndex }"
          :data-testid="`btn-speed-${i}`"
          @click="speedIndex = i"
        >
          {{ s.label }}
        </button>
      </div>

      <label class="threshold">
        Stop on
        <select v-model="threshold" data-testid="select-threshold">
          <option v-for="t in THRESHOLDS" :key="t" :value="t">{{ t }}</option>
        </select>
      </label>

      <button
        class="btn-time"
        :disabled="gameStore.busy"
        title="Advance until something interesting happens"
        data-testid="btn-fast-forward"
        @click="fastForward"
      >
        ▶▶ {{ FAST_FORWARD_SOLS }} Sols
      </button>

      <button
        class="btn-advance"
        :disabled="gameStore.busy"
        data-testid="btn-advance-turn"
        @click="advanceOne"
      >
        Advance Turn ▶
      </button>
    </div>

    <div v-if="digestOpen" class="digest" data-testid="interrupt-digest">
      <div class="digest-head">
        <strong>Stopped at sol {{ digest?.stopped_at_sol ?? world.sol }}</strong>
        <button class="btn-close" data-testid="btn-close-digest" @click="digestOpen = false">
          ✕
        </button>
      </div>
      <p v-if="digest?.halting_message" class="digest-halt" data-testid="digest-halt">
        <span class="tier">{{ digest.halting_tier }}</span>
        {{ digest.halting_message }}
      </p>
      <ul v-if="digest && digest.items.length > 0" class="digest-items">
        <li v-for="(item, i) in digest.items" :key="i">
          <span class="tier">{{ item.tier }}</span> {{ item.message }}
        </li>
      </ul>
      <p v-else class="digest-empty" data-testid="digest-empty">
        Nothing else accumulated on the way here.
      </p>
    </div>
  </footer>
</template>

<style scoped>
.turn-control-bar {
  position: relative;
  display: flex;
  align-items: center;
  gap: 1rem;
  padding: 0.4rem 1rem;
  background: #111118;
  border-top: 1px solid #334;
}

.turn-indicator { color: #8a8; font-size: 0.85rem; white-space: nowrap; }

.toast {
  flex: 1;
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  background: #1a2020;
  border: 1px solid #2a5a3a;
  border-radius: 3px;
  color: #6c9;
  padding: 0.3rem 0.6rem;
  font-size: 0.78rem;
  cursor: pointer;
}

.time-controls {
  margin-left: auto;
  display: flex;
  align-items: center;
  gap: 0.5rem;
}

.btn-time,
.btn-speed {
  background: #14202e;
  border: 1px solid #468;
  border-radius: 3px;
  color: #8cf;
  padding: 0.35rem 0.6rem;
  font-family: monospace;
  font-size: 0.8rem;
  cursor: pointer;
  white-space: nowrap;
}
.btn-time:hover:not(:disabled),
.btn-speed:hover:not(:disabled) { background: #1b2c40; }
.btn-time:disabled,
.btn-speed:disabled { opacity: 0.45; cursor: not-allowed; }
.btn-time.active,
.btn-speed.active { background: #24405c; color: #cef; border-color: #6ac; }

.speed-group { display: flex; gap: 2px; }

.threshold { color: #789; font-size: 0.72rem; display: flex; align-items: center; gap: 0.25rem; }
.threshold select {
  background: #14202e;
  border: 1px solid #468;
  color: #8cf;
  font-family: monospace;
  font-size: 0.72rem;
  border-radius: 3px;
}

/* Pin the advance button to the bottom-right of the screen. */
.btn-advance {
  background: #14202e;
  border: 1px solid #468;
  border-radius: 3px;
  color: #8cf;
  padding: 0.45rem 1.1rem;
  font-family: monospace;
  font-size: 0.9rem;
  font-weight: bold;
  cursor: pointer;
  white-space: nowrap;
}
.btn-advance:hover:not(:disabled) { background: #1b2c40; }
.btn-advance:disabled { opacity: 0.45; cursor: not-allowed; }

.digest {
  position: absolute;
  right: 1rem;
  bottom: 100%;
  margin-bottom: 0.4rem;
  width: min(30rem, calc(100vw - 2rem));
  max-height: 50vh;
  overflow-y: auto;
  background: #15151f;
  border: 1px solid #556;
  border-radius: 4px;
  padding: 0.6rem 0.8rem;
  font-size: 0.78rem;
  color: #bbc;
  box-shadow: 0 4px 16px rgb(0 0 0 / 50%);
}
.digest-head { display: flex; align-items: center; justify-content: space-between; color: #dde; }
.btn-close {
  background: none;
  border: none;
  color: #889;
  cursor: pointer;
  font-size: 0.9rem;
}
.digest-halt { color: #fc8; margin: 0.4rem 0; }
.digest-items { margin: 0.3rem 0 0; padding-left: 1rem; }
.digest-items li { margin: 0.15rem 0; }
.digest-empty { color: #778; font-style: italic; margin: 0.4rem 0 0; }
.tier {
  display: inline-block;
  min-width: 4.5rem;
  color: #7a9;
  text-transform: uppercase;
  font-size: 0.68rem;
}
</style>
