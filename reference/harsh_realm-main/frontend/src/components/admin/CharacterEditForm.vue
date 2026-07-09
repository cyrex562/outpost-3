<script setup lang="ts">
import { ref } from "vue";
import { useAdminStore } from "../../stores/admin";
import type { Character, RecalcResult } from "../../types/api";

const props = defineProps<{ editData: Character }>();
const emit = defineEmits<{ saved: []; deleted: [] }>();

const store = useAdminStore();
const recalcPreview = ref<RecalcResult | null>(null);
const classOptions = ["warrior", "expert", "adventurer"];
const attrNames = ["STR", "DEX", "CON", "INT", "WIS", "CHA"];

function attrMod(val: number): string {
    const mod = Math.floor((val - 10) / 2);
    return mod >= 0 ? `+${mod}` : `${mod}`;
}

function parseEquipmentJson(event: Event) {
    try {
        props.editData.data.equipment = JSON.parse(
            (event.target as HTMLTextAreaElement).value,
        );
    } catch {
        /* ignore parse errors while typing */
    }
}

function parsePersonalityJson(event: Event) {
    try {
        props.editData.data.personality = JSON.parse(
            (event.target as HTMLTextAreaElement).value,
        );
    } catch {
        /* ignore parse errors while typing */
    }
}

async function handlePreviewRecalc() {
    const d = props.editData.data;
    recalcPreview.value = await store.previewRecalc({
        character_class: d.character_class || "warrior",
        level: d.level || 1,
        attributes: d.attributes || {},
        equipment: d.equipment || [],
    });
}

async function handleSave() {
    const { id, ...rest } = props.editData;
    await store.updateCharacter(id, rest);
    emit("saved");
}

async function handleDelete(hard: boolean) {
    const confirmed = window.confirm(
        hard ? "Permanently delete this entity?" : "Mark this entity as dead?",
    );
    if (!confirmed) return;
    await store.deleteCharacter(props.editData.id, hard);
    emit("deleted");
}
</script>

<template>
    <div class="flex-1 overflow-y-auto">
                <div class="grid grid-cols-2 gap-3 mb-3">
                    <!-- Basic info -->
                    <div>
                        <label class="block text-xs text-gray-500 mb-0.5"
                            >Name</label
                        >
                        <input
                            v-model="editData.name"
                            class="w-full bg-gray-800 border border-gray-700 text-gray-200 rounded px-2 py-1 text-xs focus:outline-none focus:border-gray-500"
                        />
                    </div>
                    <div>
                        <label class="block text-xs text-gray-500 mb-0.5"
                            >Entity Type</label
                        >
                        <input
                            v-model="editData.entity_type"
                            class="w-full bg-gray-800 border border-gray-700 text-gray-200 rounded px-2 py-1 text-xs focus:outline-none focus:border-gray-500"
                        />
                    </div>
                    <div>
                        <label class="block text-xs text-gray-500 mb-0.5"
                            >Class</label
                        >
                        <select
                            v-model="editData.data.character_class"
                            class="w-full bg-gray-800 border border-gray-700 text-gray-200 rounded px-2 py-1 text-xs focus:outline-none focus:border-gray-500"
                        >
                            <option value="">-- none --</option>
                            <option
                                v-for="c in classOptions"
                                :key="c"
                                :value="c"
                            >
                                {{ c }}
                            </option>
                        </select>
                    </div>
                    <div>
                        <label class="block text-xs text-gray-500 mb-0.5"
                            >Level</label
                        >
                        <input
                            v-model.number="editData.data.level"
                            type="number"
                            min="1"
                            class="w-full bg-gray-800 border border-gray-700 text-gray-200 rounded px-2 py-1 text-xs focus:outline-none focus:border-gray-500"
                        />
                    </div>
                </div>

                <!-- Attributes -->
                <div class="mb-3" v-if="editData.data.attributes">
                    <label class="block text-xs text-gray-500 mb-1"
                        >Attributes</label
                    >
                    <div class="grid grid-cols-6 gap-2">
                        <div
                            v-for="attr in attrNames"
                            :key="attr"
                            class="text-center"
                        >
                            <div class="text-xs text-gray-500 mb-0.5">
                                {{ attr }}
                            </div>
                            <input
                                v-model.number="editData.data.attributes[attr]"
                                type="number"
                                class="w-full bg-gray-800 border border-gray-700 text-gray-200 rounded px-1 py-0.5 text-xs text-center focus:outline-none focus:border-gray-500"
                            />
                            <div class="text-xs text-gray-600 mt-0.5">
                                {{
                                    editData.data.attributes[attr]
                                        ? attrMod(
                                              editData.data.attributes[attr],
                                          )
                                        : ""
                                }}
                            </div>
                        </div>
                    </div>
                </div>

                <!-- HP / AC / Combat -->
                <div class="grid grid-cols-4 gap-3 mb-3">
                    <div>
                        <label class="block text-xs text-gray-500 mb-0.5"
                            >Current HP</label
                        >
                        <input
                            v-model.number="editData.data.hp"
                            type="number"
                            class="w-full bg-gray-800 border border-gray-700 text-gray-200 rounded px-2 py-1 text-xs focus:outline-none focus:border-gray-500"
                        />
                    </div>
                    <div>
                        <label class="block text-xs text-gray-500 mb-0.5"
                            >Max HP</label
                        >
                        <input
                            :value="editData.data.max_hp"
                            disabled
                            class="w-full bg-gray-900 border border-gray-700 text-gray-500 rounded px-2 py-1 text-xs"
                        />
                    </div>
                    <div>
                        <label class="block text-xs text-gray-500 mb-0.5"
                            >AC</label
                        >
                        <input
                            :value="editData.data.ac"
                            disabled
                            class="w-full bg-gray-900 border border-gray-700 text-gray-500 rounded px-2 py-1 text-xs"
                        />
                    </div>
                    <div>
                        <label class="block text-xs text-gray-500 mb-0.5"
                            >Attack Bonus</label
                        >
                        <input
                            :value="editData.data.attack_bonus"
                            disabled
                            class="w-full bg-gray-900 border border-gray-700 text-gray-500 rounded px-2 py-1 text-xs"
                        />
                    </div>
                </div>

                <!-- Saves -->
                <div class="grid grid-cols-3 gap-3 mb-3">
                    <div>
                        <label class="block text-xs text-gray-500 mb-0.5"
                            >Physical Save</label
                        >
                        <input
                            :value="editData.data.physical_save"
                            disabled
                            class="w-full bg-gray-900 border border-gray-700 text-gray-500 rounded px-2 py-1 text-xs"
                        />
                    </div>
                    <div>
                        <label class="block text-xs text-gray-500 mb-0.5"
                            >Evasion Save</label
                        >
                        <input
                            :value="editData.data.evasion_save"
                            disabled
                            class="w-full bg-gray-900 border border-gray-700 text-gray-500 rounded px-2 py-1 text-xs"
                        />
                    </div>
                    <div>
                        <label class="block text-xs text-gray-500 mb-0.5"
                            >Mental Save</label
                        >
                        <input
                            :value="editData.data.mental_save"
                            disabled
                            class="w-full bg-gray-900 border border-gray-700 text-gray-500 rounded px-2 py-1 text-xs"
                        />
                    </div>
                </div>

                <!-- Location -->
                <div class="grid grid-cols-2 gap-3 mb-3">
                    <div>
                        <label class="block text-xs text-gray-500 mb-0.5"
                            >Location Q</label
                        >
                        <input
                            v-model.number="editData.location_q"
                            type="number"
                            class="w-full bg-gray-800 border border-gray-700 text-gray-200 rounded px-2 py-1 text-xs focus:outline-none focus:border-gray-500"
                        />
                    </div>
                    <div>
                        <label class="block text-xs text-gray-500 mb-0.5"
                            >Location R</label
                        >
                        <input
                            v-model.number="editData.location_r"
                            type="number"
                            class="w-full bg-gray-800 border border-gray-700 text-gray-200 rounded px-2 py-1 text-xs focus:outline-none focus:border-gray-500"
                        />
                    </div>
                </div>

                <div class="mb-3">
                    <label class="block text-xs text-gray-500 mb-0.5"
                        >Alive</label
                    >
                    <input
                        v-model="editData.alive"
                        type="checkbox"
                        class="accent-blue-500"
                    />
                </div>

                <!-- Skills -->
                <div class="mb-3" v-if="editData.data.skills">
                    <label class="block text-xs text-gray-500 mb-1"
                        >Skills</label
                    >
                    <div class="grid grid-cols-3 gap-1">
                        <div
                            v-for="(_val, key) in editData.data.skills"
                            :key="key"
                            class="flex items-center gap-1"
                        >
                            <span class="text-xs text-gray-400 w-20 truncate">{{
                                key
                            }}</span>
                            <input
                                v-model.number="editData.data.skills[key]"
                                type="number"
                                class="w-12 bg-gray-800 border border-gray-700 text-gray-200 rounded px-1 py-0.5 text-xs focus:outline-none focus:border-gray-500"
                            />
                        </div>
                    </div>
                </div>

                <!-- Equipment (JSON textarea) -->
                <div class="mb-3">
                    <label class="block text-xs text-gray-500 mb-1"
                        >Equipment (JSON)</label
                    >
                    <textarea
                        :value="
                            JSON.stringify(
                                editData.data.equipment || [],
                                null,
                                2,
                            )
                        "
                        @input="parseEquipmentJson($event)"
                        rows="4"
                        class="w-full bg-gray-800 border border-gray-700 text-gray-200 rounded px-2 py-1 text-xs font-mono focus:outline-none focus:border-gray-500"
                    ></textarea>
                </div>

                <!-- UNE Personality (if present) -->
                <div class="mb-3" v-if="editData.data.personality">
                    <label class="block text-xs text-gray-500 mb-1"
                        >Personality (UNE)</label
                    >
                    <textarea
                        :value="
                            JSON.stringify(editData.data.personality, null, 2)
                        "
                        @input="parsePersonalityJson($event)"
                        rows="3"
                        class="w-full bg-gray-800 border border-gray-700 text-gray-200 rounded px-2 py-1 text-xs font-mono focus:outline-none focus:border-gray-500"
                    ></textarea>
                </div>

                <!-- Preview Recalc -->
                <div class="mb-3" v-if="editData.data.character_class">
                    <button
                        class="px-3 py-1 text-xs rounded border border-purple-600 text-purple-400 hover:bg-purple-900/30 mr-2"
                        @click="handlePreviewRecalc"
                    >
                        Preview Recalc
                    </button>
                    <div
                        v-if="recalcPreview"
                        class="mt-2 p-2 bg-gray-900 border border-gray-700 rounded text-xs text-gray-300 font-mono"
                    >
                        <div>
                            Max HP: {{ recalcPreview.max_hp }} | AC:
                            {{ recalcPreview.ac }} | AB:
                            {{ recalcPreview.attack_bonus }}
                        </div>
                        <div>
                            Melee: {{ recalcPreview.melee_attack }} | Ranged:
                            {{ recalcPreview.ranged_attack }}
                        </div>
                        <div>
                            Saves: Phys {{ recalcPreview.physical_save }} | Eva
                            {{ recalcPreview.evasion_save }} | Ment
                            {{ recalcPreview.mental_save }}
                        </div>
                        <div>
                            Attr Mods:
                            {{ JSON.stringify(recalcPreview.attr_mods) }}
                        </div>
                    </div>
                </div>

                <!-- Action buttons -->
                <div class="flex gap-2 mt-4 border-t border-gray-800 pt-3">
                    <button
                        class="px-3 py-1 text-xs rounded border border-green-700 text-green-400 hover:bg-green-900/30"
                        @click="handleSave"
                    >
                        Save
                    </button>
                    <button
                        class="px-3 py-1 text-xs rounded border border-yellow-700 text-yellow-400 hover:bg-yellow-900/30"
                        @click="handleDelete(false)"
                    >
                        Kill (Soft Delete)
                    </button>
                    <button
                        class="px-3 py-1 text-xs rounded border border-red-700 text-red-400 hover:bg-red-900/30"
                        @click="handleDelete(true)"
                    >
                        Delete (Hard)
                    </button>
                </div>
    </div>
</template>
