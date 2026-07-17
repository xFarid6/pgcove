<script setup lang="ts">
import { ref } from "vue";

const props = defineProps<{
  running: boolean;
}>();

const emit = defineEmits<{
  run: [sql: string];
}>();

const sql = ref("select * from ");

function run() {
  if (!sql.value.trim() || props.running) return;
  emit("run", sql.value);
}

function onKeydown(e: KeyboardEvent) {
  if ((e.metaKey || e.ctrlKey) && e.key === "Enter") {
    e.preventDefault();
    run();
  }
}
</script>

<template>
  <div class="query-editor">
    <textarea
      v-model="sql"
      class="sql-input"
      placeholder="select * from public.todos"
      spellcheck="false"
      @keydown="onKeydown"
    />
    <div class="toolbar">
      <button
        :disabled="running || !sql.trim()"
        @click="run"
      >
        {{ running ? "Running…" : "Run" }}
      </button>
      <span class="hint">⌘/Ctrl + Enter to run</span>
    </div>
  </div>
</template>

<style scoped>
.query-editor {
  display: flex;
  flex-direction: column;
  gap: 0.4rem;
  padding: 0.5rem;
  border-bottom: 1px solid rgba(128, 128, 128, 0.25);
}
.sql-input {
  width: 100%;
  min-height: 6rem;
  resize: vertical;
  font-family: ui-monospace, "SF Mono", Menlo, Consolas, monospace;
  font-size: 0.85rem;
  color: inherit;
  background-color: #252a30;
  border: 1px solid #3a414b;
  border-radius: 6px;
  padding: 0.5rem;
  box-sizing: border-box;
}
.toolbar {
  display: flex;
  align-items: center;
  gap: 0.6rem;
}
.hint {
  font-size: 0.75rem;
  opacity: 0.6;
}
</style>
