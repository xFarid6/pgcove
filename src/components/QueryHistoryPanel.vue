<script setup lang="ts">
import { onMounted, ref } from "vue";
import { listQueryHistory, deleteQueryFromHistory, clearQueryHistory, type QueryRecord } from "../api";

const emit = defineEmits<{
  load: [sql: string];
}>();

const history = ref<QueryRecord[]>([]);
const loading = ref(false);
const error = ref("");
const showConfirm = ref(false);

async function loadHistory() {
  loading.value = true;
  error.value = "";
  try {
    history.value = await listQueryHistory();
  } catch (e) {
    error.value = String(e);
  } finally {
    loading.value = false;
  }
}

async function onDelete(id: string) {
  try {
    await deleteQueryFromHistory(id);
    await loadHistory();
  } catch (e) {
    error.value = String(e);
  }
}

async function onClear() {
  try {
    await clearQueryHistory();
    await loadHistory();
    showConfirm.value = false;
  } catch (e) {
    error.value = String(e);
  }
}

function onLoad(sql: string) {
  emit("load", sql);
}

onMounted(() => {
  loadHistory();
});
</script>

<template>
  <div class="query-history-panel">
    <div class="panel-header">
      <h3>Query History</h3>
      <div class="header-actions">
        <button
          class="icon-btn"
          title="Refresh history"
          @click="loadHistory"
        >
          ⟳
        </button>
        <button
          v-if="history.length > 0"
          class="icon-btn danger"
          title="Clear all"
          @click="showConfirm = true"
        >
          ✕
        </button>
      </div>
    </div>

    <div
      v-if="showConfirm"
      class="confirm-dialog"
    >
      <p>Clear all query history?</p>
      <div class="confirm-actions">
        <button @click="onClear">
          Yes, clear
        </button>
        <button @click="showConfirm = false">
          Cancel
        </button>
      </div>
    </div>

    <p
      v-if="error"
      class="error"
    >
      {{ error }}
    </p>

    <div
      v-if="loading"
      class="loading"
    >
      Loading…
    </div>
    <div
      v-else-if="history.length === 0"
      class="empty"
    >
      No queries in history
    </div>
    <div
      v-else
      class="history-list"
    >
      <div
        v-for="record in history"
        :key="record.id"
        class="history-item"
      >
        <div class="item-header">
          <span class="sql-preview">{{ record.sql.substring(0, 50) }}{{ record.sql.length > 50 ? '…' : '' }}</span>
          <span class="connection-badge">{{ record.connectionId }}</span>
        </div>
        <div class="item-details">
          <span class="timestamp">{{ new Date(record.timestamp).toLocaleString() }}</span>
        </div>
        <div class="item-actions">
          <button
            class="action-btn"
            @click="onLoad(record.sql)"
          >
            Load
          </button>
          <button
            class="action-btn danger"
            @click="onDelete(record.id)"
          >
            Delete
          </button>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.query-history-panel {
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
  padding: 0.5rem;
  border: 1px solid rgba(128, 128, 128, 0.25);
  border-radius: 4px;
  background: rgba(0, 0, 0, 0.1);
}

.panel-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 0.5rem;
}

.panel-header h3 {
  margin: 0;
  font-size: 0.9rem;
  font-weight: 600;
  opacity: 0.8;
}

.header-actions {
  display: flex;
  gap: 0.3rem;
}

.icon-btn {
  padding: 0.2rem 0.4rem;
  border: 1px solid rgba(128, 128, 128, 0.3);
  background: rgba(255, 255, 255, 0.05);
  cursor: pointer;
  border-radius: 3px;
  font-size: 0.8rem;
}

.icon-btn:hover {
  background: rgba(255, 255, 255, 0.1);
}

.icon-btn.danger {
  color: #e74c3c;
}

.confirm-dialog {
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
  padding: 0.5rem;
  background: rgba(0, 0, 0, 0.3);
  border-radius: 3px;
}

.confirm-dialog p {
  margin: 0;
  font-size: 0.85rem;
}

.confirm-actions {
  display: flex;
  gap: 0.5rem;
}

.confirm-actions button {
  padding: 0.3rem 0.6rem;
  font-size: 0.8rem;
  cursor: pointer;
  border-radius: 3px;
}

.error {
  color: #e74c3c;
  font-size: 0.8rem;
  margin: 0;
}

.loading {
  text-align: center;
  font-size: 0.8rem;
  opacity: 0.6;
  padding: 0.5rem;
}

.empty {
  text-align: center;
  font-size: 0.8rem;
  opacity: 0.5;
  padding: 1rem 0.5rem;
}

.history-list {
  display: flex;
  flex-direction: column;
  gap: 0.4rem;
  max-height: 300px;
  overflow-y: auto;
}

.history-item {
  display: flex;
  flex-direction: column;
  gap: 0.3rem;
  padding: 0.4rem;
  background: rgba(255, 255, 255, 0.05);
  border: 1px solid rgba(128, 128, 128, 0.2);
  border-radius: 3px;
  font-size: 0.8rem;
}

.item-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 0.5rem;
}

.sql-preview {
  flex: 1;
  font-family: monospace;
  opacity: 0.8;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.connection-badge {
  flex-shrink: 0;
  padding: 0.1rem 0.3rem;
  background: rgba(100, 150, 255, 0.2);
  border-radius: 2px;
  font-size: 0.75rem;
  opacity: 0.7;
}

.item-details {
  display: flex;
  gap: 0.5rem;
  font-size: 0.75rem;
  opacity: 0.6;
}

.timestamp {
  font-size: 0.7rem;
}

.item-actions {
  display: flex;
  gap: 0.3rem;
}

.action-btn {
  padding: 0.2rem 0.4rem;
  font-size: 0.7rem;
  border: 1px solid rgba(128, 128, 128, 0.3);
  background: rgba(100, 150, 255, 0.1);
  cursor: pointer;
  border-radius: 2px;
}

.action-btn:hover {
  background: rgba(100, 150, 255, 0.2);
}

.action-btn.danger {
  background: rgba(231, 76, 60, 0.1);
  color: #e74c3c;
}

.action-btn.danger:hover {
  background: rgba(231, 76, 60, 0.2);
}
</style>
