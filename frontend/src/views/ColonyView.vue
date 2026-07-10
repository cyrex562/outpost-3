<script setup lang="ts">
import { useWorldStore } from '@/stores/worldStore'

const store = useWorldStore()
</script>

<template>
  <div class="colony-view">
    <h2>Colonies</h2>

    <div v-if="store.colonies.length === 0" class="empty-state" data-testid="no-colonies">
      No colonies founded yet.
    </div>

    <ul v-else class="colony-list" data-testid="colony-list">
      <li
        v-for="colony in store.colonies"
        :key="colony.id"
        class="colony-card"
        :data-testid="`colony-${colony.id}`"
      >
        <div class="colony-name">{{ colony.name }}</div>
        <div class="colony-stats">
          <span>Pop: {{ colony.population.toFixed(0) }}</span>
          <span>Stability: {{ (colony.stability * 100).toFixed(0) }}%</span>
          <span>Labour: {{ colony.available_labour }}</span>
        </div>
        <div v-if="colony.buildings.length > 0" class="colony-buildings">
          Buildings: {{ colony.buildings.join(', ') }}
        </div>
      </li>
    </ul>

    <div v-if="store.notifications.length > 0" class="notifications" data-testid="notifications">
      <h3>Notifications</h3>
      <ul>
        <li
          v-for="n in store.notifications"
          :key="n.id"
          :class="`notification tier-${n.tier}`"
        >
          {{ n.message }}
        </li>
      </ul>
    </div>

    <div class="research-total" data-testid="research-total">
      System research: {{ store.researchTotal.toFixed(1) }} RP
    </div>
  </div>
</template>

<style scoped>
.colony-view { max-width: 800px; }
h2 { margin-bottom: 1rem; color: #8cf; }
h3 { margin: 1rem 0 0.5rem; color: #aaa; font-size: 0.9rem; }

.empty-state { color: #666; font-style: italic; margin: 1rem 0; }

.colony-list { list-style: none; display: flex; flex-direction: column; gap: 0.75rem; }

.colony-card {
  background: #151520;
  border: 1px solid #334;
  border-radius: 4px;
  padding: 0.75rem 1rem;
}

.colony-name { font-size: 1rem; font-weight: bold; color: #dde; margin-bottom: 0.25rem; }
.colony-stats { display: flex; gap: 1.5rem; font-size: 0.8rem; color: #889; }
.colony-buildings { font-size: 0.75rem; color: #677; margin-top: 0.25rem; }

.notifications { margin-top: 1.5rem; }
.notifications ul { list-style: none; }
.notification { padding: 0.25rem 0.5rem; font-size: 0.8rem; border-left: 3px solid #555; margin-bottom: 0.25rem; }
.notification.tier-notable { border-color: #c84; color: #ca8; }
.notification.tier-urgent { border-color: #c44; color: #c66; }
.notification.tier-blocking { border-color: #f44; color: #f66; }
.notification.tier-ambient { border-color: #446; color: #778; }

.research-total { margin-top: 1rem; font-size: 0.8rem; color: #6a8; }
</style>
