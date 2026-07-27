<script setup lang="ts">
import { ref, watch } from "vue";
import type { ConnectionInfo } from "../api";

const emit = defineEmits<{
  save: [info: ConnectionInfo, password: string | undefined, serviceKey: string | undefined];
}>();

/**
 * UI-level variant, not the driver kind: a Supabase project still connects
 * over Postgres, it just also carries a project URL + service-role key.
 */
type Variant = "supabase" | "postgres" | "sqlite";

const variant = ref<Variant>("supabase");
const name = ref("");
const host = ref("localhost");
const port = ref(5432);
const user = ref("postgres");
const database = ref("postgres");
const filePath = ref("");
const password = ref("");
const projectUrl = ref("");
const serviceKey = ref("");

/** `https://abcdefgh.supabase.co` -> `abcdefgh`; null for anything else. */
function projectRefOf(url: string): string | null {
  const hostname = url.trim().replace(/^https?:\/\//, "").split("/")[0];
  const [first, ...rest] = hostname.split(".");
  return first && rest.join(".") === "supabase.co" ? first : null;
}

// Typing the project URL fills in the direct database host Supabase gives
// every project — still editable, since pooler hosts differ per region.
watch(projectUrl, (url) => {
  const projRef = projectRefOf(url);
  if (!projRef) return;
  host.value = `db.${projRef}.supabase.co`;
  user.value = "postgres";
  port.value = 5432;
  database.value = "postgres";
});

function submit() {
  if (!name.value.trim()) return;
  if (variant.value === "sqlite") {
    if (!filePath.value.trim()) return;
    emit("save", {
      id: crypto.randomUUID(),
      name: name.value.trim(),
      kind: "sqlite",
      host: "",
      port: 0,
      user: "",
      database: filePath.value.trim(),
    }, undefined, undefined);
    name.value = "";
    filePath.value = "";
    return;
  }
  const isSupabase = variant.value === "supabase";
  if (isSupabase && !projectUrl.value.trim()) return;
  if (!host.value.trim()) return;
  emit(
    "save",
    {
      id: crypto.randomUUID(),
      name: name.value.trim(),
      kind: "postgres",
      host: host.value.trim(),
      port: port.value,
      user: user.value.trim(),
      database: database.value.trim(),
      ...(isSupabase ? { supabaseUrl: projectUrl.value.trim() } : {}),
    },
    password.value || undefined,
    isSupabase ? serviceKey.value || undefined : undefined,
  );
  name.value = "";
  password.value = "";
  serviceKey.value = "";
}
</script>

<template>
  <form
    class="connection-form"
    @submit.prevent="submit"
  >
    <select v-model="variant">
      <option value="supabase">
        Supabase project
      </option>
      <option value="postgres">
        Postgres
      </option>
      <option value="sqlite">
        SQLite
      </option>
    </select>
    <input
      v-model="name"
      placeholder="Name (e.g. supabase prod)"
    >
    <template v-if="variant === 'sqlite'">
      <input
        v-model="filePath"
        placeholder="File path (e.g. /home/me/app.db, or :memory:)"
      >
    </template>
    <template v-else>
      <template v-if="variant === 'supabase'">
        <input
          v-model="projectUrl"
          placeholder="Project URL (https://<ref>.supabase.co)"
        >
        <input
          v-model="serviceKey"
          type="password"
          placeholder="Service-role key (stored in OS keyring)"
        >
      </template>
      <input
        v-model="host"
        placeholder="Host (…pooler.supabase.com)"
      >
      <input
        v-model.number="port"
        type="number"
        placeholder="Port"
      >
      <input
        v-model="user"
        placeholder="User (postgres.<project-ref>)"
      >
      <input
        v-model="database"
        placeholder="Database"
      >
      <input
        v-model="password"
        type="password"
        placeholder="Password (stored in OS keyring)"
      >
    </template>
    <button type="submit">
      Add connection
    </button>
  </form>
</template>

<style scoped>
.connection-form {
  display: flex;
  flex-direction: column;
  gap: 0.4rem;
  padding: 0.5rem;
}
</style>
