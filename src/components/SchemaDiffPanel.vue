<script setup lang="ts">
import { ref } from "vue";
import { schemaDiff, type ConnectionInfo, type SchemaDiff } from "../api";
import DataGrid from "./DataGrid.vue";

defineProps<{
  connections: ConnectionInfo[];
}>();

const leftConnectionId = ref<string>("");
const rightConnectionId = ref<string>("");
const diff = ref<SchemaDiff | null>(null);
const error = ref("");
const loading = ref(false);

async function runDiff() {
  if (!leftConnectionId.value || !rightConnectionId.value) {
    error.value = "Select both connections to compare";
    return;
  }
  if (leftConnectionId.value === rightConnectionId.value) {
    error.value = "Select different connections to compare";
    return;
  }

  loading.value = true;
  error.value = "";
  try {
    diff.value = await schemaDiff(leftConnectionId.value, rightConnectionId.value);
  } catch (e) {
    error.value = String(e);
    diff.value = null;
  } finally {
    loading.value = false;
  }
}

function hasChanges(): boolean {
  if (!diff.value) return false;
  return (
    diff.value.tablesOnlyLeft.length > 0 ||
    diff.value.tablesOnlyRight.length > 0 ||
    diff.value.columnsOnlyLeft.length > 0 ||
    diff.value.columnsOnlyRight.length > 0 ||
    diff.value.columnTypeMismatches.length > 0
  );
}
</script>

<template>
  <div class="schema-diff-panel">
    <div class="controls">
      <div class="select-group">
        <label for="left">Left (dev):</label>
        <select
          id="left"
          v-model="leftConnectionId"
        >
          <option value="">
            Select connection...
          </option>
          <option
            v-for="c in connections"
            :key="c.id"
            :value="c.id"
          >
            {{ c.name }}
          </option>
        </select>
      </div>
      <div class="select-group">
        <label for="right">Right (prod):</label>
        <select
          id="right"
          v-model="rightConnectionId"
        >
          <option value="">
            Select connection...
          </option>
          <option
            v-for="c in connections"
            :key="c.id"
            :value="c.id"
          >
            {{ c.name }}
          </option>
        </select>
      </div>
      <button
        :disabled="loading"
        @click="runDiff"
      >
        {{ loading ? "Comparing..." : "Compare" }}
      </button>
    </div>

    <p
      v-if="error"
      class="error"
    >
      {{ error }}
    </p>

    <template v-if="diff">
      <template v-if="hasChanges()">
        <template v-if="diff.tablesOnlyLeft.length > 0">
          <h3>Tables only in left</h3>
          <DataGrid :rows="diff.tablesOnlyLeft" />
        </template>

        <template v-if="diff.tablesOnlyRight.length > 0">
          <h3>Tables only in right</h3>
          <DataGrid :rows="diff.tablesOnlyRight" />
        </template>

        <template v-if="diff.columnsOnlyLeft.length > 0">
          <h3>Columns only in left</h3>
          <DataGrid :rows="diff.columnsOnlyLeft" />
        </template>

        <template v-if="diff.columnsOnlyRight.length > 0">
          <h3>Columns only in right</h3>
          <DataGrid :rows="diff.columnsOnlyRight" />
        </template>

        <template v-if="diff.columnTypeMismatches.length > 0">
          <h3>Column type mismatches</h3>
          <DataGrid :rows="diff.columnTypeMismatches" />
        </template>
      </template>
      <p
        v-else
        class="info"
      >
        Schemas are identical.
      </p>
    </template>
  </div>
</template>

<style scoped>
.schema-diff-panel {
  padding: 1rem;
  overflow-y: auto;
}

.controls {
  display: flex;
  gap: 1rem;
  margin-bottom: 1rem;
  align-items: center;
  flex-wrap: wrap;
}

.select-group {
  display: flex;
  align-items: center;
  gap: 0.5rem;
}

label {
  font-size: 0.9rem;
}

select {
  min-width: 150px;
}

h3 {
  margin-top: 1rem;
  margin-bottom: 0.5rem;
}

.error {
  color: #ff6b6b;
  font-size: 0.85rem;
}

.info {
  opacity: 0.7;
}
</style>
