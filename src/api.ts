// Typed wrappers over the Tauri IPC surface (src-tauri/src/commands.rs).

import { invoke } from "@tauri-apps/api/core";

export type DbKind = "postgres" | "sqlite";

export interface ConnectionInfo {
  id: string;
  name: string;
  /** Defaults to "postgres" server-side if omitted. */
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

export interface AuthUser {
  id: string;
  email: string;
  createdAt: string;
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

/** A table row as serialized by Postgres row_to_json. */
export type Row = Record<string, unknown>;

export const listConnections = () =>
  invoke<ConnectionInfo[]>("list_connections");

export const saveConnection = (
  info: ConnectionInfo,
  password?: string,
  serviceKey?: string,
) =>
  invoke<void>("save_connection", {
    info,
    password: password ?? null,
    serviceKey: serviceKey ?? null,
  });

export const deleteConnection = (id: string) =>
  invoke<void>("delete_connection", { id });

export const testConnection = (id: string) =>
  invoke<string>("test_connection", { id });

export const listTables = (connectionId: string) =>
  invoke<TableInfo[]>("list_tables", { connectionId });

export const tableRows = (connectionId: string, schema: string, table: string) =>
  invoke<Row[]>("table_rows", { connectionId, schema, table });

/** Run a single SELECT-shaped statement; other statement kinds are a follow-up. */
export const runQuery = (connectionId: string, sql: string) =>
  invoke<Row[]>("run_query", { connectionId, sql });

export const listPolicies = (connectionId: string) =>
  invoke<PolicyInfo[]>("list_policies", { connectionId });

export const listAuthUsers = (connectionId: string) =>
  invoke<AuthUser[]>("list_auth_users", { connectionId });

/** Supabase project self-check over HTTP; needs `supabaseUrl` + service key. */
export const supabaseProjectInfo = (connectionId: string) =>
  invoke<SupabaseProjectInfo>("supabase_project_info", { connectionId });

export const supabaseListBuckets = (connectionId: string) =>
  invoke<StorageBucket[]>("supabase_list_buckets", { connectionId });
