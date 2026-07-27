<script setup lang="ts">
// Supabase-first panel: project info and storage buckets over the project's
// HTTP API (issue #5), plus RLS policies (from the real pg_policies catalog)
// and auth.users over SQL. RLS editing is issue #6, user admin issue #7.
import type { AuthUser, PolicyInfo, StorageBucket, SupabaseProjectInfo } from "../api";

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
  }>(),
  { projectInfo: null, buckets: () => [], projectError: "" },
);
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
