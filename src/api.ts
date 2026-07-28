// Typed wrappers over the Tauri IPC surface (src-tauri/src/commands.rs).

import { invoke } from "@tauri-apps/api/core";

export type DbKind = "postgres" | "sqlite" | "mysql";

/**
 * SSH tunnel (issue #11): `host`/`port` on `ConnectionInfo` stay the DB's
 * address as reachable *from this bastion* (often `127.0.0.1:5432` for a DB
 * bound to localhost on a remote box) — pgcove opens a local port forward
 * through it and connects to that instead. The key passphrase or SSH
 * password is passed to saveConnection separately and kept in the keyring.
 */
export interface SshTunnelConfig {
  host: string;
  port: number;
  user: string;
  auth: { method: "agent" } | { method: "key"; keyPath: string } | { method: "password" };
}

export interface ConnectionInfo {
  id: string;
  name: string;
  /** Defaults to "postgres" server-side if omitted (pre-SQLite/MySQL saves). */
  kind?: DbKind;
  /** Ignored for "sqlite" — only `database` (the file path, or ":memory:") is used. */
  host: string;
  port: number;
  user: string;
  database: string;
  /**
   * `https://<project-ref>.supabase.co` when this connection is a Supabase
   * project — enables the Management/data API calls below. The matching
   * service-role key is passed to saveConnection and kept in the OS keyring.
   */
  supabaseUrl?: string;
  /** Present when this connection reaches its DB through an SSH tunnel. */
  sshTunnel?: SshTunnelConfig;
}

export interface TableInfo {
  schema: string;
  name: string;
  /** "BASE TABLE" | "VIEW" | ... */
  kind: string;
}

export interface PolicyInfo {
  schema: string;
  table: string;
  name: string;
  command: string;
  roles: string;
  expression: string;
}

export interface PolicyDraft {
  schema: string;
  table: string;
  name: string;
  /** SELECT | INSERT | UPDATE | DELETE | ALL */
  command: string;
  roles: string[];
  usingExpr?: string;
  checkExpr?: string;
}

export interface AuthUser {
  id: string;
  email: string;
  createdAt: string;
}

/** A GoTrue admin user row (issue #7) — search/ban/delete via the Management API. */
export interface AdminUser {
  id: string;
  email: string;
  createdAt: string;
  bannedUntil?: string;
}

export interface SupabaseProjectInfo {
  /** Empty for self-hosted/custom-domain projects. */
  projectRef: string;
  url: string;
  title: string;
  description: string;
  restVersion: string;
}

export interface StorageBucket {
  id: string;
  name: string;
  public: boolean;
  createdAt: string;
  updatedAt: string;
}

/** An edge function as returned by the Management API (issue #30). */
export interface EdgeFunction {
  id: string;
  name: string;
  slug: string;
  status: string;
  createdAt: string;
  updatedAt: string;
}

/** A table row as serialized by Postgres row_to_json. */
export type Row = Record<string, unknown>;

export interface RowsQuery {
  page: number;
  pageSize: number;
  orderBy?: string;
  orderDesc: boolean;
  filterColumn?: string;
  filterValue?: string;
}

export interface RowsPage {
  rows: Row[];
  /** An estimate (pg_class.reltuples), not an exact count. */
  approxTotal: number;
}

export interface ColumnInfo {
  name: string;
  dataType: string;
  nullable: boolean;
  default?: string;
}

export interface IndexInfo {
  name: string;
  definition: string;
  isUnique: boolean;
  isPrimary: boolean;
}

export interface ConstraintInfo {
  name: string;
  /** PRIMARY KEY | FOREIGN KEY | UNIQUE | CHECK */
  kind: string;
  columns: string;
  foreignTable?: string;
}

export interface TableStructure {
  columns: ColumnInfo[];
  indexes: IndexInfo[];
  constraints: ConstraintInfo[];
}

export const listConnections = () =>
  invoke<ConnectionInfo[]>("list_connections");

export const saveConnection = (
  info: ConnectionInfo,
  password?: string,
  serviceKey?: string,
  sshSecret?: string,
  mgmtToken?: string,
) =>
  invoke<void>("save_connection", {
    info,
    password: password ?? null,
    serviceKey: serviceKey ?? null,
    sshSecret: sshSecret ?? null,
    mgmtToken: mgmtToken ?? null,
  });

export const deleteConnection = (id: string) =>
  invoke<void>("delete_connection", { id });

export const testConnection = (id: string) =>
  invoke<string>("test_connection", { id });

export const listTables = (connectionId: string) =>
  invoke<TableInfo[]>("list_tables", { connectionId });

export const tableRows = (connectionId: string, schema: string, table: string, query: RowsQuery) =>
  invoke<RowsPage>("table_rows", { connectionId, schema, table, query });

export const primaryKeyColumns = (connectionId: string, schema: string, table: string) =>
  invoke<string[]>("primary_key_columns", { connectionId, schema, table });

/** `pk` maps each primary-key column to its current value (as text). */
export const updateCell = (
  connectionId: string,
  schema: string,
  table: string,
  pk: Record<string, string | null>,
  column: string,
  value: string | null,
) => invoke<void>("update_cell", { connectionId, schema, table, pk, column, value });

/** Run a single SELECT-shaped statement; other statement kinds are a follow-up. */
export const runQuery = (connectionId: string, sql: string) =>
  invoke<Row[]>("run_query", { connectionId, sql });

export const tableStructure = (connectionId: string, schema: string, table: string) =>
  invoke<TableStructure>("table_structure", { connectionId, schema, table });

export const listPolicies = (connectionId: string) =>
  invoke<PolicyInfo[]>("list_policies", { connectionId });

/** Preview commands — pure string generation, no DB round-trip. */
export const createPolicySql = (draft: PolicyDraft) =>
  invoke<string>("create_policy_sql", { draft });

export const alterPolicySql = (draft: PolicyDraft) =>
  invoke<string>("alter_policy_sql", { draft });

export const dropPolicySql = (schema: string, table: string, name: string) =>
  invoke<string>("drop_policy_sql", { schema, table, name });

export const rlsSql = (schema: string, table: string, enable: boolean) =>
  invoke<string>("rls_sql", { schema, table, enable });

/** Runs a statement already confirmed via one of the `*Sql` preview calls above. */
export const executeDdl = (connectionId: string, sql: string) =>
  invoke<void>("execute_ddl", { connectionId, sql });

export const listAuthUsers = (connectionId: string) =>
  invoke<AuthUser[]>("list_auth_users", { connectionId });

/** One local migration file, matched against the tracking table by version. */
export interface MigrationInfo {
  version: string;
  name: string;
  applied: boolean;
}

/**
 * `table` overrides tracking-table detection (defaults to
 * `supabase_migrations.schema_migrations` when present, else
 * `public.schema_migrations`, created on first `applyPendingMigrations`).
 */
export const migrationStatus = (connectionId: string, folder: string, table?: string) =>
  invoke<MigrationInfo[]>("migration_status", { connectionId, folder, table: table || null });

/** Runs pending `.sql` files in `folder` order; returns the versions applied. */
export const applyPendingMigrations = (connectionId: string, folder: string, table?: string) =>
  invoke<string[]>("apply_pending_migrations", { connectionId, folder, table: table || null });

/** Supabase project self-check over HTTP; needs `supabaseUrl` + service key. */
export const supabaseProjectInfo = (connectionId: string) =>
  invoke<SupabaseProjectInfo>("supabase_project_info", { connectionId });

export const supabaseListBuckets = (connectionId: string) =>
  invoke<StorageBucket[]>("supabase_list_buckets", { connectionId });

export const supabaseListUsers = (connectionId: string, page: number, perPage: number) =>
  invoke<AdminUser[]>("supabase_list_users", { connectionId, page, perPage });

/** `banDuration` is a GoTrue duration string, e.g. "24h"; pass "none" to unban. */
export const supabaseBanUser = (connectionId: string, userId: string, banDuration: string) =>
  invoke<void>("supabase_ban_user", { connectionId, userId, banDuration });

export const supabaseDeleteUser = (connectionId: string, userId: string) =>
  invoke<void>("supabase_delete_user", { connectionId, userId });

export const supabaseListEdgeFunctions = (connectionId: string) =>
  invoke<EdgeFunction[]>("supabase_list_edge_functions", { connectionId });

export interface AppSettings {
  /** "light" | "dark" | "system" — defaults to "dark" */
  theme: "light" | "dark" | "system";
  /** Default row limit for table browse/query results. */
  defaultRowLimit: number;
  /** Statement timeout in seconds. */
  defaultStatementTimeout: number;
}

export const loadSettings = () =>
  invoke<AppSettings>("load_settings");

export const saveSettings = (settings: AppSettings) =>
  invoke<void>("save_settings", { settings });

export const importRowsFromFile = (
  connectionId: string,
  schema: string,
  table: string,
  filePath: string,
) => invoke<void>("import_rows_from_file", { connectionId, schema, table, filePath });

export const importRows = (
  connectionId: string,
  schema: string,
  table: string,
  rows: Row[],
) => invoke<void>("import_rows", { connectionId, schema, table, rows });
