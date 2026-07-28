<script setup lang="ts">
import { ref, watch } from "vue";
import type { AppSettings } from "../api";

const props = defineProps<{
  settings: AppSettings;
}>();

const emit = defineEmits<{
  update: [updates: Partial<AppSettings>];
  close: [];
}>();

const theme = ref<"light" | "dark" | "system">(props.settings.theme);
const defaultRowLimit = ref(props.settings.defaultRowLimit);
const defaultStatementTimeout = ref(props.settings.defaultStatementTimeout);

watch(() => props.settings, (newSettings) => {
  theme.value = newSettings.theme;
  defaultRowLimit.value = newSettings.defaultRowLimit;
  defaultStatementTimeout.value = newSettings.defaultStatementTimeout;
});

function handleSave() {
  emit("update", {
    theme: theme.value,
    defaultRowLimit: defaultRowLimit.value,
    defaultStatementTimeout: defaultStatementTimeout.value,
  });
  emit("close");
}

function handleCancel() {
  emit("close");
}
</script>

<template>
  <div
    class="modal-overlay"
    @click.self="handleCancel"
  >
    <div class="modal">
      <h2>Settings</h2>
      <div class="settings-form">
        <div class="setting">
          <label for="theme">Theme</label>
          <select
            id="theme"
            v-model="theme"
          >
            <option value="light">
              Light
            </option>
            <option value="dark">
              Dark
            </option>
            <option value="system">
              System
            </option>
          </select>
        </div>
        <div class="setting">
          <label for="row-limit">Default Row Limit</label>
          <input
            id="row-limit"
            v-model.number="defaultRowLimit"
            type="number"
            min="1"
            max="10000"
          >
        </div>
        <div class="setting">
          <label for="timeout">Statement Timeout (seconds)</label>
          <input
            id="timeout"
            v-model.number="defaultStatementTimeout"
            type="number"
            min="1"
            max="3600"
          >
        </div>
      </div>
      <div class="modal-actions">
        <button @click="handleCancel">
          Cancel
        </button>
        <button
          class="primary"
          @click="handleSave"
        >
          Save
        </button>
      </div>
    </div>
  </div>
</template>

<style scoped>
.modal-overlay {
  position: fixed;
  top: 0;
  left: 0;
  right: 0;
  bottom: 0;
  background-color: rgba(0, 0, 0, 0.5);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 1000;
}

.modal {
  background-color: var(--bg-color);
  color: var(--text-color);
  border: 1px solid var(--border-color);
  border-radius: 8px;
  padding: 2rem;
  max-width: 400px;
  width: 90%;
  box-shadow: 0 4px 12px rgba(0, 0, 0, 0.3);
}

.modal h2 {
  margin-top: 0;
  margin-bottom: 1.5rem;
  font-size: 1.3rem;
}

.settings-form {
  display: flex;
  flex-direction: column;
  gap: 1.5rem;
  margin-bottom: 2rem;
}

.setting {
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
}

.setting label {
  font-weight: 500;
  font-size: 0.9rem;
}

.setting input,
.setting select {
  padding: 0.5em 0.6em;
  border-radius: 4px;
  border: 1px solid var(--border-color);
  background-color: var(--input-bg);
  color: var(--text-color);
  font-family: inherit;
}

.modal-actions {
  display: flex;
  gap: 1rem;
  justify-content: flex-end;
}

.modal-actions button {
  min-width: 100px;
}

.modal-actions .primary {
  background-color: var(--accent-color);
  color: white;
}

.modal-actions .primary:hover {
  opacity: 0.9;
}
</style>
