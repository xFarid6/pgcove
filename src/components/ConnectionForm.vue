<script setup lang="ts">
import { ref } from "vue";
import type { ConnectionInfo } from "../api";

const emit = defineEmits<{
  save: [info: ConnectionInfo, password: string | undefined];
}>();

const name = ref("");
const host = ref("localhost");
const port = ref(5432);
const user = ref("postgres");
const database = ref("postgres");
const password = ref("");

function submit() {
  if (!name.value.trim() || !host.value.trim()) return;
  emit(
    "save",
    {
      id: crypto.randomUUID(),
      name: name.value.trim(),
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
    <input
      v-model="name"
      placeholder="Name (e.g. supabase prod)"
    >
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
