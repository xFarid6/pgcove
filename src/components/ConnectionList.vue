<script setup lang="ts">
import { ref } from "vue";
import { invoke } from "@tauri-apps/api/core";
import type { ConnectionInfo } from "../api";

defineProps<{
  connections: ConnectionInfo[];
  activeId: string | null;
}>();

defineEmits<{
  select: [id: string];
  remove: [id: string];
}>();

const pingState = ref<{ [key: string]: "ok" | "failed" | "checking" }>({});

const ping = async (id: string) => {
  pingState.value[id] = "checking";
  try {
    await invoke("ping_connection", { id });
    pingState.value[id] = "ok";
  } catch {
    pingState.value[id] = "failed";
  }
};

const getPingIcon = (status: string | undefined) => {
  switch (status) {
    case "ok": return "✓";
    case "failed": return "✗";
    case "checking": return "…";
    default: return "○";
  }
};
</script>

<template>
  <ul class="connection-list">
    <li
      v-for="c in connections"
      :key="c.id"
      :class="{ active: c.id === activeId }"
    >
      <button
        class="name"
        @click="$emit('select', c.id)"
      >
        {{ c.name }}
        <span
          v-if="c.kind === 'sqlite'"
          class="detail"
        >{{ c.database }}</span>
        <span
          v-else
          class="detail"
        >{{ c.user }}@{{ c.host }}:{{ c.port }}/{{ c.database }}</span>
      </button>
      <button
        class="ping"
        :class="pingState[c.id]"
        :title="pingState[c.id] ? `Connection ${pingState[c.id]}` : 'Check connection health'"
        :disabled="pingState[c.id] === 'checking'"
        @click="ping(c.id)"
      >
        {{ getPingIcon(pingState[c.id]) }}
      </button>
      <button
        class="remove"
        title="Delete connection"
        @click="$emit('remove', c.id)"
      >
        ✕
      </button>
    </li>
    <li
      v-if="connections.length === 0"
      class="empty"
    >
      No connections yet.
    </li>
  </ul>
</template>

<style scoped>
.connection-list {
  list-style: none;
  margin: 0;
  padding: 0;
}
.connection-list li {
  display: flex;
  align-items: center;
  gap: 0.25rem;
}
.connection-list li.active .name {
  font-weight: 700;
}
.name {
  flex: 1;
  text-align: left;
  background: none;
  border: none;
  box-shadow: none;
  cursor: pointer;
  padding: 0.4rem 0.5rem;
}
.detail {
  display: block;
  font-size: 0.72rem;
  opacity: 0.6;
  word-break: break-all;
}
.ping {
  background: none;
  border: none;
  box-shadow: none;
  cursor: pointer;
  opacity: 0.5;
  min-width: 1.5rem;
  text-align: center;
  color: inherit;
}
.ping:hover:not(:disabled) {
  opacity: 1;
}
.ping:disabled {
  cursor: wait;
}
.ping.ok {
  color: #22c55e;
  opacity: 1;
}
.ping.failed {
  color: #ef4444;
  opacity: 1;
}
.ping.checking {
  opacity: 0.7;
}
.remove {
  background: none;
  border: none;
  box-shadow: none;
  cursor: pointer;
  opacity: 0.5;
}
.remove:hover {
  opacity: 1;
}
.empty {
  opacity: 0.6;
  padding: 0.4rem 0.5rem;
}
</style>
