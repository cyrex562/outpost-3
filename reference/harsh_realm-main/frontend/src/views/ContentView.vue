<script setup lang="ts">
import { computed, onMounted, ref } from "vue";

import { useContentStore } from "../stores/content";
import { useWorldStore } from "../stores/world";

const store = useContentStore();
const worldStore = useWorldStore();

const editorText = ref<string>("");
const showReference = ref<boolean>(false);
const showBrowser = ref<boolean>(false);
const selectedCategory = ref<string>("");
const fileInput = ref<HTMLInputElement | null>(null);
const demoCreatureId = ref<string>("");
const demoWeaponId = ref<string>("");

const report = computed(() => store.report);
const hasDiagnostics = computed(() => {
  const r = store.report;
  return r !== null && (r.syntax_errors.length > 0 || r.unimplemented.length > 0);
});
const succeeded = computed(() => {
  const r = store.report;
  return r !== null && r.syntax_errors.length === 0 && r.unimplemented.length === 0;
});
const storedSummary = computed(() =>
  Object.entries(store.storedCounts)
    .map(([type, n]) => `${type}: ${n}`)
    .join("  ·  "),
);

/** Stored records grouped by component type → sorted ids. */
const groupedRecords = computed<Record<string, string[]>>(() => {
  const groups: Record<string, string[]> = {};
  for (const record of store.storedRecords) {
    const type = String(record.component_type ?? "?");
    const id = String(record.id ?? "?");
    (groups[type] ??= []).push(id);
  }
  for (const ids of Object.values(groups)) {
    ids.sort();
  }
  return groups;
});

onMounted(async () => {
  // Resolve the active world so content can be stored/browsed even when the
  // user came straight here (not via gameplay).
  await Promise.all([worldStore.fetchWorlds(), worldStore.fetchCurrent()]);
  await Promise.all([
    store.loadReference(),
    store.loadLegacyCategories(),
    store.loadStoredRecords(),
  ]);
});

async function onSelectWorld(event: Event): Promise<void> {
  const file = (event.target as HTMLSelectElement).value;
  if (!file) return;
  const ok = await worldStore.loadWorld(file);
  if (ok) await store.loadStoredRecords();
}

function toggleBrowser(): void {
  showBrowser.value = !showBrowser.value;
}

async function onLoadRecord(componentType: string, id: string): Promise<void> {
  editorText.value = await store.loadRecordYaml(componentType, id);
}

async function onDeleteRecord(componentType: string, id: string): Promise<void> {
  await store.deleteRecord(componentType, id);
}

async function onCompile(): Promise<void> {
  await store.compile(editorText.value);
}

async function onStore(): Promise<void> {
  await store.store(editorText.value);
}

async function onMigrate(): Promise<void> {
  if (!selectedCategory.value) {
    return;
  }
  editorText.value = await store.migrate(selectedCategory.value);
}

async function onSeedActions(): Promise<void> {
  await store.seedActions();
}

async function onLoadTemplate(): Promise<void> {
  editorText.value = await store.loadTemplate();
}

function onUploadClick(): void {
  fileInput.value?.click();
}

function onFileChosen(event: Event): void {
  const input = event.target as HTMLInputElement;
  const file = input.files?.[0];
  if (!file) {
    return;
  }
  const reader = new FileReader();
  reader.onload = () => {
    editorText.value = typeof reader.result === "string" ? reader.result : "";
  };
  reader.readAsText(file);
  input.value = "";
}

function onDownload(): void {
  const blob = new Blob([editorText.value], { type: "text/yaml" });
  const url = URL.createObjectURL(blob);
  const anchor = document.createElement("a");
  anchor.href = url;
  anchor.download = "content.yaml";
  anchor.click();
  URL.revokeObjectURL(url);
}

function toggleReference(): void {
  showReference.value = !showReference.value;
}

const demoOutcome = computed(() => store.demo?.outcome ?? null);
const demoError = computed(() => store.demo?.error ?? null);

async function onRunDemo(): Promise<void> {
  if (!demoCreatureId.value) {
    return;
  }
  await store.runDemoCombat(
    editorText.value,
    demoCreatureId.value,
    demoWeaponId.value || null,
  );
}
</script>

<template>
  <div class="min-h-screen bg-gray-900 text-gray-200 font-mono flex flex-col">
    <header class="flex items-center justify-between px-4 py-3 border-b border-gray-700">
      <h1 class="text-lg font-bold">Content Authoring</h1>
      <div class="flex items-center gap-2">
        <button data-testid="load-template" class="toolbtn" @click="onLoadTemplate">
          Load template
        </button>
        <button data-testid="upload-yaml" class="toolbtn" @click="onUploadClick">Upload</button>
        <button data-testid="download-yaml" class="toolbtn" @click="onDownload">Download</button>
        <button data-testid="toggle-browser" class="toolbtn" @click="toggleBrowser">
          {{ showBrowser ? "Hide" : "Browse" }} stored
        </button>
        <button data-testid="toggle-reference" class="toolbtn" @click="toggleReference">
          {{ showReference ? "Hide" : "Show" }} reference
        </button>
        <RouterLink to="/" class="toolbtn">Back to Game</RouterLink>
      </div>
    </header>

    <!-- Active world bar (HR-404): load a world without leaving Content -->
    <div class="flex items-center gap-2 px-4 py-2 border-b border-gray-800 text-sm bg-gray-950/40">
      <span class="text-gray-500">Active world:</span>
      <select
        :value="worldStore.activeWorld ?? ''"
        data-testid="content-world-select"
        class="bg-gray-950 border border-gray-700 rounded px-2 py-1"
        @change="onSelectWorld"
      >
        <option value="">— none —</option>
        <option v-for="w in worldStore.available" :key="w.file" :value="w.file">
          {{ w.name }} ({{ w.file }})
        </option>
      </select>
      <span
        v-if="worldStore.activeWorld"
        data-testid="content-active-world"
        class="text-emerald-400"
      >
        {{ worldStore.activeName }}
      </span>
      <span v-else data-testid="content-no-world" class="text-amber-400">
        No world loaded — load one to Store or Browse compiled records (Compile works without one).
      </span>
    </div>

    <!-- Migration toolbar -->
    <div class="flex items-center gap-2 px-4 py-2 border-b border-gray-800 text-sm">
      <span class="text-gray-500">Port legacy content:</span>
      <select
        v-model="selectedCategory"
        data-testid="legacy-category"
        class="bg-gray-950 border border-gray-700 rounded px-2 py-1"
      >
        <option value="">Select category…</option>
        <option v-for="c in store.legacyCategories" :key="c.category" :value="c.category">
          {{ c.category }} ({{ c.count }})
        </option>
      </select>
      <button
        data-testid="migrate-button"
        :disabled="!selectedCategory"
        class="toolbtn disabled:opacity-50"
        @click="onMigrate"
      >
        Migrate → grammar
      </button>
      <button data-testid="seed-actions" class="toolbtn" @click="onSeedActions">
        Seed action vocabulary
      </button>
      <span class="flex-1" />
      <span v-if="storedSummary" data-testid="stored-summary" class="text-gray-500">
        Stored — {{ storedSummary }}
      </span>
    </div>

    <input
      ref="fileInput"
      type="file"
      accept=".yaml,.yml"
      class="hidden"
      data-testid="file-input"
      @change="onFileChosen"
    />

    <div class="flex flex-1 overflow-hidden">
      <!-- Stored IR browser -->
      <aside
        v-if="showBrowser"
        data-testid="record-browser"
        class="w-72 border-r border-gray-700 p-3 overflow-auto bg-gray-950 text-sm"
      >
        <div class="flex items-center justify-between mb-2">
          <h2 class="font-bold text-gray-300">Stored records</h2>
          <button
            data-testid="browser-refresh"
            class="toolbtn"
            @click="store.loadStoredRecords()"
          >
            Refresh
          </button>
        </div>
        <p v-if="store.storedRecords.length === 0" class="text-gray-500">
          Nothing stored yet. Compile + “Store to world”, or seed the actions.
        </p>
        <div v-for="(ids, type) in groupedRecords" :key="type" class="mb-3">
          <h3 class="text-gray-500 uppercase text-xs tracking-wide mb-1">
            {{ type }} ({{ ids.length }})
          </h3>
          <ul>
            <li
              v-for="id in ids"
              :key="id"
              class="flex items-center justify-between group hover:bg-gray-900 rounded px-1"
            >
              <button
                class="text-left flex-1 text-gray-300 hover:text-emerald-300 truncate"
                :data-testid="`record-${type}-${id}`"
                @click="onLoadRecord(type, id)"
              >
                {{ id }}
              </button>
              <button
                class="text-rose-500 hover:text-rose-300 px-1 opacity-0 group-hover:opacity-100"
                :data-testid="`delete-${type}-${id}`"
                title="Delete"
                @click="onDeleteRecord(type, id)"
              >
                ×
              </button>
            </li>
          </ul>
        </div>
      </aside>

      <section class="flex flex-col flex-1 p-4 gap-3 overflow-auto">
        <textarea
          v-model="editorText"
          data-testid="yaml-editor"
          spellcheck="false"
          placeholder="Write form-2 YAML, migrate a legacy category, or upload a file — then Compile / Store…"
          class="flex-1 min-h-[280px] bg-gray-950 border border-gray-700 rounded p-3 text-sm resize-none focus:outline-none focus:border-gray-500"
        />
        <div class="flex items-center gap-3">
          <button
            data-testid="compile-button"
            :disabled="store.compiling"
            class="px-4 py-2 text-sm rounded bg-emerald-700 hover:bg-emerald-600 disabled:opacity-50"
            @click="onCompile"
          >
            {{ store.compiling ? "Compiling…" : "Compile" }}
          </button>
          <button
            data-testid="store-button"
            :disabled="!worldStore.activeWorld"
            :title="worldStore.activeWorld ? '' : 'Load a world first'"
            class="px-4 py-2 text-sm rounded bg-sky-700 hover:bg-sky-600 disabled:opacity-50 disabled:cursor-not-allowed"
            @click="onStore"
          >
            Store to world
          </button>
          <span v-if="store.error" data-testid="compile-error" class="text-rose-400 text-sm">
            {{ store.error }}
          </span>
        </div>

        <div v-if="store.storedCount !== null" data-testid="store-result" class="text-sky-300 text-sm">
          Stored {{ store.storedCount }} record(s) to the world's IR store.
        </div>

        <!-- Content demo: run an isolated combat to see new content in action. -->
        <div class="flex flex-wrap items-center gap-2 border-t border-gray-800 pt-3 text-sm">
          <span class="text-gray-500">Demo in combat:</span>
          <input
            v-model="demoCreatureId"
            data-testid="demo-creature-id"
            placeholder="creature id (from your YAML)"
            class="bg-gray-950 border border-gray-700 rounded px-2 py-1 w-56"
          />
          <input
            v-model="demoWeaponId"
            data-testid="demo-weapon-id"
            placeholder="weapon id (optional)"
            class="bg-gray-950 border border-gray-700 rounded px-2 py-1 w-48"
          />
          <button
            data-testid="demo-combat-run"
            :disabled="store.demoRunning || !demoCreatureId"
            class="px-3 py-1 text-sm rounded bg-purple-700 hover:bg-purple-600 disabled:opacity-50 disabled:cursor-not-allowed"
            @click="onRunDemo"
          >
            {{ store.demoRunning ? "Running…" : "Run demo" }}
          </button>
          <span class="text-gray-600 text-xs">
            Compiles your YAML and fights a PC vs the named creature — no world needed.
          </span>
        </div>

        <div v-if="demoError" data-testid="demo-error" class="text-rose-400 text-sm">
          {{ demoError }}
        </div>

        <div v-if="demoOutcome" data-testid="demo-transcript" class="text-sm space-y-2">
          <h2 class="font-bold text-purple-300">
            Demo result — winner: {{ demoOutcome.winner }} ({{ demoOutcome.rounds }} round(s))
          </h2>
          <ol class="space-y-1">
            <li
              v-for="(turn, i) in demoOutcome.transcript"
              :key="i"
              class="border-l-2 border-gray-700 pl-2"
            >
              <span class="text-gray-400">R{{ turn.round }}</span>
              <span class="text-gray-200">
                {{ turn.attacker }} → {{ turn.target }}:
                <span :class="turn.hit ? 'text-emerald-400' : 'text-gray-500'">
                  {{ turn.hit ? `hit for ${turn.damage}` : "miss" }}
                </span>
              </span>
              <span class="text-gray-500"> ({{ turn.target }} hp {{ turn.target_hp }})</span>
              <ul v-if="turn.triggers.length" class="ml-4 text-amber-300 text-xs">
                <li v-for="(note, j) in turn.triggers" :key="j">⚡ {{ note }}</li>
              </ul>
            </li>
          </ol>
          <p v-if="demoOutcome.errors.length" class="text-rose-400 text-xs">
            Runtime errors: {{ demoOutcome.errors.join("; ") }}
          </p>
        </div>

        <div v-if="succeeded" data-testid="compile-success" class="text-emerald-400 text-sm">
          ✓ Compiled {{ report?.records.length ?? 0 }} record(s). No diagnostics.
        </div>

        <div v-if="hasDiagnostics" data-testid="diagnostics" class="text-sm space-y-3">
          <div v-if="report && report.syntax_errors.length > 0">
            <h2 class="text-rose-400 font-bold">Syntax errors — your text is malformed:</h2>
            <ul class="list-disc ml-5 text-rose-300">
              <li v-for="(d, i) in report.syntax_errors" :key="`s${i}`">
                {{ d.message }}<span v-if="d.location" class="text-gray-500"> [{{ d.location }}]</span>
              </li>
            </ul>
          </div>
          <div v-if="report && report.unimplemented.length > 0">
            <h2 class="text-amber-400 font-bold">
              Unimplemented primitives — unknown to the engine:
            </h2>
            <ul class="list-disc ml-5 text-amber-300">
              <li v-for="(d, i) in report.unimplemented" :key="`u${i}`">
                {{ d.message }}<span v-if="d.location" class="text-gray-500"> [{{ d.location }}]</span>
              </li>
            </ul>
          </div>
        </div>

        <!-- Migration guidance -->
        <div v-if="store.unmet.length > 0" data-testid="migration-unmet" class="text-sm">
          <h2 class="text-amber-400 font-bold">
            Unmet references — not yet in the IR store:
          </h2>
          <p class="text-amber-300">{{ store.unmet.join(", ") }}</p>
          <p class="text-gray-500 text-xs mt-1">
            Tip: click “Seed action vocabulary” to store the standard attack actions.
          </p>
        </div>
        <div
          v-else-if="store.needs.length > 0"
          data-testid="migration-needs"
          class="text-sm"
        >
          <h2 class="text-emerald-400 font-bold">All references resolved ✓</h2>
          <p class="text-gray-400">{{ store.needs.join(", ") }}</p>
        </div>
        <div v-if="store.notes.length > 0" data-testid="migration-notes" class="text-sm">
          <h2 class="text-gray-400 font-bold">Migration notes:</h2>
          <ul class="list-disc ml-5 text-gray-400">
            <li v-for="(n, i) in store.notes" :key="`n${i}`">{{ n }}</li>
          </ul>
        </div>
      </section>

      <aside
        v-if="showReference"
        data-testid="reference-panel"
        class="w-1/2 border-l border-gray-700 p-4 overflow-auto bg-gray-950"
      >
        <pre class="text-xs whitespace-pre-wrap leading-relaxed">{{ store.reference }}</pre>
      </aside>
    </div>
  </div>
</template>

<style scoped>
.toolbtn {
  @apply px-2 py-1 text-xs rounded border border-gray-600 hover:border-gray-400;
}
</style>
