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
  supabaseProjectInfo: vi.fn(),
  supabaseListBuckets: vi.fn(),
}));

import * as api from "../api";
import App from "../App.vue";

describe("App", () => {
  beforeEach(() => {
    vi.clearAllMocks();
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

  it("skips the Supabase project calls for a connection without a project URL", async () => {
    vi.mocked(api.listPolicies).mockResolvedValue([]);
    vi.mocked(api.listAuthUsers).mockResolvedValue([]);

    const w = mount(App);
    await flushPromises();
    await w.find(".connection-list button.name").trigger("click");
    await flushPromises();

    expect(api.supabaseProjectInfo).not.toHaveBeenCalled();
    expect(api.supabaseListBuckets).not.toHaveBeenCalled();
  });

  it("loads project info and buckets for a Supabase connection", async () => {
    vi.mocked(api.listConnections).mockResolvedValue([
      {
        id: "sb",
        name: "supabase prod",
        kind: "postgres",
        host: "db.abcdefgh.supabase.co",
        port: 5432,
        user: "postgres",
        database: "postgres",
        supabaseUrl: "https://abcdefgh.supabase.co",
      },
    ]);
    vi.mocked(api.listPolicies).mockResolvedValue([]);
    vi.mocked(api.listAuthUsers).mockResolvedValue([]);
    vi.mocked(api.supabaseProjectInfo).mockResolvedValue({
      projectRef: "abcdefgh",
      url: "https://abcdefgh.supabase.co",
      title: "PostgREST API",
      description: "standard public schema",
      restVersion: "12.2.0",
    });
    vi.mocked(api.supabaseListBuckets).mockResolvedValue([
      { id: "avatars", name: "avatars", public: true, createdAt: "2026-01-02", updatedAt: "2026-01-02" },
    ]);

    const w = mount(App);
    await flushPromises();
    await w.find(".connection-list button.name").trigger("click");
    await flushPromises();

    expect(api.supabaseProjectInfo).toHaveBeenCalledWith("sb");
    await w.findAll(".toolbar button")[2].trigger("click");
    expect(w.text()).toContain("abcdefgh");
    expect(w.text()).toContain("avatars");
  });

  it("shows the project error and no buckets when the API call fails", async () => {
    vi.mocked(api.listConnections).mockResolvedValue([
      {
        id: "sb",
        name: "supabase prod",
        kind: "postgres",
        host: "db.abcdefgh.supabase.co",
        port: 5432,
        user: "postgres",
        database: "postgres",
        supabaseUrl: "https://abcdefgh.supabase.co",
      },
    ]);
    vi.mocked(api.listPolicies).mockResolvedValue([]);
    vi.mocked(api.listAuthUsers).mockResolvedValue([]);
    vi.mocked(api.supabaseProjectInfo).mockRejectedValue(
      new Error("Supabase /rest/v1/ returned 401 Unauthorized"),
    );

    const w = mount(App);
    await flushPromises();
    await w.find(".connection-list button.name").trigger("click");
    await flushPromises();

    // A failed self-check must not go on to ask for buckets with a bad key.
    expect(api.supabaseListBuckets).not.toHaveBeenCalled();
    await w.findAll(".toolbar button")[2].trigger("click");
    expect(w.text()).toContain("401 Unauthorized");
  });
});
