<script setup lang="ts">
import { ref, computed, onMounted, onUpdated, nextTick } from "vue";
import { useAdminStore } from "../../stores/admin";

const store = useAdminStore();

/** Read a string field from a loosely-typed event record. */
function eventStr(e: Record<string, unknown>, key: string): string {
    const value = e[key];
    return typeof value === "string" ? value : "";
}

/** The event's type, falling back to the legacy ``type`` field. */
function eventType(e: Record<string, unknown>): string {
    return eventStr(e, "event_type") || eventStr(e, "type");
}
const logContainer = ref<HTMLElement | null>(null);
const autoScroll = ref(true);

function scrollToBottom() {
    if (autoScroll.value && logContainer.value) {
        logContainer.value.scrollTop = logContainer.value.scrollHeight;
    }
}

onUpdated(() => {
    nextTick(scrollToBottom);
});

onMounted(() => {
    scrollToBottom();
});

function formatTime(iso: string) {
    return new Date(iso).toLocaleTimeString([], {
        hour: "2-digit",
        minute: "2-digit",
        second: "2-digit",
        hour12: false,
    });
}

function clearLog() {
    store.eventLog = [];
}

const filterText = ref("");

const filteredEvents = computed(() => {
    if (!filterText.value) return store.eventLog;
    const search = filterText.value.toLowerCase();
    return store.eventLog.filter(
        (e) =>
            eventType(e).toLowerCase().includes(search) ||
            JSON.stringify(e.data ?? e.text ?? "")
                .toLowerCase()
                .includes(search),
    );
});
</script>

<template>
    <div
        class="flex flex-col h-full overflow-hidden border border-gray-800 rounded bg-gray-950 font-mono text-[11px]"
    >
        <!-- Header -->
        <div
            class="flex items-center justify-between p-2 bg-gray-900 border-b border-gray-800 shrink-0"
        >
            <div class="flex items-center gap-4">
                <h3 class="text-gray-400 font-bold uppercase tracking-wider">
                    Live Event Feed
                </h3>
                <div class="flex items-center gap-1.5 text-gray-500">
                    <input
                        type="checkbox"
                        v-model="autoScroll"
                        id="autoscroll"
                        class="bg-gray-800 border-gray-700 rounded"
                    />
                    <label for="autoscroll" class="cursor-pointer select-none"
                        >Auto-scroll</label
                    >
                </div>
                <div class="relative">
                    <input
                        v-model="filterText"
                        placeholder="Filter..."
                        class="bg-gray-800 border border-gray-700 text-gray-300 rounded px-2 py-0.5 w-32 focus:outline-none focus:border-gray-500"
                    />
                </div>
            </div>
            <button
                @click="clearLog"
                class="text-gray-500 hover:text-red-400 transition-colors"
            >
                Clear
            </button>
        </div>

        <!-- Feed -->
        <div
            ref="logContainer"
            class="flex-1 overflow-y-auto p-2 space-y-0.5 scrollbar-thin scrollbar-thumb-gray-800"
        >
            <div
                v-if="filteredEvents.length === 0"
                class="text-gray-700 italic text-center mt-8"
            >
                No events captured yet.
            </div>

            <div
                v-for="(ev, idx) in filteredEvents"
                :key="idx"
                class="flex gap-3 hover:bg-gray-900/50 group py-0.5"
            >
                <span class="text-gray-600 shrink-0 select-none">{{
                    formatTime(eventStr(ev, "timestamp"))
                }}</span>

                <div
                    class="flex-1 flex flex-wrap items-baseline gap-x-3 overflow-hidden"
                >
                    <span
                        class="font-bold shrink-0"
                        :class="{
                            'text-blue-400':
                                eventType(ev).startsWith('exploration'),
                            'text-red-400': eventType(ev).startsWith('combat'),
                            'text-amber-400':
                                eventType(ev).startsWith('shopping'),
                            'text-sky-400': eventType(ev).startsWith('social'),
                            'text-purple-400':
                                eventType(ev).startsWith('character'),
                            'text-emerald-400':
                                eventType(ev).startsWith('oracle'),
                            'text-gray-400': !eventType(ev),
                        }"
                    >
                        {{ eventType(ev) }}
                    </span>

                    <span
                        v-if="ev.tick !== undefined"
                        class="text-gray-600 shrink-0"
                        >[T:{{ ev.tick }}]</span
                    >

                    <span class="text-gray-300 break-words overflow-hidden">
                        <template v-if="ev.type === 'narration'">{{
                            ev.text
                        }}</template>
                        <template v-else-if="ev.type === 'player_input'"
                            >> {{ ev.text }}</template
                        >
                        <template v-else>{{ ev.data }}</template>
                    </span>
                </div>
            </div>
        </div>
    </div>
</template>

<style scoped>
.scrollbar-thin::-webkit-scrollbar {
    width: 6px;
}
.scrollbar-thin::-webkit-scrollbar-track {
    background: transparent;
}
.scrollbar-thin::-webkit-scrollbar-thumb {
    background: #1f2937;
    border-radius: 3px;
}
.scrollbar-thin::-webkit-scrollbar-thumb:hover {
    background: #374151;
}
</style>
