<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted } from 'vue'
import { useRouter, useRoute } from 'vue-router'
import { useGameSocket } from '@/composables/useGameSocket'
import { useTheme } from '@/composables/useTheme'
import { useWorldStore } from '@/stores/worldStore'
import {
  exitApp as tauriExit,
  isTauri,
  resetEngine,
  saveGame,
  listSaves,
  loadGame,
  setCustomDifficulty,
  snapshot as fetchSnapshot,
} from '@/services/tauriBridge'
import { useGameStore } from '@/stores/game'
import CustomDifficultyPanel from '@/components/CustomDifficultyPanel.vue'
import BalanceTuningPanel from '@/components/BalanceTuningPanel.vue'
import SystemStatsBar from '@/components/SystemStatsBar.vue'
import TurnControlBar from '@/components/TurnControlBar.vue'
import AlertToast from '@/components/AlertToast.vue'

useGameSocket()

const router = useRouter()
const route = useRoute()
const store = useWorldStore()
const gameStore = useGameStore()

const inGame = computed(() => route.path !== '/' && route.path !== '/menu')

// ── Theme ────────────────────────────────────────────────────────────────
// Three-state: follow the OS, or force light/dark. The icon shows the mode
// itself rather than what clicking does, so "auto" stays distinguishable
// from a manual pick that happens to match the OS right now.
const { mode: themeMode, resolved: resolvedTheme, cycleTheme } = useTheme()

// Plain words, not sun/moon glyphs: the UI is monospace throughout, and the
// astronomical symbols fall back to a substitute glyph in that font stack.
const themeIcon = computed(() => {
  if (themeMode.value === 'light') return 'light'
  if (themeMode.value === 'dark') return 'dark'
  return 'auto'
})

const themeTitle = computed(() => {
  if (themeMode.value === 'light') return 'Theme: light. Click for dark.'
  if (themeMode.value === 'dark') return 'Theme: dark. Click to follow your system setting.'
  return `Theme: follows your system (currently ${resolvedTheme.value}). Click for light.`
})
const showMenu = ref(false)
const showDifficulty = ref(false)
const showBalance = ref(false)
const savePath = ref('outpost3.o3save')
const availableSaves = ref<string[]>([])
const menuError = ref<string | null>(null)

async function onDifficultyChange(payload: {
  scalars: Record<string, number>
  menaceEnabled: boolean
  hazardsEnabled: boolean
  maintenanceEnabled: boolean
}) {
  if (!isTauri) return
  try {
    await setCustomDifficulty(
      payload.scalars,
      payload.menaceEnabled,
      payload.hazardsEnabled,
      payload.maintenanceEnabled,
    )
    // Refresh snapshot so any UI that reads world state sees the update.
    const snap = await fetchSnapshot()
    store.hydrate({ sol: snap.sol, month: snap.month, colonies: snap.colonies })
  } catch (e) {
    menuError.value = `Difficulty change failed: ${e instanceof Error ? e.message : String(e)}`
  }
}

async function openMenu(): Promise<void> {
  showMenu.value = true
  menuError.value = null
  if (isTauri) {
    try {
      availableSaves.value = await listSaves('.')
    } catch {
      availableSaves.value = []
    }
  }
}

function closeMenu(): void {
  showMenu.value = false
}

async function doSave(): Promise<void> {
  menuError.value = null
  try {
    await saveGame(savePath.value)
    menuError.value = `Saved to ${savePath.value}`
  } catch (e) {
    menuError.value = `Save failed: ${e instanceof Error ? e.message : String(e)}`
  }
}

async function doLoad(name: string): Promise<void> {
  menuError.value = null
  try {
    const snap = await loadGame(name)
    store.hydrate({ sol: snap.sol, month: snap.month, colonies: snap.colonies })
    showMenu.value = false
    if (route.path !== '/colony') router.push('/colony')
  } catch (e) {
    menuError.value = `Load failed: ${e instanceof Error ? e.message : String(e)}`
  }
}

async function goToMainMenu(): Promise<void> {
  if (isTauri) {
    await resetEngine()
  }
  gameStore.$reset?.()
  store.reset()
  showMenu.value = false
  router.push('/')
}

async function exitApp(): Promise<void> {
  if (isTauri) {
    await tauriExit()
  } else {
    window.close()
  }
}

// ─── Escape-to-back navigation (navigation rework #7 phase 1) ──────────────
//
// Route params (`:colonyId` etc.) are now the source of truth for drill-down
// depth, so `router.back()` naturally walks the stack — no manual bookkeeping
// needed. Known limitation, deferred to a later phase: this doesn't yet know
// about component-local modals (e.g. BuildingDetailsHud) — pressing Escape
// while one is open navigates back *and* leaves the modal open, rather than
// closing the modal first. Promoting that modal to a routed facility page
// (phase 2) removes the ambiguity entirely rather than requiring cross-
// component modal-state plumbing here.
/** True while focus is inside a form control — Escape there should let the
 * browser/input handle it (e.g. clear focus, close a native dropdown)
 * rather than navigate the whole app away and discard in-progress input. */
function isEditingFormField(): boolean {
  const el = document.activeElement
  if (!el) return false
  if (el instanceof HTMLElement && el.isContentEditable) return true
  return el.tagName === 'INPUT' || el.tagName === 'TEXTAREA' || el.tagName === 'SELECT'
}

function onKeydown(event: KeyboardEvent): void {
  if (event.key !== 'Escape') return
  if (route.path === '/') return
  if (isEditingFormField()) return
  // A direct deep-link load (or a fresh Tauri session) has no in-app
  // history to go back to — `history.length` is 1 in that case. Without
  // this guard, `router.back()` would navigate the whole window away from
  // the app (browser mode) or do nothing useful (Tauri).
  if (window.history.length <= 1) return
  router.back()
}

onMounted(() => window.addEventListener('keydown', onKeydown))
onUnmounted(() => window.removeEventListener('keydown', onKeydown))
</script>

<template>
  <div class="app">
    <header class="app-header" v-if="inGame">
      <button
        class="menu-btn"
        data-testid="app-menu-btn"
        aria-label="Open menu"
        @click="openMenu"
      >
        ☰
      </button>
      <h1>Outpost 3</h1>
      <nav class="header-nav">
        <RouterLink to="/system" class="nav-link">System</RouterLink>
        <RouterLink to="/planet" class="nav-link">Planet</RouterLink>
        <RouterLink to="/colonies" class="nav-link">Colonies</RouterLink>
        <RouterLink to="/tech" class="nav-link">Tech</RouterLink>
        <RouterLink to="/outposts" class="nav-link">Outposts</RouterLink>
        <RouterLink to="/installations" class="nav-link">Installations</RouterLink>
        <RouterLink to="/bodies" class="nav-link">Bodies</RouterLink>
        <RouterLink to="/buildings" class="nav-link">Buildings</RouterLink>
        <RouterLink to="/trade" class="nav-link">Trade</RouterLink>
      </nav>
      <button
        class="theme-btn"
        data-testid="theme-toggle"
        :title="themeTitle"
        :aria-label="themeTitle"
        @click="cycleTheme"
      >
        {{ themeIcon }}
      </button>
      <span
        v-if="!isTauri"
        class="connection-status"
        :data-status="store.connectionStatus"
        data-testid="connection-status"
      >{{ store.connectionStatus }}</span>
    </header>

    <SystemStatsBar v-if="inGame" />
    <AlertToast v-if="inGame" />

    <main class="app-main">
      <RouterView />
    </main>

    <TurnControlBar v-if="inGame" />

    <div v-if="showMenu" class="menu-backdrop" @click.self="closeMenu">
      <div class="menu-dialog" data-testid="app-menu-dialog">
        <h3>Menu</h3>

        <section class="menu-section">
          <h4>Save</h4>
          <div class="row">
            <input v-model="savePath" class="menu-input" placeholder="Save file name" />
            <button class="menu-action" @click="doSave">Save</button>
          </div>
        </section>

        <section class="menu-section">
          <h4>Load</h4>
          <div v-if="availableSaves.length === 0" class="hint">
            No save files found in the current directory.
          </div>
          <ul v-else class="save-list">
            <li v-for="name in availableSaves" :key="name">
              <button class="menu-action wide" @click="doLoad(name)">{{ name }}</button>
            </li>
          </ul>
        </section>

        <section class="menu-section">
          <div class="row">
            <button
              class="menu-action"
              data-testid="btn-difficulty"
              @click="showDifficulty = !showDifficulty"
            >{{ showDifficulty ? 'Hide Difficulty' : 'Difficulty…' }}</button>
          </div>
          <div v-if="showDifficulty" class="difficulty-slot">
            <CustomDifficultyPanel :live="true" @change="onDifficultyChange" />
          </div>
        </section>

        <section class="menu-section">
          <h4>Dev Tools</h4>
          <div class="row">
            <button
              class="menu-action"
              data-testid="btn-balance"
              @click="showBalance = !showBalance"
            >{{ showBalance ? 'Hide Balance' : 'Balance Tuning…' }}</button>
          </div>
          <p v-if="showBalance" class="hint">
            Tunes live simulation scalars — not a normal gameplay control. Changes take effect on
            the next sol.
          </p>
          <div v-if="showBalance" class="difficulty-slot">
            <BalanceTuningPanel />
          </div>
        </section>

        <section class="menu-section">
          <div class="row">
            <button class="menu-action" @click="goToMainMenu">Main Menu</button>
            <button class="menu-action danger" @click="exitApp">Exit to Desktop</button>
            <button class="menu-action" @click="closeMenu">Resume</button>
          </div>
        </section>

        <p v-if="menuError" class="menu-msg">{{ menuError }}</p>
      </div>
    </div>
  </div>
</template>

<style>
* { box-sizing: border-box; margin: 0; padding: 0; }
body { font-family: monospace; background: var(--bg); color: var(--text-bright); }

/*
 * App shell (UI-rework PR2/PR3): a fixed-height flex column that fills the
 * viewport and never lets the page (body) scroll. The header, system-stats
 * bar, and turn-control footer are auto-height; the main workspace takes the
 * rest (`flex: 1; min-height: 0`) and is the single internal scroll container
 * so an overflowing view scrolls itself instead of pushing a full-page
 * scrollbar. Flex (not a fixed-row grid) keeps `main` filling correctly even
 * when the bars are hidden (e.g. on the root menu route). Later PRs convert
 * individual views to fill without scrolling.
 */
.app {
  display: flex;
  flex-direction: column;
  height: 100dvh;
  overflow: hidden;
}

.app-header {
  display: flex;
  align-items: center;
  gap: 1rem;
  padding: 0.5rem 1rem;
  background: var(--surface-header);
  border-bottom: 1px solid var(--border);
}

.app-header h1 { font-size: 1.1rem; color: var(--accent); }

/* Pushed to the right edge, ahead of the connection pill. */
.theme-btn {
  margin-left: auto;
  background: transparent;
  border: 1px solid var(--border-strong);
  color: var(--accent);
  font-size: 0.75rem;
  line-height: 1;
  padding: 0.3rem 0.5rem;
  border-radius: 3px;
  cursor: pointer;
  font-family: monospace;
}
.theme-btn:hover { background: var(--surface-alt); }

.menu-btn {
  background: transparent;
  border: 1px solid var(--border-strong);
  color: var(--accent);
  font-size: 1rem;
  padding: 0.2rem 0.55rem;
  border-radius: 3px;
  cursor: pointer;
  font-family: monospace;
}
.menu-btn:hover { background: var(--surface-alt); }

.header-nav { display: flex; gap: 0.5rem; }
.nav-link {
  color: var(--text-muted);
  text-decoration: none;
  font-size: 0.8rem;
  padding: 0.2rem 0.55rem;
  border-radius: 3px;
  border: 1px solid transparent;
}
.nav-link:hover { color: var(--text); background: var(--surface-alt); }
.nav-link.router-link-active { color: var(--accent); border-color: var(--border-strong); background: var(--surface-alt); }

/* No `margin-left: auto` here — `.theme-btn` sits immediately before it and
   already pushes the pair to the right edge. Two auto margins would split
   the free space between them instead of grouping them. */
.connection-status[data-status="connected"] { color: var(--good); }
.connection-status[data-status="connecting"] { color: var(--good-dim); }
.connection-status[data-status="disconnected"] { color: var(--text-muted); }
.connection-status[data-status="error"] { color: var(--danger-dim); }

.app-main { flex: 1; min-height: 0; overflow: auto; padding: 1rem; }

.menu-backdrop {
  position: fixed;
  inset: 0;
  background: var(--overlay-backdrop);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 200;
}
.menu-dialog {
  background: var(--surface-2);
  border: 1px solid var(--border-strong);
  border-radius: 6px;
  padding: 1.5rem;
  min-width: 320px;
  max-width: 560px;
  max-height: calc(100vh - 4rem);
  overflow-y: auto;
  display: flex;
  flex-direction: column;
  gap: 1rem;
}
.difficulty-slot { margin-top: 0.4rem; }
.menu-dialog h3 { color: var(--accent); }
.menu-dialog h4 { color: var(--text-dim); font-size: 0.78rem; letter-spacing: 0.06em; text-transform: uppercase; }
.menu-section { display: flex; flex-direction: column; gap: 0.4rem; }
.row { display: flex; gap: 0.5rem; flex-wrap: wrap; }
.menu-input {
  flex: 1;
  min-width: 160px;
  background: var(--surface-3);
  border: 1px solid var(--border);
  border-radius: 3px;
  color: var(--text-bright);
  padding: 0.35rem 0.5rem;
  font-family: monospace;
  font-size: 0.85rem;
}
.menu-action {
  background: var(--surface-btn);
  border: 1px solid var(--border-strong);
  border-radius: 3px;
  color: var(--text);
  padding: 0.4rem 0.75rem;
  font-family: monospace;
  font-size: 0.82rem;
  cursor: pointer;
}
.menu-action:hover { background: var(--surface-btn-hover); }
.menu-action.wide { width: 100%; text-align: left; }
.menu-action.danger { border-color: var(--danger-dim); color: var(--danger-dim); }
.save-list { list-style: none; display: flex; flex-direction: column; gap: 0.25rem; }
.hint { font-size: 0.75rem; color: var(--text-dim); font-style: italic; }
.menu-msg { color: var(--good-dim); font-size: 0.8rem; }
</style>
