<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import { erdData, type ErdData } from "../api";

const props = defineProps<{
  connectionId: string;
  schema: string;
}>();

const data = ref<ErdData | null>(null);
const loading = ref(false);
const error = ref("");

const tablePositions = computed(() => {
  if (!data.value) return new Map<string, { x: number; y: number }>();

  const positions = new Map<string, { x: number; y: number }>();
  const cols = Math.ceil(Math.sqrt(data.value.tables.length));
  const cellWidth = 250;
  const cellHeight = 150;

  data.value.tables.forEach((table, index) => {
    const col = index % cols;
    const row = Math.floor(index / cols);
    positions.set(table.name, {
      x: col * cellWidth + 20,
      y: row * cellHeight + 20,
    });
  });

  return positions;
});

const svgDimensions = computed(() => {
  if (!data.value || data.value.tables.length === 0) {
    return { width: 800, height: 600 };
  }

  const cols = Math.ceil(Math.sqrt(data.value.tables.length));
  const rows = Math.ceil(data.value.tables.length / cols);
  const width = cols * 250 + 40;
  const height = rows * 150 + 40;

  return { width: Math.max(800, width), height: Math.max(600, height) };
});

async function loadErd() {
  loading.value = true;
  error.value = "";

  try {
    data.value = await erdData(props.connectionId, props.schema);
  } catch (err) {
    error.value = err instanceof Error ? err.message : String(err);
  } finally {
    loading.value = false;
  }
}

onMounted(() => {
  loadErd();
});

function getTablePosition(tableName: string) {
  return tablePositions.value.get(tableName) || { x: 0, y: 0 };
}

function getLinkPath(fromTable: string, toTable: string) {
  const fromPos = getTablePosition(fromTable);
  const toPos = getTablePosition(toTable);

  const fromX = fromPos.x + 125;
  const fromY = fromPos.y + 75;
  const toX = toPos.x + 125;
  const toY = toPos.y + 75;

  return `M${fromX},${fromY} Q${(fromX + toX) / 2},${Math.max(fromY, toY) + 50} ${toX},${toY}`;
}
</script>

<template>
  <div class="erd-view">
    <button
      class="refresh-btn"
      :disabled="loading"
      @click="loadErd"
    >
      {{ loading ? "Loading..." : "Refresh" }}
    </button>

    <p
      v-if="error"
      class="error"
    >
      {{ error }}
    </p>

    <div
      v-else-if="data"
      class="erd-container"
    >
      <svg
        :width="svgDimensions.width"
        :height="svgDimensions.height"
        class="erd-svg"
      >
        <!-- Draw FK lines first so they appear behind tables -->
        <g class="relationships">
          <path
            v-for="fk in data.foreignKeys"
            :key="`${fk.fromTable}-${fk.fromColumn}-${fk.toTable}`"
            :d="getLinkPath(fk.fromTable, fk.toTable)"
            class="fk-line"
          />
        </g>

        <!-- Draw tables -->
        <g class="tables">
          <g
            v-for="table in data.tables"
            :key="table.name"
            :transform="`translate(${getTablePosition(table.name).x}, ${getTablePosition(table.name).y})`"
            class="table-box"
          >
            <rect
              width="230"
              height="130"
              rx="4"
              class="table-rect"
            />
            <text
              x="115"
              y="25"
              text-anchor="middle"
              class="table-name"
            >
              {{ table.name }}
            </text>
            <line
              x1="5"
              y1="35"
              x2="225"
              y2="35"
              class="divider"
            />
          </g>
        </g>
      </svg>
    </div>
  </div>
</template>

<style scoped>
.erd-view {
  display: flex;
  flex-direction: column;
  gap: 1rem;
  padding: 1rem;
}

.refresh-btn {
  align-self: flex-start;
  padding: 0.5rem 1rem;
  background: var(--color-primary);
  color: var(--color-text-inverse);
  border: none;
  border-radius: 4px;
  cursor: pointer;
  font-size: 0.875rem;
}

.refresh-btn:hover:not(:disabled) {
  background: var(--color-primary-dark);
}

.refresh-btn:disabled {
  opacity: 0.6;
  cursor: not-allowed;
}

.error {
  padding: 1rem;
  background: var(--color-error-bg);
  color: var(--color-error-text);
  border-radius: 4px;
  font-size: 0.875rem;
}

.erd-container {
  flex: 1;
  overflow: auto;
  border: 1px solid var(--color-border);
  border-radius: 4px;
  background: var(--color-background-secondary);
}

.erd-svg {
  display: block;
  min-width: 100%;
  min-height: 100%;
}

.fk-line {
  stroke: var(--color-fk-line);
  stroke-width: 2;
  fill: none;
  marker-end: url(#arrowhead);
}

.table-box {
  cursor: pointer;
}

.table-rect {
  fill: var(--color-table-bg);
  stroke: var(--color-table-border);
  stroke-width: 2;
}

.table-box:hover .table-rect {
  stroke: var(--color-table-border-hover);
  stroke-width: 3;
}

.table-name {
  font-size: 14px;
  font-weight: bold;
  fill: var(--color-text-primary);
}

.divider {
  stroke: var(--color-table-border);
  stroke-width: 1;
}

@media (prefers-color-scheme: dark) {
  :root {
    --color-primary: #3b82f6;
    --color-primary-dark: #2563eb;
    --color-text-inverse: #fff;
    --color-error-bg: #7f1d1d;
    --color-error-text: #fecaca;
    --color-border: #4b5563;
    --color-background-secondary: #1e293b;
    --color-fk-line: #94a3b8;
    --color-table-bg: #334155;
    --color-table-border: #64748b;
    --color-table-border-hover: #cbd5e1;
    --color-text-primary: #f1f5f9;
  }
}

@media (prefers-color-scheme: light) {
  :root {
    --color-primary: #3b82f6;
    --color-primary-dark: #2563eb;
    --color-text-inverse: #fff;
    --color-error-bg: #fee2e2;
    --color-error-text: #7f1d1d;
    --color-border: #d1d5db;
    --color-background-secondary: #f9fafb;
    --color-fk-line: #6b7280;
    --color-table-bg: #f3f4f6;
    --color-table-border: #9ca3af;
    --color-table-border-hover: #374151;
    --color-text-primary: #1f2937;
  }
}
</style>
