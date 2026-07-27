import { beforeEach, describe, expect, it, vi } from "vitest";

const invoke = vi.fn();
vi.mock("@tauri-apps/api/core", () => ({
  invoke: (...args: unknown[]) => invoke(...args),
}));

import {
  listTables,
  runQuery,
  saveConnection,
  supabaseListBuckets,
  supabaseProjectInfo,
  tableRows,
} from "../api";

describe("api wrappers", () => {
  beforeEach(() => invoke.mockReset());

  it("save_connection passes info and null password by default", async () => {
    const info = {
      id: "1",
      name: "x",
      host: "localhost",
      port: 5432,
      user: "postgres",
      database: "postgres",
    };
    await saveConnection(info);
    expect(invoke).toHaveBeenCalledWith("save_connection", {
      info,
      password: null,
      serviceKey: null,
    });
  });

  it("save_connection passes the Supabase service key when given", async () => {
    const info = {
      id: "1",
      name: "supabase prod",
      host: "db.abcdefgh.supabase.co",
      port: 5432,
      user: "postgres",
      database: "postgres",
      supabaseUrl: "https://abcdefgh.supabase.co",
    };
    await saveConnection(info, "dbpass", "sk-service");
    expect(invoke).toHaveBeenCalledWith("save_connection", {
      info,
      password: "dbpass",
      serviceKey: "sk-service",
    });
  });

  it("supabase_project_info and supabase_list_buckets pass the connection id", async () => {
    invoke.mockResolvedValue([]);
    await supabaseProjectInfo("c1");
    expect(invoke).toHaveBeenCalledWith("supabase_project_info", {
      connectionId: "c1",
    });
    await supabaseListBuckets("c1");
    expect(invoke).toHaveBeenCalledWith("supabase_list_buckets", {
      connectionId: "c1",
    });
  });

  it("list_tables passes the connection id", async () => {
    invoke.mockResolvedValue([]);
    await listTables("c1");
    expect(invoke).toHaveBeenCalledWith("list_tables", { connectionId: "c1" });
  });

  it("table_rows passes schema, table and the paging/sort/filter query", async () => {
    invoke.mockResolvedValue({ rows: [], approxTotal: 0 });
    const query = {
      page: 1,
      pageSize: 50,
      orderBy: "id",
      orderDesc: true,
      filterColumn: "name",
      filterValue: "a",
    };
    await tableRows("c1", "public", "todos", query);
    expect(invoke).toHaveBeenCalledWith("table_rows", {
      connectionId: "c1",
      schema: "public",
      table: "todos",
      query,
    });
  });

  it("run_query passes the connection id and sql text", async () => {
    invoke.mockResolvedValue([]);
    await runQuery("c1", "select * from todos");
    expect(invoke).toHaveBeenCalledWith("run_query", {
      connectionId: "c1",
      sql: "select * from todos",
    });
  });
});
