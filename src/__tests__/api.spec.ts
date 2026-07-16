import { beforeEach, describe, expect, it, vi } from "vitest";

const invoke = vi.fn();
vi.mock("@tauri-apps/api/core", () => ({
  invoke: (...args: unknown[]) => invoke(...args),
}));

import { listTables, saveConnection, tableRows } from "../api";

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
    });
  });

  it("list_tables passes the connection id", async () => {
    invoke.mockResolvedValue([]);
    await listTables("c1");
    expect(invoke).toHaveBeenCalledWith("list_tables", { connectionId: "c1" });
  });

  it("table_rows passes schema and table", async () => {
    invoke.mockResolvedValue([]);
    await tableRows("c1", "public", "todos");
    expect(invoke).toHaveBeenCalledWith("table_rows", {
      connectionId: "c1",
      schema: "public",
      table: "todos",
    });
  });
});
