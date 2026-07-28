<script setup lang="ts">
import { ref, computed, onMounted, onBeforeUnmount } from "vue";
import { EditorView, basicSetup } from "codemirror";
import { keymap, lineNumbers } from "@codemirror/view";
import { EditorState } from "@codemirror/state";
import { sql } from "@codemirror/lang-sql";
import { indentWithTab } from "@codemirror/commands";
import { autocompletion, Completion, CompletionContext, type CompletionResult } from "@codemirror/autocomplete";
import type { TableInfo } from "../api";

const props = defineProps<{
  running: boolean;
  initialSql?: string;
  tables?: TableInfo[];
}>();

const emit = defineEmits<{
  run: [sql: string];
  update: [sql: string];
}>();

const editorContainer = ref<HTMLDivElement>();
const editor = ref<EditorView | null>(null);
const sqlContent = ref("");
const initialSql = props.initialSql ?? "select * from ";
const hasContent = computed(() => sqlContent.value.trim().length > 0);

function schemaCompletions(context: CompletionContext): CompletionResult | null {
  if (!props.tables || props.tables.length === 0) return null;

  const word = context.matchBefore(/\w*/);
  if (!word) return null;

  const prefix = word.text.toLowerCase();
  const options: Completion[] = [];

  for (const table of props.tables) {
    if (table.name.toLowerCase().startsWith(prefix)) {
      options.push({
        label: table.name,
        detail: `${table.schema}.${table.name}`,
        type: table.kind === "VIEW" ? "variable" : "class",
      });
    }
  }

  return {
    from: word.from,
    options,
  };
}

onMounted(() => {
  if (!editorContainer.value) return;

  sqlContent.value = initialSql;
  const state = EditorState.create({
    doc: initialSql,
    extensions: [
      basicSetup,
      sql(),
      lineNumbers(),
      keymap.of([
        {
          key: "Ctrl-Enter",
          run: () => {
            runQuery();
            return true;
          },
        },
        {
          key: "Cmd-Enter",
          run: () => {
            runQuery();
            return true;
          },
        },
        indentWithTab,
      ]),
      EditorView.updateListener.of((update) => {
        if (update.docChanged) {
          sqlContent.value = update.state.doc.toString();
          emit("update", sqlContent.value);
        }
      }),
      autocompletion({
        override: [schemaCompletions],
      }),
      EditorView.theme(
        {
          ".cm-content": {
            minHeight: "6rem",
            fontSize: "0.85rem",
          },
          ".cm-gutters": {
            backgroundColor: "#252a30",
            borderRight: "1px solid #3a414b",
          },
          ".cm-activeLineGutter": {
            backgroundColor: "#3a414b",
          },
        },
        { dark: true }
      ),
    ],
  });

  editor.value = new EditorView({
    state,
    parent: editorContainer.value,
  });
});

onBeforeUnmount(() => {
  try {
    editor.value?.destroy();
  } catch {
    // happy-dom's MutationObserver mock chokes on CodeMirror's teardown in
    // tests; a real webview's DOM implementation doesn't hit this.
  }
});

function runQuery() {
  if (!hasContent.value || props.running) return;
  emit("run", sqlContent.value);
}

function run() {
  runQuery();
}

defineExpose({ sqlContent, editor });
</script>

<template>
  <div class="query-editor">
    <div
      ref="editorContainer"
      class="editor-container"
    />
    <div class="toolbar">
      <button
        :disabled="running || !hasContent"
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
.editor-container {
  border: 1px solid #3a414b;
  border-radius: 6px;
  overflow: hidden;
}
.editor-container :deep(.cm-editor) {
  font-family: ui-monospace, "SF Mono", Menlo, Consolas, monospace;
  color: inherit;
  background-color: #252a30;
}
.editor-container :deep(.cm-cursor) {
  border-left-color: inherit;
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
