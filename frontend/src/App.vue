<script setup lang="ts">
import { ref, computed } from 'vue'
import { useRouter, useRoute } from 'vue-router'
import { useGameSocket } from '@/composables/useGameSocket'
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

useGameSocket()

const router = useRouter()
const route = useRoute()
const store = useWorldStore()
const gameStore = useGameStore()

const inGame = computed(() => route.path !== '/' && route.path !== '/menu')
const showMenu = ref(false)
const showDifficulty = ref(false)
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
        <RouterLink to="/colony" class="nav-link">Colony</RouterLink>
        <RouterLink to="/tech" class="nav-link">Tech</RouterLink>
      </nav>
      <span
        v-if="!isTauri"
        class="connection-status"
        :data-status="store.connectionStatus"
        data-testid="connection-status"
      >{{ store.connectionStatus }}</span>
      <div class="time-display" data-testid="time-display">
        Sol {{ store.sol }} · Month {{ store.month }}
      </div>
    </header>

    <main class="app-main">
      <RouterView />
    </main>

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
body { font-family: monospace; background: #0a0a0f; color: #cdd; }

.app { display: flex; flex-direction: column; min-height: 100vh; }

.app-header {
  display: flex;
  align-items: center;
  gap: 1rem;
  padding: 0.5rem 1rem;
  background: #111;
  border-bottom: 1px solid #333;
}

.app-header h1 { font-size: 1.1rem; color: #8cf; }

.menu-btn {
  background: transparent;
  border: 1px solid #446;
  color: #8cf;
  font-size: 1rem;
  padding: 0.2rem 0.55rem;
  border-radius: 3px;
  cursor: pointer;
  font-family: monospace;
}
.menu-btn:hover { background: #1a1a2a; }

.header-nav { display: flex; gap: 0.5rem; }
.nav-link {
  color: #778;
  text-decoration: none;
  font-size: 0.8rem;
  padding: 0.2rem 0.55rem;
  border-radius: 3px;
  border: 1px solid transparent;
}
.nav-link:hover { color: #aac; background: #1a1a2a; }
.nav-link.router-link-active { color: #8cf; border-color: #446; background: #1a1a2a; }

.connection-status[data-status="connected"] { color: #4c4; }
.connection-status[data-status="connecting"] { color: #cc4; }
.connection-status[data-status="disconnected"] { color: #888; }
.connection-status[data-status="error"] { color: #c44; }

.time-display { margin-left: auto; color: #8a8; font-size: 0.85rem; }

.app-main { flex: 1; padding: 1rem; }

.menu-backdrop {
  position: fixed;
  inset: 0;
  background: rgba(0, 0, 0, 0.7);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 200;
}
.menu-dialog {
  background: #14141e;
  border: 1px solid #446;
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
.menu-dialog h3 { color: #8cf; }
.menu-dialog h4 { color: #668; font-size: 0.78rem; letter-spacing: 0.06em; text-transform: uppercase; }
.menu-section { display: flex; flex-direction: column; gap: 0.4rem; }
.row { display: flex; gap: 0.5rem; flex-wrap: wrap; }
.menu-input {
  flex: 1;
  min-width: 160px;
  background: #0d0d15;
  border: 1px solid #334;
  border-radius: 3px;
  color: #cdd;
  padding: 0.35rem 0.5rem;
  font-family: monospace;
  font-size: 0.85rem;
}
.menu-action {
  background: #1a1a28;
  border: 1px solid #446;
  border-radius: 3px;
  color: #aac;
  padding: 0.4rem 0.75rem;
  font-family: monospace;
  font-size: 0.82rem;
  cursor: pointer;
}
.menu-action:hover { background: #22223a; }
.menu-action.wide { width: 100%; text-align: left; }
.menu-action.danger { border-color: #a55; color: #c88; }
.save-list { list-style: none; display: flex; flex-direction: column; gap: 0.25rem; }
.hint { font-size: 0.75rem; color: #667; font-style: italic; }
.menu-msg { color: #ac6; font-size: 0.8rem; }
</style>
