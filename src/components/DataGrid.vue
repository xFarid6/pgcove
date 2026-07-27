<script setup lang="ts">
import { ref, watch } from "vue";
import type { Row } from "../api";

const props = withDefaults(
  defineProps<{
    rows: Row[];
    /** Show the pager/filter controls — off by default for non-paginated grids. */
    pageable?: boolean;
    page?: number;
    pageSize?: number;
    approxTotal?: number;
    sortColumn?: string;
    sortDesc?: boolean;
  }>(),
  { pageable: false, page: 1, pageSize: 50, approxTotal: 0, sortColumn: "", sortDesc: false },
);

const emit = defineEmits<{
  sort: [column: string];
  page: [page: number];
  filter: [column: string, value: string];
}>();

// Cached rather than a plain computed so the header/filter list survives a
// filter that returns zero rows.
const columns = ref<string[]>([]);
watch(
  () => props.rows,
  (rows) => {
    if (rows.length > 0) columns.value = Object.keys(rows[0]);
  },
  { immediate: true },
);

const filterColumn = ref("");
const filterValue = ref("");
function submitFilter() {
  emit("filter", filterColumn.value, filterValue.value);
}

function pageCount(): number {
  return Math.max(1, Math.ceil(props.approxTotal / props.pageSize));
}

function cell(v: unknown): string {
  if (v === null || v === undefined) return "∅";
  if (typeof v === "object") return JSON.stringify(v);
  return String(v);
}
</script>

<template>
  <div class="data-grid">
    <div
      v-if="pageable"
      class="controls"
    >
      <input
        v-model="filterColumn"
        list="dg-columns"
        placeholder="filter column"
      >
      <datalist id="dg-columns">
        <option
          v-for="c in columns"
          :key="c"
          :value="c"
        />
      </datalist>
      <input
        v-model="filterValue"
        placeholder="filter value"
        @keyup.enter="submitFilter"
      >
      <button
        type="button"
        @click="submitFilter"
      >
        Filter
      </button>
      <button
        type="button"
        :disabled="page <= 1"
        @click="emit('page', page - 1)"
      >
        Prev
      </button>
      <span>Page {{ page }} / {{ pageCount() }}</span>
      <button
        type="button"
        :disabled="page >= pageCount()"
        @click="emit('page', page + 1)"
      >
        Next
      </button>
    </div>
    <table v-if="rows.length > 0">
      <thead>
        <tr>
          <th
            v-for="c in columns"
            :key="c"
            class="sortable"
            @click="emit('sort', c)"
          >
            {{ c }}<span v-if="c === sortColumn">{{ sortDesc ? " ▼" : " ▲" }}</span>
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
.controls {
  display: flex;
  gap: 0.4rem;
  align-items: center;
  padding: 0.4rem;
  font-size: 0.85rem;
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
th.sortable {
  cursor: pointer;
  user-select: none;
}
td.null {
  opacity: 0.4;
}
.empty {
  opacity: 0.6;
  padding: 1rem;
}
</style>
