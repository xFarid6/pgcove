<script setup lang="ts">
import type { ConnectionInfo } from "../api";

defineProps<{
  connections: ConnectionInfo[];
  activeId: string | null;
}>();

defineEmits<{
  select: [id: string];
  remove: [id: string];
}>();
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
