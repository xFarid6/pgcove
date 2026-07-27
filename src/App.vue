<script setup lang="ts">
import { onMounted, ref } from "vue";
import {
  deleteConnection,
  listAuthUsers,
  listConnections,
  listPolicies,
  listTables,
  runQuery,
  saveConnection,
  supabaseListBuckets,
  supabaseProjectInfo,
  tableRows,
  testConnection,
  type AuthUser,
  type ConnectionInfo,
  type PolicyInfo,
  type Row,
  type StorageBucket,
  type SupabaseProjectInfo,
  type TableInfo,
} from "./api";
import ConnectionForm from "./components/ConnectionForm.vue";
import ConnectionList from "./components/ConnectionList.vue";
import DataGrid from "./components/DataGrid.vue";
import QueryEditor from "./components/QueryEditor.vue";
import SupabasePanel from "./components/SupabasePanel.vue";
import TableList from "./components/TableList.vue";

const connections = ref<ConnectionInfo[]>([]);
const activeId = ref<string | null>(null);
const tables = ref<TableInfo[]>([]);
const activeTable = ref<TableInfo | null>(null);
const rows = ref<Row[]>([]);
const policies = ref<PolicyInfo[]>([]);
const authUsers = ref<AuthUser[]>([]);
const authError = ref("");
const projectInfo = ref<SupabaseProjectInfo | null>(null);
const buckets = ref<StorageBucket[]>([]);
const projectError = ref("");
const tab = ref<"data" | "query" | "supabase">("data");
const status = ref("");
const error = ref("");
const queryRows = ref<Row[]>([]);
const queryError = ref("");
const queryRunning = ref(false);

async function refreshConnections() {
  connections.value = await listConnections();
}

async function onSelect(id: string) {
  activeId.value = id;
  activeTable.value = null;
  rows.value = [];
  error.value = "";
  try {
    tables.value = await listTables(id);
  } catch (e) {
    error.value = String(e);
    tables.value = [];
    return;
  }
  // Policies/auth users are a Postgres/Supabase-only feature (SQLite errors
  // on both) — failing here shouldn't wipe out the tables list above.
  try {
    policies.value = await listPolicies(id);
  } catch {
    policies.value = [];
  }
  try {
    authUsers.value = await listAuthUsers(id);
    authError.value = "";
  } catch (e) {
    authUsers.value = [];
    authError.value = `auth.users not readable — not a Supabase database? (${e})`;
  }
  await refreshSupabaseProject(id);
}

/** Project URL + service-role key APIs; only Supabase connections have them. */
async function refreshSupabaseProject(id: string) {
  projectInfo.value = null;
  buckets.value = [];
  projectError.value = "";
  if (!connections.value.find((c) => c.id === id)?.supabaseUrl) return;
  try {
    projectInfo.value = await supabaseProjectInfo(id);
  } catch (e) {
    projectError.value = String(e);
    return;
  }
  try {
    buckets.value = await supabaseListBuckets(id);
  } catch {
    buckets.value = [];
  }
}

async function onSelectTable(t: TableInfo) {
  if (!activeId.value) return;
  activeTable.value = t;
  tab.value = "data";
  error.value = "";
  try {
    rows.value = await tableRows(activeId.value, t.schema, t.name);
  } catch (e) {
    error.value = String(e);
    rows.value = [];
  }
}

async function onRunQuery(sql: string) {
  if (!activeId.value) return;
  queryRunning.value = true;
  queryError.value = "";
  try {
    queryRows.value = await runQuery(activeId.value, sql);
  } catch (e) {
    queryError.value = String(e);
    queryRows.value = [];
  } finally {
    queryRunning.value = false;
  }
}

async function onSave(
  info: ConnectionInfo,
  password: string | undefined,
  serviceKey: string | undefined,
) {
  await saveConnection(info, password, serviceKey);
  await refreshConnections();
}

async function onRemove(id: string) {
  await deleteConnection(id);
  if (activeId.value === id) {
    activeId.value = null;
    tables.value = [];
    rows.value = [];
  }
  await refreshConnections();
}

async function onTest() {
  if (!activeId.value) return;
  try {
    status.value = await testConnection(activeId.value);
  } catch (e) {
    status.value = `connection failed: ${e}`;
  }
}

onMounted(refreshConnections);
</script>

<template>
  <div class="layout">
    <aside class="sidebar">
      <h1>pgcove</h1>
      <ConnectionList
        :connections="connections"
        :active-id="activeId"
        @select="onSelect"
        @remove="onRemove"
      />
      <ConnectionForm @save="onSave" />
      <h2 v-if="activeId">
        Tables
      </h2>
      <TableList
        v-if="activeId"
        :tables="tables"
        :active="activeTable"
        @select="onSelectTable"
      />
    </aside>
    <main class="main">
      <div class="toolbar">
        <button
          :class="{ active: tab === 'data' }"
          @click="tab = 'data'"
        >
          Data
        </button>
        <button
          :class="{ active: tab === 'query' }"
          @click="tab = 'query'"
        >
          Query
        </button>
        <button
          :class="{ active: tab === 'supabase' }"
          @click="tab = 'supabase'"
        >
          Supabase
        </button>
        <button
          :disabled="!activeId"
          @click="onTest"
        >
          Test connection
        </button>
        <span
          v-if="status"
          class="status"
        >{{ status }}</span>
        <span
          v-if="error"
          class="error"
        >{{ error }}</span>
      </div>
      <template v-if="activeId">
        <DataGrid
          v-if="tab === 'data'"
          :rows="rows"
        />
        <template v-else-if="tab === 'query'">
          <QueryEditor
            :running="queryRunning"
            @run="onRunQuery"
          />
          <p
            v-if="queryError"
            class="error"
          >
            {{ queryError }}
          </p>
          <DataGrid :rows="queryRows" />
        </template>
        <SupabasePanel
          v-else
          :policies="policies"
          :auth-users="authUsers"
          :auth-error="authError"
          :project-info="projectInfo"
          :buckets="buckets"
          :project-error="projectError"
        />
      </template>
      <p
        v-else
        class="hint"
      >
        Select or add a database connection to get started.
      </p>
    </main>
  </div>
</template>

<style>
:root {
  font-family: Inter, Avenir, Helvetica, Arial, sans-serif;
  font-size: 15px;
  color: #e8e8e8;
  background-color: #1c2024;
}
body {
  margin: 0;
}
button {
  border-radius: 6px;
  border: 1px solid transparent;
  padding: 0.4em 0.9em;
  font-family: inherit;
  color: inherit;
  background-color: #2b3138;
  cursor: pointer;
}
button:hover:not(:disabled) {
  border-color: #34a06f;
}
button.active {
  border-color: #34a06f;
}
button:disabled {
  opacity: 0.45;
  cursor: default;
}
input {
  border-radius: 6px;
  border: 1px solid #3a414b;
  padding: 0.4em 0.6em;
  font-family: inherit;
  color: inherit;
  background-color: #252a30;
}
</style>

<style scoped>
.layout {
  display: flex;
  height: 100vh;
}
.sidebar {
  width: 280px;
  border-right: 1px solid rgba(128, 128, 128, 0.25);
  overflow-y: auto;
  padding: 0.5rem;
}
.sidebar h1 {
  font-size: 1.1rem;
  margin: 0.3rem 0.5rem 0.8rem;
}
.sidebar h2 {
  font-size: 0.85rem;
  margin: 0.8rem 0.5rem 0.3rem;
  text-transform: uppercase;
  opacity: 0.7;
}
.main {
  flex: 1;
  display: flex;
  flex-direction: column;
  overflow: hidden;
}
.toolbar {
  display: flex;
  gap: 0.5rem;
  align-items: center;
  padding: 0.5rem;
  flex-wrap: wrap;
}
.status {
  font-size: 0.8rem;
  opacity: 0.8;
}
.error {
  color: #ff6b6b;
  font-size: 0.85rem;
}
.hint {
  padding: 1rem;
  opacity: 0.7;
}
</style>
