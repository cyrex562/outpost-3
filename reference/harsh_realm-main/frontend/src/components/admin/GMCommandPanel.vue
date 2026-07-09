<script setup lang="ts">
import { ref, onMounted } from "vue";
import { useAdminStore } from "../../stores/admin";
import type { Character } from "../../types/api";

const store = useAdminStore();

const activeCommand = ref<string>("teleport");
const resultMessage = ref("");
const resultError = ref("");

// Entity list for selectors
const entities = ref<Character[]>([]);

onMounted(async () => {
  await store.loadCharacters();
  entities.value = store.characters;
});

// Teleport
const teleportEntityId = ref("");
const teleportQ = ref(0);
const teleportR = ref(0);

async function doTeleport() {
  resultError.value = "";
  try {
    const res = await store.gmTeleport(teleportEntityId.value, teleportQ.value, teleportR.value);
    resultMessage.value = `Teleported ${res.name} to (${res.q}, ${res.r})`;
  } catch (e) {
    resultError.value = String(e);
  }
}

// Spawn
const spawnType = ref("npc");
const spawnName = ref("");
const spawnQ = ref(0);
const spawnR = ref(0);

async function doSpawn() {
  resultError.value = "";
  try {
    const res = await store.gmSpawnEntity({
      entity_type: spawnType.value,
      name: spawnName.value,
      q: spawnQ.value,
      r: spawnR.value,
    });
    resultMessage.value = `Spawned ${res.name} (${res.entity_type}) at (${res.q}, ${res.r})`;
    spawnName.value = "";
  } catch (e) {
    resultError.value = String(e);
  }
}

// Give item
const giveEntityId = ref("");
const giveItemName = ref("");
const giveItemType = ref("misc");

async function doGiveItem() {
  resultError.value = "";
  try {
    const res = await store.gmGiveItem(giveEntityId.value, {
      name: giveItemName.value,
      type: giveItemType.value,
      data: {},
    });
    resultMessage.value = `Gave ${giveItemName.value} to ${res.name} (${res.equipment_count} items)`;
    giveItemName.value = "";
  } catch (e) {
    resultError.value = String(e);
  }
}

// Set HP
const hpEntityId = ref("");
const hpValue = ref(10);

async function doSetHp() {
  resultError.value = "";
  try {
    const res = await store.gmSetHp(hpEntityId.value, hpValue.value);
    resultMessage.value = `Set ${res.name} HP to ${res.hp}`;
  } catch (e) {
    resultError.value = String(e);
  }
}

// Set Gold
const goldEntityId = ref("");
const goldValue = ref(100);

async function doSetGold() {
  resultError.value = "";
  try {
    const ok = await store.gmSetGold(goldEntityId.value, goldValue.value);
    if (ok) resultMessage.value = `Set gold to ${goldValue.value}`;
    else resultError.value = "Failed to set gold.";
  } catch (e) {
    resultError.value = String(e);
  }
}

// Set XP
const xpEntityId = ref("");
const xpValue = ref(0);

async function doSetXp() {
  resultError.value = "";
  try {
    const ok = await store.gmSetXp(xpEntityId.value, xpValue.value);
    if (ok) resultMessage.value = `Set XP to ${xpValue.value}`;
    else resultError.value = "Failed to set XP.";
  } catch (e) {
    resultError.value = String(e);
  }
}

// Set Attribute
const attrEntityId = ref("");
const attrName = ref("STR");
const attrValue = ref(10);
const attributes = ["STR", "DEX", "CON", "INT", "WIS", "CHA"];

async function doSetAttr() {
  resultError.value = "";
  try {
    const ok = await store.gmSetAttr(attrEntityId.value, attrName.value, attrValue.value);
    if (ok) resultMessage.value = `Set ${attrName.value} to ${attrValue.value}`;
    else resultError.value = "Failed to set attribute.";
  } catch (e) {
    resultError.value = String(e);
  }
}

const commands = [
  { id: "teleport", label: "Teleport" },
  { id: "spawn", label: "Spawn Entity" },
  { id: "give", label: "Give Item" },
  { id: "hp", label: "Set HP" },
  { id: "gold", label: "Set Gold" },
  { id: "xp", label: "Set XP" },
  { id: "attr", label: "Set Attr" },
];
</script>

<template>
  <div class="space-y-4">
    <h3 class="text-sm font-mono text-gray-300">GM Commands</h3>

    <p v-if="resultMessage" class="text-xs text-green-400">{{ resultMessage }}</p>
    <p v-if="resultError" class="text-xs text-red-400">{{ resultError }}</p>

    <!-- Command selector -->
    <div class="flex gap-2">
      <button
        v-for="cmd in commands"
        :key="cmd.id"
        class="px-3 py-1 text-xs font-mono rounded border transition-colors"
        :class="activeCommand === cmd.id
          ? 'border-blue-500 text-blue-400 bg-blue-900/20'
          : 'border-gray-700 text-gray-500 hover:text-gray-300'"
        @click="activeCommand = cmd.id"
      >
        {{ cmd.label }}
      </button>
    </div>

    <!-- Teleport -->
    <div v-if="activeCommand === 'teleport'" class="flex items-end gap-2">
      <div>
        <label class="block text-xs text-gray-500 mb-1">Entity</label>
        <select
          v-model="teleportEntityId"
          class="bg-gray-800 border border-gray-700 text-gray-200 rounded px-2 py-1 text-xs w-48"
        >
          <option value="">Select...</option>
          <option v-for="e in entities" :key="e.id" :value="e.id">
            {{ e.name }} ({{ e.entity_type }})
          </option>
        </select>
      </div>
      <div>
        <label class="block text-xs text-gray-500 mb-1">Q</label>
        <input v-model.number="teleportQ" type="number"
          class="bg-gray-800 border border-gray-700 text-gray-200 rounded px-2 py-1 text-xs w-16" />
      </div>
      <div>
        <label class="block text-xs text-gray-500 mb-1">R</label>
        <input v-model.number="teleportR" type="number"
          class="bg-gray-800 border border-gray-700 text-gray-200 rounded px-2 py-1 text-xs w-16" />
      </div>
      <button
        class="px-3 py-1 text-xs rounded border border-green-700 text-green-400 hover:bg-green-900/30"
        @click="doTeleport"
      >
        Teleport
      </button>
    </div>

    <!-- Spawn -->
    <div v-if="activeCommand === 'spawn'" class="flex items-end gap-2">
      <div>
        <label class="block text-xs text-gray-500 mb-1">Type</label>
        <select v-model="spawnType"
          class="bg-gray-800 border border-gray-700 text-gray-200 rounded px-2 py-1 text-xs">
          <option value="npc">NPC</option>
          <option value="creature">Creature</option>
          <option value="object">Object</option>
        </select>
      </div>
      <div>
        <label class="block text-xs text-gray-500 mb-1">Name</label>
        <input v-model="spawnName"
          class="bg-gray-800 border border-gray-700 text-gray-200 rounded px-2 py-1 text-xs w-40" />
      </div>
      <div>
        <label class="block text-xs text-gray-500 mb-1">Q</label>
        <input v-model.number="spawnQ" type="number"
          class="bg-gray-800 border border-gray-700 text-gray-200 rounded px-2 py-1 text-xs w-16" />
      </div>
      <div>
        <label class="block text-xs text-gray-500 mb-1">R</label>
        <input v-model.number="spawnR" type="number"
          class="bg-gray-800 border border-gray-700 text-gray-200 rounded px-2 py-1 text-xs w-16" />
      </div>
      <button
        class="px-3 py-1 text-xs rounded border border-green-700 text-green-400 hover:bg-green-900/30"
        @click="doSpawn"
      >
        Spawn
      </button>
    </div>

    <!-- Give Item -->
    <div v-if="activeCommand === 'give'" class="flex items-end gap-2">
      <div>
        <label class="block text-xs text-gray-500 mb-1">Entity</label>
        <select v-model="giveEntityId"
          class="bg-gray-800 border border-gray-700 text-gray-200 rounded px-2 py-1 text-xs w-48">
          <option value="">Select...</option>
          <option v-for="e in entities" :key="e.id" :value="e.id">
            {{ e.name }} ({{ e.entity_type }})
          </option>
        </select>
      </div>
      <div>
        <label class="block text-xs text-gray-500 mb-1">Item Name</label>
        <input v-model="giveItemName"
          class="bg-gray-800 border border-gray-700 text-gray-200 rounded px-2 py-1 text-xs w-40" />
      </div>
      <div>
        <label class="block text-xs text-gray-500 mb-1">Type</label>
        <select v-model="giveItemType"
          class="bg-gray-800 border border-gray-700 text-gray-200 rounded px-2 py-1 text-xs">
          <option value="weapon">Weapon</option>
          <option value="armor">Armor</option>
          <option value="consumable">Consumable</option>
          <option value="misc">Misc</option>
          <option value="currency">Currency</option>
        </select>
      </div>
      <button
        class="px-3 py-1 text-xs rounded border border-green-700 text-green-400 hover:bg-green-900/30"
        @click="doGiveItem"
      >
        Give
      </button>
    </div>

    <!-- Set HP -->
    <div v-if="activeCommand === 'hp'" class="flex items-end gap-2">
      <div>
        <label class="block text-xs text-gray-500 mb-1">Entity</label>
        <select v-model="hpEntityId"
          class="bg-gray-800 border border-gray-700 text-gray-200 rounded px-2 py-1 text-xs w-48">
          <option value="">Select...</option>
          <option v-for="e in entities" :key="e.id" :value="e.id">
            {{ e.name }} ({{ e.entity_type }})
          </option>
        </select>
      </div>
      <div>
        <label class="block text-xs text-gray-500 mb-1">HP</label>
        <input v-model.number="hpValue" type="number"
          class="bg-gray-800 border border-gray-700 text-gray-200 rounded px-2 py-1 text-xs w-20" />
      </div>
      <button
        class="px-3 py-1 text-xs rounded border border-green-700 text-green-400 hover:bg-green-900/30"
        @click="doSetHp"
      >
        Set HP
      </button>
    </div>

    <!-- Set Gold -->
    <div v-if="activeCommand === 'gold'" class="flex items-end gap-2">
      <div>
        <label class="block text-xs text-gray-500 mb-1">Entity</label>
        <select v-model="goldEntityId"
          class="bg-gray-800 border border-gray-700 text-gray-200 rounded px-2 py-1 text-xs w-48">
          <option value="">Select...</option>
          <option v-for="e in entities" :key="e.id" :value="e.id">
            {{ e.name }} ({{ e.entity_type }})
          </option>
        </select>
      </div>
      <div>
        <label class="block text-xs text-gray-500 mb-1">Gold</label>
        <input v-model.number="goldValue" type="number"
          class="bg-gray-800 border border-gray-700 text-gray-200 rounded px-2 py-1 text-xs w-20" />
      </div>
      <button
        class="px-3 py-1 text-xs rounded border border-green-700 text-green-400 hover:bg-green-900/30"
        @click="doSetGold"
      >
        Set Gold
      </button>
    </div>

    <!-- Set XP -->
    <div v-if="activeCommand === 'xp'" class="flex items-end gap-2">
      <div>
        <label class="block text-xs text-gray-500 mb-1">Entity</label>
        <select v-model="xpEntityId"
          class="bg-gray-800 border border-gray-700 text-gray-200 rounded px-2 py-1 text-xs w-48">
          <option value="">Select...</option>
          <option v-for="e in entities" :key="e.id" :value="e.id">
            {{ e.name }} ({{ e.entity_type }})
          </option>
        </select>
      </div>
      <div>
        <label class="block text-xs text-gray-500 mb-1">XP</label>
        <input v-model.number="xpValue" type="number"
          class="bg-gray-800 border border-gray-700 text-gray-200 rounded px-2 py-1 text-xs w-24" />
      </div>
      <button
        class="px-3 py-1 text-xs rounded border border-green-700 text-green-400 hover:bg-green-900/30"
        @click="doSetXp"
      >
        Set XP
      </button>
    </div>

    <!-- Set Attribute -->
    <div v-if="activeCommand === 'attr'" class="flex items-end gap-2">
      <div>
        <label class="block text-xs text-gray-500 mb-1">Entity</label>
        <select v-model="attrEntityId"
          class="bg-gray-800 border border-gray-700 text-gray-200 rounded px-2 py-1 text-xs w-48">
          <option value="">Select...</option>
          <option v-for="e in entities" :key="e.id" :value="e.id">
            {{ e.name }} ({{ e.entity_type }})
          </option>
        </select>
      </div>
      <div>
        <label class="block text-xs text-gray-500 mb-1">Attribute</label>
        <select v-model="attrName"
          class="bg-gray-800 border border-gray-700 text-gray-200 rounded px-2 py-1 text-xs w-20">
          <option v-for="a in attributes" :key="a" :value="a">{{ a }}</option>
        </select>
      </div>
      <div>
        <label class="block text-xs text-gray-500 mb-1">Value</label>
        <input v-model.number="attrValue" type="number"
          class="bg-gray-800 border border-gray-700 text-gray-200 rounded px-2 py-1 text-xs w-16" />
      </div>
      <button
        class="px-3 py-1 text-xs rounded border border-green-700 text-green-400 hover:bg-green-900/30"
        @click="doSetAttr"
      >
        Set Attr
      </button>
    </div>
  </div>
</template>
