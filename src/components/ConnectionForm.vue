<script setup lang="ts">
import { ref } from "vue";
import type { ConnectionInfo, DbKind } from "../api";

const emit = defineEmits<{
  save: [info: ConnectionInfo, password: string | undefined];
}>();

const kind = ref<DbKind>("postgres");
const name = ref("");
const host = ref("localhost");
const port = ref(5432);
const user = ref("postgres");
const database = ref("postgres");
const filePath = ref("");
const password = ref("");

function submit() {
  if (kind.value === "sqlite") {
    if (!name.value.trim() || !filePath.value.trim()) return;
    emit("save", {
      id: crypto.randomUUID(),
      name: name.value.trim(),
      kind: "sqlite",
      host: "",
      port: 0,
      user: "",
      database: filePath.value.trim(),
    }, undefined);
    name.value = "";
    filePath.value = "";
    return;
  }
  if (!name.value.trim() || !host.value.trim()) return;
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
    },
    password.value || undefined,
  );
  name.value = "";
  password.value = "";
}
</script>

<template>
  <form
    class="connection-form"
    @submit.prevent="submit"
  >
    <select v-model="kind">
      <option value="postgres">
        Postgres / Supabase
      </option>
      <option value="sqlite">
        SQLite
      </option>
    </select>
    <input
      v-model="name"
      placeholder="Name (e.g. supabase prod)"
    >
    <template v-if="kind === 'sqlite'">
      <input
        v-model="filePath"
        placeholder="File path (e.g. /home/me/app.db, or :memory:)"
      >
    </template>
    <template v-else>
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
