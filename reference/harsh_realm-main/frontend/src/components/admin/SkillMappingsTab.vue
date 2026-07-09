<script setup lang="ts">
import { ref, watch, onMounted } from "vue";
import { useAdminStore } from "../../stores/admin";
import type { SkillMapping } from "../../types/api";
import ImportExportBar from "./ImportExportBar.vue";

const store = useAdminStore();
const editSkills = ref<SkillMapping[]>([]);
const attributes = ["STR", "DEX", "CON", "INT", "WIS", "CHA"];

const newSkill = ref<SkillMapping>({
    verb: "",
    skill: "",
    attribute: "STR",
    base_difficulty: 8,
    opposed: false,
    description: "",
    notes: "",
});
const showAddSkill = ref(false);

function sync() {
    editSkills.value = store.skillMappings.map((s) => ({ ...s }));
}
watch(() => store.skillMappings, sync, { deep: true });
onMounted(sync);

async function handleSaveSkill(idx: number) {
    await store.saveSkillMapping(editSkills.value[idx]);
}
async function handleResetSkill(verb: string) {
    await store.resetSkillMapping(verb);
}
async function handleAddSkill() {
    if (!newSkill.value.verb.trim()) return;
    await store.saveSkillMapping(newSkill.value);
    newSkill.value = {
        verb: "",
        skill: "",
        attribute: "STR",
        base_difficulty: 8,
        opposed: false,
        description: "",
        notes: "",
    };
    showAddSkill.value = false;
}
async function handleDeleteSkill(verb: string) {
    await store.deleteSkillMapping(verb);
}
</script>

<template>
    <div>
                <div class="mb-4 text-sm text-gray-400 max-w-4xl">
                    <p class="mb-2">
                        <strong class="text-gray-200">Skill Mappings</strong>
                        define how player verbs (e.g.,
                        <code class="text-blue-400">sneak</code>,
                        <code class="text-blue-400">bribe</code>) map to
                        underlying character skills and attributes.
                    </p>
                    <ul class="grid grid-cols-2 gap-x-6 gap-y-1 list-disc pl-5">
                        <li>
                            <strong>Verb:</strong> The exact word the player
                            types to trigger the check.
                        </li>
                        <li>
                            <strong>Skill:</strong> The XWN skill used (e.g.,
                            <code class="text-gray-300">sneak</code>,
                            <code class="text-gray-300">talk</code>).
                        </li>
                        <li>
                            <strong>Attribute:</strong> The ability modifier
                            added to the 2d6 roll.
                        </li>
                        <li>
                            <strong>Difficulty:</strong> The target number to
                            meet or beat (e.g., 6 is Easy, 10 is Hard).
                        </li>
                        <li>
                            <strong>Opposed:</strong> If checked, the difficulty
                            is contested by a target's skill check.
                        </li>
                    </ul>
                </div>

                <!-- Common Skills Datalist -->
                <datalist id="skill-list">
                    <option value="administer" />
                    <option value="connect" />
                    <option value="convince" />
                    <option value="craft" />
                    <option value="exert" />
                    <option value="fix" />
                    <option value="heal" />
                    <option value="know" />
                    <option value="lead" />
                    <option value="notice" />
                    <option value="perform" />
                    <option value="pray" />
                    <option value="punch" />
                    <option value="ride" />
                    <option value="shoot" />
                    <option value="sneak" />
                    <option value="stab" />
                    <option value="survive" />
                    <option value="talk" />
                    <option value="trade" />
                    <option value="work" />
                </datalist>

                <div class="flex items-center gap-2 mb-2">
                    <ImportExportBar
                        table="skill_mappings"
                        @imported="store.loadAllData()"
                    />
                    <button
                        data-testid="skill-add-toggle"
                        class="px-3 py-1 text-xs rounded border border-green-700 text-green-400 hover:bg-green-900/30"
                        @click="showAddSkill = !showAddSkill"
                    >
                        {{ showAddSkill ? "Cancel" : "+ Add Mapping" }}
                    </button>
                </div>
                <table class="w-full text-sm">
                    <thead>
                        <tr
                            class="text-left text-gray-500 border-b border-gray-800"
                        >
                            <th class="px-2 py-1">Verb</th>
                            <th class="px-2 py-1">Skill</th>
                            <th class="px-2 py-1">Attribute</th>
                            <th class="px-2 py-1">Difficulty</th>
                            <th class="px-2 py-1">Opposed</th>
                            <th class="px-2 py-1">Description</th>
                            <th class="px-2 py-1">Actions</th>
                        </tr>
                    </thead>
                    <tbody>
                        <!-- Inline add row -->
                        <tr
                            v-if="showAddSkill"
                            class="border-b border-green-800/40 bg-green-950/20"
                        >
                            <td class="px-2 py-1">
                                <input
                                    v-model="newSkill.verb"
                                    data-testid="new-skill-verb"
                                    placeholder="verb"
                                    class="bg-gray-800 border border-green-700 text-gray-200 rounded px-1 py-0.5 w-24 text-xs focus:outline-none focus:border-green-500"
                                    @keydown.enter="handleAddSkill"
                                />
                            </td>
                            <td class="px-2 py-1">
                                <input
                                    v-model="newSkill.skill"
                                    placeholder="skill"
                                    list="skill-list"
                                    class="bg-gray-800 border border-green-700 text-gray-200 rounded px-1 py-0.5 w-24 text-xs focus:outline-none focus:border-green-500"
                                />
                            </td>
                            <td class="px-2 py-1">
                                <select
                                    v-model="newSkill.attribute"
                                    class="bg-gray-800 border border-green-700 text-gray-200 rounded px-1 py-0.5 text-xs focus:outline-none focus:border-green-500"
                                >
                                    <option
                                        v-for="a in attributes"
                                        :key="a"
                                        :value="a"
                                    >
                                        {{ a }}
                                    </option>
                                </select>
                            </td>
                            <td class="px-2 py-1">
                                <input
                                    v-model.number="newSkill.base_difficulty"
                                    type="number"
                                    class="bg-gray-800 border border-green-700 text-gray-200 rounded px-1 py-0.5 w-16 text-xs focus:outline-none focus:border-green-500"
                                />
                            </td>
                            <td class="px-2 py-1 text-center">
                                <input
                                    v-model="newSkill.opposed"
                                    type="checkbox"
                                    class="accent-green-500"
                                />
                            </td>
                            <td class="px-2 py-1">
                                <input
                                    v-model="newSkill.description"
                                    placeholder="description"
                                    class="bg-gray-800 border border-green-700 text-gray-200 rounded px-1 py-0.5 w-full text-xs focus:outline-none focus:border-green-500"
                                />
                            </td>
                            <td class="px-2 py-1">
                                <button
                                    data-testid="skill-add-confirm"
                                    class="px-2 py-0.5 text-xs rounded border border-green-700 text-green-400 hover:bg-green-900/30"
                                    @click="handleAddSkill"
                                >
                                    Add
                                </button>
                            </td>
                        </tr>
                        <tr
                            v-for="(sm, idx) in editSkills"
                            :key="sm.verb"
                            :data-testid="`skill-row-${sm.verb}`"
                            class="border-b border-gray-800/50 hover:bg-gray-900/50"
                        >
                            <td class="px-2 py-1 font-mono text-gray-300">
                                {{ sm.verb }}
                            </td>
                            <td class="px-2 py-1">
                                <input
                                    v-model="sm.skill"
                                    list="skill-list"
                                    class="bg-gray-800 border border-gray-700 text-gray-200 rounded px-1 py-0.5 w-24 text-xs focus:outline-none focus:border-gray-500"
                                />
                            </td>
                            <td class="px-2 py-1">
                                <select
                                    v-model="sm.attribute"
                                    class="bg-gray-800 border border-gray-700 text-gray-200 rounded px-1 py-0.5 text-xs focus:outline-none focus:border-gray-500"
                                >
                                    <option
                                        v-for="a in attributes"
                                        :key="a"
                                        :value="a"
                                    >
                                        {{ a }}
                                    </option>
                                </select>
                            </td>
                            <td class="px-2 py-1">
                                <input
                                    v-model.number="sm.base_difficulty"
                                    type="number"
                                    class="bg-gray-800 border border-gray-700 text-gray-200 rounded px-1 py-0.5 w-16 text-xs focus:outline-none focus:border-gray-500"
                                />
                            </td>
                            <td class="px-2 py-1 text-center">
                                <input
                                    v-model="sm.opposed"
                                    type="checkbox"
                                    class="accent-blue-500"
                                />
                            </td>
                            <td class="px-2 py-1">
                                <input
                                    v-model="sm.description"
                                    class="bg-gray-800 border border-gray-700 text-gray-200 rounded px-1 py-0.5 w-full text-xs focus:outline-none focus:border-gray-500"
                                />
                            </td>
                            <td class="px-2 py-1 whitespace-nowrap">
                                <button
                                    :data-testid="`skill-save-${sm.verb}`"
                                    class="px-2 py-0.5 text-xs rounded border border-green-700 text-green-400 hover:bg-green-900/30 mr-1"
                                    @click="handleSaveSkill(idx)"
                                >
                                    Save
                                </button>
                                <button
                                    :data-testid="`skill-reset-${sm.verb}`"
                                    class="px-2 py-0.5 text-xs rounded border border-yellow-700 text-yellow-400 hover:bg-yellow-900/30 mr-1"
                                    @click="handleResetSkill(sm.verb)"
                                >
                                    Reset
                                </button>
                                <button
                                    :data-testid="`skill-delete-${sm.verb}`"
                                    class="px-2 py-0.5 text-xs rounded border border-red-700 text-red-400 hover:bg-red-900/30"
                                    @click="handleDeleteSkill(sm.verb)"
                                >
                                    Delete
                                </button>
                            </td>
                        </tr>
                    </tbody>
                </table>
                <p
                    v-if="editSkills.length === 0 && !store.loading"
                    class="text-gray-600 text-sm mt-4"
                >
                    No skill mappings found.
                </p>
    </div>
</template>
