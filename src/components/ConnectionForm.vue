<script setup lang="ts">
import { ref, watch } from "vue";
import { open } from "@tauri-apps/plugin-dialog";
import type { ConnectionInfo, SshTunnelConfig } from "../api";

const emit = defineEmits<{
  save: [
    info: ConnectionInfo,
    password: string | undefined,
    serviceKey: string | undefined,
    sshSecret: string | undefined,
  ];
}>();

/**
 * UI-level variant, not the driver kind: a Supabase project still connects
 * over Postgres, it just also carries a project URL + service-role key.
 */
type Variant = "supabase" | "postgres" | "mysql" | "sqlite";

const variant = ref<Variant>("supabase");
const DEFAULT_PORTS: Record<"postgres" | "mysql", number> = { postgres: 5432, mysql: 3306 };

const name = ref("");
const host = ref("localhost");
const port = ref(DEFAULT_PORTS.postgres);
const user = ref("postgres");
const database = ref("postgres");
const filePath = ref("");
const password = ref("");
const projectUrl = ref("");
const serviceKey = ref("");

// SSH tunnel (issue #11) — only offered for postgres/supabase, not sqlite
// (a local file has nothing to tunnel to).
const sshEnabled = ref(false);
const sshHost = ref("");
const sshPort = ref(22);
const sshUser = ref("");
const sshAuthMethod = ref<"agent" | "key" | "password">("key");
const sshKeyPath = ref("");
const sshSecret = ref("");

/** `https://abcdefgh.supabase.co` -> `abcdefgh`; null for anything else. */
function projectRefOf(url: string): string | null {
  const hostname = url.trim().replace(/^https?:\/\//, "").split("/")[0];
  const [first, ...rest] = hostname.split(".");
  return first && rest.join(".") === "supabase.co" ? first : null;
}

// Typing the project URL fills in the direct database host Supabase gives
// every project — still editable, since pooler hosts differ per region.
watch(projectUrl, (url) => {
  const projRef = projectRefOf(url);
  if (!projRef) return;
  host.value = `db.${projRef}.supabase.co`;
  user.value = "postgres";
  port.value = 5432;
  database.value = "postgres";
});

/** `undefined` when the SSH section isn't filled in enough to tunnel with. */
function sshTunnelConfig(): SshTunnelConfig | undefined {
  if (!sshEnabled.value || !sshHost.value.trim() || !sshUser.value.trim()) return undefined;
  return {
    host: sshHost.value.trim(),
    port: sshPort.value,
    user: sshUser.value.trim(),
    auth:
      sshAuthMethod.value === "agent"
        ? { method: "agent" }
        : sshAuthMethod.value === "key"
          ? { method: "key", keyPath: sshKeyPath.value.trim() }
          : { method: "password" },
  };
}

function resetSshFields() {
  sshEnabled.value = false;
  sshHost.value = "";
  sshPort.value = 22;
  sshUser.value = "";
  sshKeyPath.value = "";
  sshSecret.value = "";
}

async function browseSqliteFile() {
  try {
    const selected = await open({
      filters: [
        { name: "SQLite", extensions: ["db", "sqlite"] },
        { name: "All Files", extensions: ["*"] },
      ],
    });
    if (selected && typeof selected === "string") {
      filePath.value = selected;
    }
  } catch {
    // User cancelled the dialog or an error occurred, fall back to manual entry
  }
}

// Only follow the engine's default port while the field still holds *a*
// default — once the user types their own port, stop overwriting it.
function onVariantChange() {
  if (variant.value !== "postgres" && variant.value !== "mysql") return;
  if (Object.values(DEFAULT_PORTS).includes(port.value)) {
    port.value = DEFAULT_PORTS[variant.value];
  }
}

function submit() {
  if (!name.value.trim()) return;
  const sshTunnel = variant.value === "sqlite" ? undefined : sshTunnelConfig();
  if (variant.value === "sqlite") {
    if (!filePath.value.trim()) return;
    emit("save", {
      id: crypto.randomUUID(),
      name: name.value.trim(),
      kind: "sqlite",
      host: "",
      port: 0,
      user: "",
      database: filePath.value.trim(),
    }, undefined, undefined, undefined);
    name.value = "";
    filePath.value = "";
    return;
  }
  const isSupabase = variant.value === "supabase";
  if (isSupabase && !projectUrl.value.trim()) return;
  if (!host.value.trim()) return;
  emit(
    "save",
    {
      id: crypto.randomUUID(),
      name: name.value.trim(),
      kind: isSupabase ? "postgres" : (variant.value as "postgres" | "mysql"),
      host: host.value.trim(),
      port: port.value,
      user: user.value.trim(),
      database: database.value.trim(),
      ...(isSupabase ? { supabaseUrl: projectUrl.value.trim() } : {}),
      ...(sshTunnel ? { sshTunnel } : {}),
    },
    password.value || undefined,
    isSupabase ? serviceKey.value || undefined : undefined,
    sshTunnel ? sshSecret.value || undefined : undefined,
  );
  name.value = "";
  password.value = "";
  serviceKey.value = "";
  resetSshFields();
}
</script>

<template>
  <form
    class="connection-form"
    @submit.prevent="submit"
  >
    <select
      v-model="variant"
      @change="onVariantChange"
    >
      <option value="supabase">
        Supabase project
      </option>
      <option value="postgres">
        PostgreSQL
      </option>
      <option value="mysql">
        MySQL
      </option>
      <option value="sqlite">
        SQLite
      </option>
    </select>
    <input
      v-model="name"
      placeholder="Name (e.g. supabase prod)"
    >
    <template v-if="variant === 'sqlite'">
      <div class="file-picker-row">
        <input
          v-model="filePath"
          placeholder="File path (e.g. /home/me/app.db, or :memory:)"
        >
        <button
          type="button"
          @click="browseSqliteFile"
        >
          Browse…
        </button>
      </div>
    </template>
    <template v-else>
      <template v-if="variant === 'supabase'">
        <input
          v-model="projectUrl"
          placeholder="Project URL (https://<ref>.supabase.co)"
        >
        <input
          v-model="serviceKey"
          type="password"
          placeholder="Service-role key (stored in OS keyring)"
        >
      </template>
      <input
        v-model="host"
        placeholder="Host (…pooler.supabase.com)"
      >
      <input
        v-model.number="port"
        type="number"
        placeholder="Port"
      >
      <input
        v-model="user"
        placeholder="User (postgres.<project-ref>)"
      >
      <input
        v-model="database"
        placeholder="Database"
      >
      <input
        v-model="password"
        type="password"
        placeholder="Password (stored in OS keyring)"
      >
      <details class="ssh-section">
        <summary>
          <input
            v-model="sshEnabled"
            type="checkbox"
            @click.stop
          >
          Connect through an SSH tunnel
        </summary>
        <template v-if="sshEnabled">
          <input
            v-model="sshHost"
            placeholder="SSH host (bastion)"
          >
          <input
            v-model.number="sshPort"
            type="number"
            placeholder="SSH port"
          >
          <input
            v-model="sshUser"
            placeholder="SSH user"
          >
          <select v-model="sshAuthMethod">
            <option value="agent">
              SSH agent
            </option>
            <option value="key">
              Private key
            </option>
            <option value="password">
              Password
            </option>
          </select>
          <input
            v-if="sshAuthMethod === 'key'"
            v-model="sshKeyPath"
            placeholder="Private key path (e.g. ~/.ssh/id_ed25519)"
          >
          <input
            v-if="sshAuthMethod !== 'agent'"
            v-model="sshSecret"
            type="password"
            :placeholder="sshAuthMethod === 'key' ? 'Key passphrase, if any (stored in OS keyring)' : 'SSH password (stored in OS keyring)'"
          >
        </template>
      </details>
    </template>
    <button type="submit">
      Add connection
    </button>
  </form>
</template>

<style scoped>
.connection-form {
  display: flex;
  flex-direction: column;
  gap: 0.4rem;
  padding: 0.5rem;
}

.file-picker-row {
  display: flex;
  gap: 0.4rem;
}

.file-picker-row input {
  flex: 1;
  min-width: 0;
}

.file-picker-row button {
  flex-shrink: 0;
}
</style>
