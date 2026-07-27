<script setup lang="ts">
import { ref } from "vue";
import type { MigrationInfo } from "../api";

const props = withDefaults(
  defineProps<{
    migrations: MigrationInfo[];
    error?: string;
    running?: boolean;
  }>(),
  { error: "", running: false },
);

const emit = defineEmits<{
  refresh: [folder: string, table: string];
  run: [folder: string, table: string];
}>();

const folder = ref("");
const table = ref("");

const pendingCount = () => props.migrations.filter((m) => !m.applied).length;
</script>

<template>
  <div class="migrations-panel">
    <div class="row">
      <input
        v-model="folder"
        placeholder="Migrations folder (e.g. /home/me/project/supabase/migrations)"
      >
      <input
        v-model="table"
        placeholder="Tracking table (blank = auto-detect)"
      >
      <button
        type="button"
        :disabled="!folder.trim()"
        @click="emit('refresh', folder.trim(), table.trim())"
      >
        Refresh
      </button>
      <button
        type="button"
        :disabled="!folder.trim() || running || pendingCount() === 0"
        @click="emit('run', folder.trim(), table.trim())"
      >
        {{ running ? "Running…" : `Run pending (${pendingCount()})` }}
      </button>
    </div>
    <p
      v-if="error"
      class="error"
    >
      {{ error }}
    </p>
    <table v-if="migrations.length > 0">
      <thead>
        <tr>
          <th>Version</th>
          <th>Name</th>
          <th>Status</th>
        </tr>
      </thead>
      <tbody>
        <tr
          v-for="m in migrations"
          :key="m.version"
        >
          <td class="expr">
            {{ m.version }}
          </td>
          <td>{{ m.name || "—" }}</td>
          <td :class="{ applied: m.applied, pending: !m.applied }">
            {{ m.applied ? "applied" : "pending" }}
          </td>
        </tr>
      </tbody>
    </table>
    <p
      v-else
      class="empty"
    >
      No migrations loaded — set a folder and click Refresh.
    </p>
  </div>
</template>

<style scoped>
.migrations-panel {
  padding: 0.5rem;
  overflow: auto;
  flex: 1;
}
.row {
  display: flex;
  gap: 0.4rem;
  margin-bottom: 0.6rem;
}
.row input {
  flex: 1;
}
table {
  border-collapse: collapse;
  font-size: 0.85rem;
  width: 100%;
}
th,
td {
  text-align: left;
  padding: 0.3rem 0.6rem;
  border-bottom: 1px solid rgba(128, 128, 128, 0.25);
}
.expr {
  font-family: monospace;
  font-size: 0.78rem;
}
.applied {
  color: #34a06f;
}
.pending {
  opacity: 0.7;
}
.empty {
  opacity: 0.6;
}
.error {
  color: #e06c75;
}
</style>
