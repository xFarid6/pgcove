<script setup lang="ts">
// Supabase-first panel: project info and storage buckets over the project's
// HTTP API (issue #5), plus RLS policies (from the real pg_policies catalog,
// editable per issue #6) and auth.users over SQL (user admin is issue #7).
import { ref } from "vue";
import type { AuthUser, PolicyDraft, PolicyInfo, StorageBucket, SupabaseProjectInfo } from "../api";

withDefaults(
  defineProps<{
    policies: PolicyInfo[];
    authUsers: AuthUser[];
    /** e.g. "auth.users not found — not a Supabase database?" */
    authError: string;
    /** null when this connection has no Supabase project URL. */
    projectInfo?: SupabaseProjectInfo | null;
    buckets?: StorageBucket[];
    /** Why the project URL + service-role key calls didn't return data. */
    projectError?: string;
    /** Error from the most recent policy/RLS DDL attempt, if any. */
    ddlError?: string;
  }>(),
  { projectInfo: null, buckets: () => [], projectError: "", ddlError: "" },
);

const emit = defineEmits<{
  "create-policy": [draft: PolicyDraft];
  "alter-policy": [draft: PolicyDraft];
  "drop-policy": [policy: PolicyInfo];
  "toggle-rls": [schema: string, table: string, enable: boolean];
}>();

const schema = ref("public");
const table = ref("");
const name = ref("");
const command = ref("ALL");
const roles = ref("");
const usingExpr = ref("");
const checkExpr = ref("");

function draft(): PolicyDraft {
  return {
    schema: schema.value.trim() || "public",
    table: table.value.trim(),
    name: name.value.trim(),
    command: command.value,
    roles: roles.value
      .split(",")
      .map((r) => r.trim())
      .filter(Boolean),
    usingExpr: usingExpr.value.trim() || undefined,
    checkExpr: checkExpr.value.trim() || undefined,
  };
}

function submitCreate() {
  if (!table.value.trim() || !name.value.trim()) return;
  emit("create-policy", draft());
}

function submitAlter() {
  if (!table.value.trim() || !name.value.trim()) return;
  emit("alter-policy", draft());
}

function toggleRls(enable: boolean) {
  if (!table.value.trim()) return;
  emit("toggle-rls", schema.value.trim() || "public", table.value.trim(), enable);
}
</script>

<template>
  <div class="supabase-panel">
    <section>
      <h2>Project</h2>
      <table v-if="projectInfo">
        <tbody>
          <tr>
            <th>Project ref</th>
            <td>{{ projectInfo.projectRef || "—" }}</td>
          </tr>
          <tr>
            <th>URL</th>
            <td class="expr">
              {{ projectInfo.url }}
            </td>
          </tr>
          <tr>
            <th>API</th>
            <td>{{ projectInfo.title }} — PostgREST {{ projectInfo.restVersion }}</td>
          </tr>
        </tbody>
      </table>
      <p
        v-else
        class="empty"
      >
        {{ projectError || "Not a Supabase project connection — add a project URL and service-role key to see project details." }}
      </p>
    </section>
    <section>
      <h2>Storage buckets</h2>
      <table v-if="buckets.length > 0">
        <thead>
          <tr>
            <th>Name</th>
            <th>Access</th>
            <th>Created</th>
          </tr>
        </thead>
        <tbody>
          <tr
            v-for="b in buckets"
            :key="b.id"
          >
            <td>{{ b.name || b.id }}</td>
            <td>{{ b.public ? "public" : "private" }}</td>
            <td>{{ b.createdAt }}</td>
          </tr>
        </tbody>
      </table>
      <p
        v-else
        class="empty"
      >
        No storage buckets.
      </p>
    </section>
    <section>
      <h2>Edge functions</h2>
      <!--
        Honest empty state rather than fake rows: listing edge functions is a
        Management API call (api.supabase.com) authenticated with a personal
        access token, which is a different credential from the service-role
        key this connection stores. Adding that token is the fast-follow to
        issue #5 — see src-tauri/src/supabase.rs.
      -->
      <p class="empty">
        Listing edge functions needs a Supabase management access token, not the
        project service-role key — a fast-follow to this feature.
      </p>
    </section>
    <section>
      <h2>RLS policies</h2>
      <form
        class="policy-form"
        @submit.prevent="submitCreate"
      >
        <div class="row">
          <input
            v-model="schema"
            placeholder="schema"
            size="10"
          >
          <input
            v-model="table"
            placeholder="table"
            required
          >
          <button
            type="button"
            @click="toggleRls(true)"
          >
            Enable RLS
          </button>
          <button
            type="button"
            @click="toggleRls(false)"
          >
            Disable RLS
          </button>
        </div>
        <div class="row">
          <input
            v-model="name"
            placeholder="policy name"
            required
          >
          <select v-model="command">
            <option>ALL</option>
            <option>SELECT</option>
            <option>INSERT</option>
            <option>UPDATE</option>
            <option>DELETE</option>
          </select>
          <input
            v-model="roles"
            placeholder="roles (comma-separated, blank = default)"
          >
        </div>
        <textarea
          v-model="usingExpr"
          class="expr-input"
          placeholder="USING expression"
        />
        <textarea
          v-model="checkExpr"
          class="expr-input"
          placeholder="WITH CHECK expression"
        />
        <div class="row">
          <button type="submit">
            Create policy
          </button>
          <button
            type="button"
            @click="submitAlter"
          >
            Alter policy (roles/using/check only)
          </button>
        </div>
      </form>
      <p
        v-if="ddlError"
        class="error"
      >
        {{ ddlError }}
      </p>
      <table v-if="policies.length > 0">
        <thead>
          <tr>
            <th>Table</th>
            <th>Policy</th>
            <th>Command</th>
            <th>Roles</th>
            <th>Expression</th>
            <th />
          </tr>
        </thead>
        <tbody>
          <tr
            v-for="p in policies"
            :key="`${p.schema}.${p.table}.${p.name}`"
          >
            <td>{{ p.schema }}.{{ p.table }}</td>
            <td>{{ p.name }}</td>
            <td>{{ p.command }}</td>
            <td>{{ p.roles }}</td>
            <td class="expr">
              {{ p.expression }}
            </td>
            <td>
              <button
                type="button"
                @click="emit('drop-policy', p)"
              >
                Drop
              </button>
            </td>
          </tr>
        </tbody>
      </table>
      <p
        v-else
        class="empty"
      >
        No RLS policies in this database.
      </p>
    </section>
    <section>
      <h2>Auth users</h2>
      <p
        v-if="authError"
        class="empty"
      >
        {{ authError }}
      </p>
      <table v-else-if="authUsers.length > 0">
        <thead>
          <tr>
            <th>Email</th>
            <th>ID</th>
            <th>Created</th>
          </tr>
        </thead>
        <tbody>
          <tr
            v-for="u in authUsers"
            :key="u.id"
          >
            <td>{{ u.email }}</td>
            <td class="expr">
              {{ u.id }}
            </td>
            <td>{{ u.createdAt }}</td>
          </tr>
        </tbody>
      </table>
      <p
        v-else
        class="empty"
      >
        No auth users.
      </p>
    </section>
  </div>
</template>

<style scoped>
.supabase-panel {
  padding: 0.5rem;
  overflow: auto;
  flex: 1;
}
h2 {
  font-size: 0.95rem;
}
table {
  border-collapse: collapse;
  font-size: 0.85rem;
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
.empty {
  opacity: 0.6;
}
.error {
  color: #e06c75;
}
.policy-form {
  display: flex;
  flex-direction: column;
  gap: 0.4rem;
  margin-bottom: 0.6rem;
  max-width: 32rem;
}
.policy-form .row {
  display: flex;
  gap: 0.4rem;
}
.expr-input {
  font-family: ui-monospace, "SF Mono", Menlo, Consolas, monospace;
  font-size: 0.8rem;
  min-height: 2.2rem;
  resize: vertical;
}
</style>
