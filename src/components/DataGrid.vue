<script setup lang="ts">
import { computed } from "vue";
import type { Row } from "../api";

const props = defineProps<{
  rows: Row[];
}>();

const columns = computed(() =>
  props.rows.length > 0 ? Object.keys(props.rows[0]) : [],
);

function cell(v: unknown): string {
  if (v === null || v === undefined) return "∅";
  if (typeof v === "object") return JSON.stringify(v);
  return String(v);
}
</script>

<template>
  <div class="data-grid">
    <table v-if="rows.length > 0">
      <thead>
        <tr>
          <th
            v-for="c in columns"
            :key="c"
          >
            {{ c }}
          </th>
        </tr>
      </thead>
      <tbody>
        <tr
          v-for="(r, i) in rows"
          :key="i"
        >
          <td
            v-for="c in columns"
            :key="c"
            :class="{ null: r[c] === null }"
          >
            {{ cell(r[c]) }}
          </td>
        </tr>
      </tbody>
    </table>
    <p
      v-else
      class="empty"
    >
      No rows.
    </p>
  </div>
</template>

<style scoped>
.data-grid {
  overflow: auto;
  flex: 1;
}
table {
  border-collapse: collapse;
  font-size: 0.85rem;
  white-space: nowrap;
}
th,
td {
  text-align: left;
  padding: 0.3rem 0.6rem;
  border-bottom: 1px solid rgba(128, 128, 128, 0.25);
  max-width: 24rem;
  overflow: hidden;
  text-overflow: ellipsis;
}
td.null {
  opacity: 0.4;
}
.empty {
  opacity: 0.6;
  padding: 1rem;
}
</style>
