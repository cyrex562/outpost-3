<script setup lang="ts">
import FieldInput from "./shared/FieldInput.vue";
import WeightedListEditor from "./widgets/WeightedListEditor.vue";
import RegistryEditor from "./widgets/RegistryEditor.vue";
import TieredConfigEditor from "./widgets/TieredConfigEditor.vue";
import RangeTableEditor from "./widgets/RangeTableEditor.vue";
import MatrixEditor from "./widgets/MatrixEditor.vue";
import GroupedListEditor from "./widgets/GroupedListEditor.vue";
import PasteImporter from "./PasteImporter.vue";
import { useSchemaFormEditor } from "./useSchemaFormEditor";

const props = defineProps<{
  filePath: string;
}>();

const emit = defineEmits<{
  saved: [];
  status: [msg: string];
}>();

const {
  schema, entries, tieredData, matrixData, matrixRowKeys, matrixColKeys,
  groupedData, loading, editingRaw, rawContent, showPasteImporter,
  widgetName, tierKeys, save, saveRaw, getMetaValue, setMetaValue,
  onPasteImport,
} = useSchemaFormEditor(() => props.filePath, emit);
</script>

<template>
  <div v-if="loading" class="text-gray-500 text-xs py-4">Loading...</div>

  <div v-else-if="!schema" class="text-gray-500 text-xs py-4">
    No schema found for this file. Use raw YAML editing.
  </div>

  <div v-else-if="editingRaw">
    <textarea
      v-model="rawContent"
      class="w-full h-96 bg-gray-800 border border-gray-700 text-gray-200 rounded px-3 py-2 text-xs font-mono focus:outline-none focus:border-gray-500"
      spellcheck="false"
    ></textarea>
    <div class="flex gap-2 mt-2">
      <button
        class="px-3 py-1 text-xs rounded border border-green-700 text-green-400 hover:bg-green-900/30"
        @click="saveRaw"
      >Save</button>
      <button
        class="px-3 py-1 text-xs rounded border border-gray-600 text-gray-400 hover:bg-gray-800"
        @click="editingRaw = false"
      >Cancel</button>
    </div>
  </div>

  <div v-else>
    <!-- Meta fields -->
    <div v-if="schema.metaFields?.length" class="flex flex-wrap items-center gap-3 mb-3">
      <div v-for="mf in schema.metaFields" :key="mf.key" class="flex items-center gap-1">
        <label class="text-xs text-gray-500">{{ mf.label ?? mf.key }}:</label>
        <FieldInput
          :field="mf"
          :modelValue="getMetaValue(mf.key)"
          @update:modelValue="setMetaValue(mf.key, $event)"
        />
      </div>
    </div>

    <!-- Toolbar -->
    <div class="flex items-center gap-2 mb-3">
      <button
        class="px-3 py-1 text-xs rounded border border-green-700 text-green-400 hover:bg-green-900/30"
        @click="save"
      >Save</button>
      <button
        class="px-3 py-1 text-xs rounded border border-gray-600 text-gray-400 hover:bg-gray-800"
        @click="editingRaw = true"
      >Edit Raw YAML</button>
      <button
        v-if="widgetName !== 'MatrixEditor'"
        class="px-3 py-1 text-xs rounded border border-blue-700 text-blue-400 hover:bg-blue-900/30"
        @click="showPasteImporter = true"
      >Paste Import</button>
    </div>

    <!-- Widget -->
    <WeightedListEditor
      v-if="widgetName === 'WeightedListEditor'"
      :entries="entries"
      :fields="schema.widget.fields"
      :sortable="schema.widget.sortable"
      :allowAdd="schema.widget.allowAdd"
      :allowDelete="schema.widget.allowDelete"
      @update:entries="entries = $event"
    />

    <RegistryEditor
      v-else-if="widgetName === 'RegistryEditor'"
      :entries="entries"
      :fields="schema.widget.fields"
      :entryKey="schema.widget.entryKey"
      :allowAdd="schema.widget.allowAdd"
      :allowDelete="schema.widget.allowDelete"
      @update:entries="entries = $event"
    />

    <TieredConfigEditor
      v-else-if="widgetName === 'TieredConfigEditor'"
      :tiers="tieredData"
      :tierKeys="tierKeys"
      :fields="schema.widget.fields"
      :allowAdd="schema.widget.allowAdd"
      :allowDelete="schema.widget.allowDelete"
      @update:tiers="tieredData = $event"
    />

    <RangeTableEditor
      v-else-if="widgetName === 'RangeTableEditor'"
      :entries="entries"
      :fields="schema.widget.fields"
      :allowAdd="schema.widget.allowAdd"
      :allowDelete="schema.widget.allowDelete"
      @update:entries="entries = $event"
    />

    <MatrixEditor
      v-else-if="widgetName === 'MatrixEditor'"
      :matrix="matrixData"
      :rowKeys="matrixRowKeys"
      :colKeys="matrixColKeys"
      @update:matrix="matrixData = $event"
    />

    <GroupedListEditor
      v-else-if="widgetName === 'GroupedListEditor'"
      :groups="groupedData"
      :fields="schema.widget.fields"
      :allowAdd="schema.widget.allowAdd"
      :allowDelete="schema.widget.allowDelete"
      @update:groups="groupedData = $event"
    />

    <div v-else class="text-gray-500 text-xs py-4">
      Unknown widget: {{ widgetName }}
    </div>

    <!-- Paste importer modal -->
    <PasteImporter
      v-if="showPasteImporter && schema"
      :fields="schema.widget.fields"
      @import="onPasteImport"
      @close="showPasteImporter = false"
    />
  </div>
</template>
