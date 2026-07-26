import { beforeEach, describe, expect, it, vi } from "vitest";
import { flushPromises, mount } from "@vue/test-utils";

vi.mock("../api", () => ({
  listConnections: vi.fn(),
  listTables: vi.fn(),
  tableRows: vi.fn(),
  runQuery: vi.fn(),
  listPolicies: vi.fn(),
  listAuthUsers: vi.fn(),
  saveConnection: vi.fn(),
  deleteConnection: vi.fn(),
  testConnection: vi.fn(),
}));

import * as api from "../api";
import App from "../App.vue";

describe("App", () => {
  beforeEach(() => {
    vi.mocked(api.listConnections).mockResolvedValue([
      {
        id: "c1",
        name: "local.db",
        kind: "sqlite",
        host: "",
        port: 0,
        user: "",
        database: "/tmp/local.db",
      },
    ]);
    vi.mocked(api.listTables).mockResolvedValue([
      { schema: "main", name: "todos", kind: "BASE TABLE" },
    ]);
  });

  it("keeps the tables list when policies/auth users error, as a SQLite connection always does", async () => {
    vi.mocked(api.listPolicies).mockRejectedValue(
      new Error("row-level security policies are a Postgres/Supabase feature"),
    );
    vi.mocked(api.listAuthUsers).mockRejectedValue(
      new Error("Supabase auth users are a Postgres/Supabase feature"),
    );

    const w = mount(App);
    await flushPromises();

    await w.find(".connection-list button.name").trigger("click");
    await flushPromises();

    expect(w.text()).toContain("todos");
    expect(w.find(".error").exists()).toBe(false);
  });
});
