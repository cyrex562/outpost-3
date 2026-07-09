<script setup lang="ts">
import { onMounted } from "vue";
import { RouterView } from "vue-router";
import { useWebSocket } from "./composables/useWebSocket";
import { useConfigStore } from "./stores/config";
import { useWorldStore } from "./stores/world";

const { connect } = useWebSocket();
const worldStore = useWorldStore();
const configStore = useConfigStore();

onMounted(async () => {
    // Load runtime config (admin mode, launch mode) and the active world.
    await Promise.all([configStore.load(), worldStore.fetchCurrent()]);

    // Connect WebSocket - it will handle re-fetching character data if world is active
    connect();
});
</script>

<template>
    <RouterView />
</template>
