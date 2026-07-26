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

/** A table row as serialized by Postgres row_to_json. */
export type Row = Record<string, unknown>;

export const listConnections = () =>
  invoke<ConnectionInfo[]>("list_connections");

export const saveConnection = (info: ConnectionInfo, password?: string) =>
  invoke<void>("save_connection", { info, password: password ?? null });

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
