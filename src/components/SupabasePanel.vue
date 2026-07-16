<script setup lang="ts">
// Supabase-first panel: RLS policies (from the real pg_policies catalog) and
// auth.users. Management-API integration is issue #5, RLS editing issue #6,
// user admin issue #7.
import type { AuthUser, PolicyInfo } from "../api";

defineProps<{
  policies: PolicyInfo[];
  authUsers: AuthUser[];
  /** e.g. "auth.users not found — not a Supabase database?" */
  authError: string;
}>();
</script>

<template>
  <div class="supabase-panel">
    <section>
      <h2>RLS policies</h2>
      <table v-if="policies.length > 0">
        <thead>
          <tr>
            <th>Table</th>
            <th>Policy</th>
            <th>Command</th>
            <th>Roles</th>
            <th>Expression</th>
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
</style>
