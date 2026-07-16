<script setup lang="ts">
import type { TableInfo } from "../api";

defineProps<{
  tables: TableInfo[];
  active: TableInfo | null;
}>();

defineEmits<{
  select: [table: TableInfo];
}>();
</script>

<template>
  <ul class="table-list">
    <li
      v-for="t in tables"
      :key="`${t.schema}.${t.name}`"
    >
      <button
        :class="{ active: active?.schema === t.schema && active?.name === t.name }"
        @click="$emit('select', t)"
      >
        <span class="schema">{{ t.schema }}.</span>{{ t.name }}
        <span
          v-if="t.kind === 'VIEW'"
          class="kind"
        >view</span>
      </button>
    </li>
    <li
      v-if="tables.length === 0"
      class="empty"
    >
      No tables.
    </li>
  </ul>
</template>

<style scoped>
.table-list {
  list-style: none;
  margin: 0;
  padding: 0;
  font-size: 0.85rem;
}
button {
  width: 100%;
  text-align: left;
  background: none;
  border: none;
  box-shadow: none;
  cursor: pointer;
  padding: 0.25rem 0.5rem;
}
button.active {
  font-weight: 700;
}
.schema {
  opacity: 0.55;
}
.kind {
  font-size: 0.7rem;
  opacity: 0.6;
  margin-left: 0.3rem;
}
.empty {
  opacity: 0.6;
  padding: 0.25rem 0.5rem;
}
</style>
