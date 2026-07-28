<script setup lang="ts">
import { ref, computed } from "vue";
import QueryEditor from "./QueryEditor.vue";
import type { TableInfo } from "../api";

interface Tab {
  id: string;
  name: string;
  sql: string;
}

defineProps<{
  running: boolean;
  tables?: TableInfo[];
}>();

const emit = defineEmits<{
  run: [sql: string];
}>();

const tabs = ref<Tab[]>([
  { id: "1", name: "Query 1", sql: "select * from " }
]);
const activeTabId = ref("1");
let nextTabId = 2;

const activeTab = computed(() => tabs.value.find(t => t.id === activeTabId.value));

function addTab() {
  const newId = String(nextTabId++);
  tabs.value.push({
    id: newId,
    name: `Query ${tabs.value.length + 1}`,
    sql: "select * from "
  });
  activeTabId.value = newId;
}

function removeTab(id: string) {
  if (tabs.value.length === 1) return;

  const index = tabs.value.findIndex(t => t.id === id);
  if (index === -1) return;

  tabs.value.splice(index, 1);

  if (activeTabId.value === id) {
    activeTabId.value = tabs.value[Math.min(index, tabs.value.length - 1)].id;
  }
}

function updateTabSql(sql: string) {
  const tab = activeTab.value;
  if (tab) {
    tab.sql = sql;
  }
}

function onRun(sql: string) {
  emit("run", sql);
}

function renameTab(id: string) {
  const tab = tabs.value.find(t => t.id === id);
  if (!tab) return;

  const newName = window.prompt("Tab name:", tab.name);
  if (newName && newName.trim()) {
    tab.name = newName.trim();
  }
}

function loadQuery(sql: string) {
  const tab = activeTab.value;
  if (tab) {
    tab.sql = sql;
  }
}

defineExpose({ loadQuery });
</script>

<template>
  <div class="query-editor-tabs">
    <div class="tabs-bar">
      <button
        v-for="tab in tabs"
        :key="tab.id"
        :class="['tab', { active: tab.id === activeTabId }]"
        @click="activeTabId = tab.id"
      >
        <span
          class="tab-name"
          @dblclick.stop="renameTab(tab.id)"
        >{{ tab.name }}</span>
        <button
          v-if="tabs.length > 1"
          class="tab-close"
          title="Close tab"
          @click.stop="removeTab(tab.id)"
        >
          ✕
        </button>
      </button>
      <button
        class="add-tab"
        title="Add new query tab"
        @click="addTab"
      >
        +
      </button>
    </div>
    <div
      v-if="activeTab"
      class="editor-wrapper"
    >
      <QueryEditor
        :key="activeTab.id"
        :running="running"
        :initial-sql="activeTab.sql"
        :tables="tables"
        @run="onRun"
        @update="updateTabSql"
      />
    </div>
  </div>
</template>

<style scoped>
.query-editor-tabs {
  display: flex;
  flex-direction: column;
  height: 100%;
  border-bottom: 1px solid rgba(128, 128, 128, 0.25);
}

.tabs-bar {
  display: flex;
  gap: 0.2rem;
  padding: 0.3rem 0.5rem;
  border-bottom: 1px solid rgba(128, 128, 128, 0.25);
  background-color: #1c2024;
  align-items: center;
}

.tab {
  display: flex;
  align-items: center;
  gap: 0.4rem;
  padding: 0.3em 0.6em;
  background-color: #2b3138;
  border: 1px solid transparent;
  border-radius: 4px 4px 0 0;
  cursor: pointer;
  font-size: 0.9rem;
  color: #a0a0a0;
  transition: all 0.2s;
}

.tab:hover:not(.active) {
  background-color: #34414b;
}

.tab.active {
  background-color: #252a30;
  color: #e8e8e8;
  border-bottom-color: #252a30;
}

.tab-name {
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  max-width: 150px;
}

.tab-close {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 1.2rem;
  height: 1.2rem;
  padding: 0;
  border: none;
  background-color: transparent;
  color: inherit;
  cursor: pointer;
  font-size: 0.8rem;
  opacity: 0.6;
  transition: opacity 0.2s;
}

.tab-close:hover {
  opacity: 1;
}

.add-tab {
  padding: 0.3em 0.6em;
  margin-left: auto;
  font-weight: bold;
}

.editor-wrapper {
  flex: 1;
  overflow: hidden;
}
</style>
