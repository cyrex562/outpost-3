<script setup lang="ts">
import { ref } from "vue";

type Tab = "skills" | "commands";
const activeTab = ref<Tab>("skills");

interface SkillEntry {
  name: string;
  attr: string;
  description: string;
}

interface CommandEntry {
  syntax: string;
  description: string;
}

const SKILLS: SkillEntry[] = [
  { name: "Administer", attr: "INT", description: "Manage organizations, navigate bureaucracy, handle logistics and administration." },
  { name: "Connect",    attr: "CHA", description: "Find useful contacts, extract information through social networks, call in favors." },
  { name: "Exert",      attr: "STR/CON", description: "Apply physical force, feats of strength, endurance, and athletic ability." },
  { name: "Fix",        attr: "INT", description: "Repair devices, build equipment, understand mechanical and electronic systems." },
  { name: "Heal",       attr: "INT", description: "Treat wounds, cure disease, perform surgery, and understand anatomy." },
  { name: "Know",       attr: "INT", description: "Recall history, science, esoteric lore, and general scholarship." },
  { name: "Lead",       attr: "CHA", description: "Inspire and command others under pressure; manage morale and group cohesion." },
  { name: "Notice",     attr: "WIS", description: "Perceive hidden details, spot ambushes, sense lies, and read a room." },
  { name: "Perform",    attr: "CHA", description: "Entertain crowds, create art, play instruments, or give compelling speeches." },
  { name: "Pilot",      attr: "DEX", description: "Operate vehicles, mounts, and primitive aircraft or watercraft." },
  { name: "Program",    attr: "INT", description: "Code software, hack computer systems, and interface with TL4+ technology." },
  { name: "Punch",      attr: "STR/DEX", description: "Fight unarmed or with natural weapons; brawling and hand-to-hand combat." },
  { name: "Ride",       attr: "DEX", description: "Handle animals, ride mounts, and manage beasts of burden." },
  { name: "Sail",       attr: "DEX", description: "Navigate watercraft, read weather, and pilot vessels by sail or oar." },
  { name: "Shoot",      attr: "DEX", description: "Use ranged weapons — bows, firearms, crossbows — with accuracy." },
  { name: "Sneak",      attr: "DEX", description: "Move silently, hide, pick pockets, bypass locks, and avoid detection." },
  { name: "Stab",       attr: "STR/DEX", description: "Fight with melee weapons: swords, axes, knives, spears." },
  { name: "Survive",    attr: "WIS", description: "Navigate wilderness, forage for food and water, track prey, avoid natural hazards." },
  { name: "Talk",       attr: "CHA", description: "Persuade, deceive, negotiate, and read social dynamics." },
  { name: "Trade",      attr: "CHA", description: "Buy, sell, appraise goods, manage commercial ventures, and spot a bargain." },
  { name: "Work",       attr: "varies", description: "A catch-all for mundane labor and practiced trades (farming, mining, crafts)." },
];

const SKILL_LEVELS = [
  { level: -1, label: "Untrained", note: "−1 penalty to checks" },
  { level: 0,  label: "Trained",   note: "No penalty; basic competence" },
  { level: 1,  label: "Novice",    note: "+1 to checks" },
  { level: 2,  label: "Veteran",   note: "+2 to checks" },
  { level: 3,  label: "Expert",    note: "+3 to checks" },
  { level: 4,  label: "Master",    note: "+4 to checks; near the human limit" },
];

const COMMANDS: CommandEntry[] = [
  { syntax: "look / l",               description: "Describe your current hex — terrain, features, and what lies nearby." },
  { syntax: "go <dir> / <dir>",       description: "Move in a direction. Valid: northeast (ne), east (e), southeast (se), southwest (sw), west (w), northwest (nw). No pure north/south — the hex grid has six diagonal/lateral directions." },
  { syntax: "search",                 description: "Search the current hex for discoveries. Results depend on terrain; some finds require a skill check." },
  { syntax: "explore",                description: "Explore a settlement hex in detail — lists establishments and notable residents." },
  { syntax: "examine <name>",         description: "Examine an NPC at your current location. Shows description, occupation, and demeanor." },
  { syntax: "talk to <name>",         description: "Speak with an NPC. Their response reflects their personality and disposition." },
  { syntax: "status / stats",         description: "Display your full character sheet: attributes, skills, HP, AC, XP, and equipment." },
  { syntax: "inventory / inv",        description: "List everything you are carrying." },
  { syntax: "wait / rest",            description: "Pass time without acting. May be required for certain events to trigger." },
  { syntax: "help / ?",               description: "List available commands for the current scene." },
];
</script>

<template>
  <div class="flex flex-col h-full font-mono text-sm bg-gray-900 text-gray-200">
    <!-- Tab bar -->
    <div class="flex border-b border-gray-700 shrink-0">
      <button
        v-for="tab in (['skills', 'commands'] as Tab[])"
        :key="tab"
        class="px-4 py-2 text-xs uppercase tracking-wider transition-colors"
        :class="activeTab === tab
          ? 'text-green-400 border-b-2 border-green-400 -mb-px bg-gray-900'
          : 'text-gray-500 hover:text-gray-300'"
        @click="activeTab = tab"
      >
        {{ tab }}
      </button>
    </div>

    <!-- Tab content -->
    <div class="flex-1 overflow-y-auto min-h-0">

      <!-- Skills tab -->
      <div v-if="activeTab === 'skills'" class="p-3 space-y-4">
        <!-- Skill check reminder -->
        <div class="text-xs text-gray-500 border border-gray-700 rounded p-2">
          <span class="text-gray-400 font-bold">Skill check: </span>2d6 + skill level + attribute modifier vs difficulty (usually 8).
        </div>

        <!-- Skill levels reference -->
        <div>
          <div class="text-xs text-gray-400 uppercase tracking-wider mb-1">Skill Levels</div>
          <div class="grid grid-cols-2 gap-x-4 gap-y-0.5">
            <div
              v-for="lvl in SKILL_LEVELS"
              :key="lvl.level"
              class="flex items-baseline gap-2 text-xs py-0.5"
            >
              <span class="w-4 text-right text-yellow-500 shrink-0">{{ lvl.level }}</span>
              <span class="text-gray-300 shrink-0">{{ lvl.label }}</span>
              <span class="text-gray-600">{{ lvl.note }}</span>
            </div>
          </div>
        </div>

        <!-- Skill list -->
        <div>
          <div class="text-xs text-gray-400 uppercase tracking-wider mb-2">All Skills</div>
          <div class="space-y-2">
            <div v-for="skill in SKILLS" :key="skill.name" class="border-l-2 border-gray-700 pl-2">
              <div class="flex items-baseline gap-2">
                <span class="text-green-400 font-bold">{{ skill.name }}</span>
                <span class="text-xs text-gray-500">[{{ skill.attr }}]</span>
              </div>
              <p class="text-xs text-gray-400 leading-relaxed mt-0.5">{{ skill.description }}</p>
            </div>
          </div>
        </div>
      </div>

      <!-- Commands tab -->
      <div v-if="activeTab === 'commands'" class="p-3 space-y-2">
        <div class="text-xs text-gray-500 border border-gray-700 rounded p-2">
          Commands are entered in the <span class="text-gray-300">Chat / Log</span> panel. Most accept shorthand (e.g. <span class="text-green-400">n</span> for north, <span class="text-green-400">l</span> for look).
        </div>
        <div class="space-y-3 mt-2">
          <div v-for="cmd in COMMANDS" :key="cmd.syntax" class="border-l-2 border-gray-700 pl-2">
            <div class="text-green-400 font-bold text-xs">{{ cmd.syntax }}</div>
            <p class="text-xs text-gray-400 leading-relaxed mt-0.5">{{ cmd.description }}</p>
          </div>
        </div>
      </div>

    </div>
  </div>
</template>
