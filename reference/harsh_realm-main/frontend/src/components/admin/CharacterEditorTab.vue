<script setup lang="ts">
import { ref, computed, onMounted } from "vue";
import { useAdminStore } from "../../stores/admin";
import type { Character } from "../../types/api";
import ImportExportBar from "./ImportExportBar.vue";
import CharacterEditForm from "./CharacterEditForm.vue";

const store = useAdminStore();
const filter = ref<string>("all");
const selectedId = ref<string | null>(null);
const editData = ref<Character | null>(null);
const statusMsg = ref("");

const showCreateForm = ref(false);
const newName = ref("");
const newType = ref("npc");

const filteredCharacters = computed(() => {
    const list = store.characters;
    if (filter.value === "all") return list;
    if (filter.value === "pc")
        return list.filter((c) => c.entity_type === "character");
    if (filter.value === "npc")
        return list.filter((c) => c.entity_type === "npc" && c.alive);
    if (filter.value === "dead") return list.filter((c) => !c.alive);
    return list;
});

async function loadData() {
    await store.loadCharacters();
}

async function selectCharacter(id: string) {
    selectedId.value = id;
    const char = await store.getCharacter(id);
    editData.value = JSON.parse(JSON.stringify(char));
}

async function handleCreate() {
    if (!newName.value.trim()) return;
    await store.createCharacter({
        name: newName.value.trim(),
        entity_type: newType.value,
        location_q: 0,
        location_r: 0,
        data: {},
    });
    showCreateForm.value = false;
    newName.value = "";
    await store.loadCharacters();
    flash("Character created.");
}

function onSaved() {
    flash("Character saved.");
    void store.loadCharacters();
}

function onDeleted() {
    selectedId.value = null;
    editData.value = null;
    void store.loadCharacters();
    flash("Character deleted.");
}

function flash(msg: string) {
    statusMsg.value = msg;
    setTimeout(() => {
        statusMsg.value = "";
    }, 3000);
}

onMounted(loadData);
</script>

<template>
    <div>
        <ImportExportBar table="entities" @imported="loadData()" />
        <div class="flex gap-4 h-full">
            <!-- Left: character list -->
            <div class="w-64 shrink-0 flex flex-col">
                <div class="flex items-center justify-between mb-2">
                    <h2
                        class="text-sm font-bold text-gray-300 uppercase tracking-wide"
                    >
                        Entities
                    </h2>
                    <span v-if="statusMsg" class="text-xs text-green-400">{{
                        statusMsg
                    }}</span>
                </div>

                <!-- Filter buttons -->
                <div class="flex gap-1 mb-2">
                    <button
                        v-for="f in ['all', 'pc', 'npc', 'dead']"
                        :key="f"
                        class="px-2 py-0.5 text-xs rounded border transition-colors"
                        :class="
                            filter === f
                                ? 'border-blue-500 text-blue-400 bg-blue-900/20'
                                : 'border-gray-700 text-gray-500 hover:text-gray-300'
                        "
                        @click="filter = f"
                    >
                        {{ f.toUpperCase() }}
                    </button>
                </div>

                <!-- Create button -->
                <button
                    class="px-2 py-1 text-xs rounded border border-green-700 text-green-400 hover:bg-green-900/30 mb-2"
                    @click="showCreateForm = !showCreateForm"
                >
                    + New Entity
                </button>

                <div
                    v-if="showCreateForm"
                    class="mb-2 p-2 bg-gray-900 border border-gray-700 rounded"
                >
                    <input
                        v-model="newName"
                        class="w-full bg-gray-800 border border-gray-700 text-gray-200 rounded px-1 py-0.5 text-xs mb-1 focus:outline-none focus:border-gray-500"
                        placeholder="Name"
                    />
                    <select
                        v-model="newType"
                        class="w-full bg-gray-800 border border-gray-700 text-gray-200 rounded px-1 py-0.5 text-xs mb-1 focus:outline-none focus:border-gray-500"
                    >
                        <option value="player">Player</option>
                        <option value="npc">NPC</option>
                        <option value="creature">Creature</option>
                    </select>
                    <button
                        class="px-2 py-0.5 text-xs rounded border border-green-700 text-green-400 hover:bg-green-900/30"
                        @click="handleCreate"
                    >
                        Create
                    </button>
                </div>

                <!-- Character list -->
                <div class="flex-1 overflow-y-auto">
                    <div
                        v-for="c in filteredCharacters"
                        :key="c.id"
                        class="px-2 py-1 text-xs cursor-pointer rounded mb-0.5 flex items-center justify-between"
                        :class="
                            selectedId === c.id
                                ? 'bg-blue-900/30 text-blue-300 border border-blue-700'
                                : 'text-gray-400 hover:bg-gray-900 hover:text-gray-200 border border-transparent'
                        "
                        @click="selectCharacter(c.id)"
                    >
                        <span class="truncate">{{ c.name }}</span>
                        <span class="text-gray-600 ml-1">{{
                            c.entity_type === "character" ? "PC" : c.entity_type
                        }}</span>
                    </div>
                    <p
                        v-if="filteredCharacters.length === 0"
                        class="text-gray-600 text-xs mt-2"
                    >
                        No entities found.
                    </p>
                </div>
            </div>

            <!-- Right: edit form -->
            <CharacterEditForm
                v-if="editData"
                :edit-data="editData"
                @saved="onSaved"
                @deleted="onDeleted"
            />

            <!-- No selection -->
            <div
                v-else
                class="flex-1 flex items-center justify-center text-gray-600 text-sm"
            >
                Select an entity from the list to edit.
            </div>
        </div>
    </div>
</template>
