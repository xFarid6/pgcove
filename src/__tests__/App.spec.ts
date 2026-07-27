import { beforeEach, describe, expect, it, vi } from "vitest";
import { flushPromises, mount } from "@vue/test-utils";

vi.mock("../api", () => ({
  listConnections: vi.fn(),
  listTables: vi.fn(),
  tableRows: vi.fn(),
  tableStructure: vi.fn(),
  runQuery: vi.fn(),
  listPolicies: vi.fn(),
  listAuthUsers: vi.fn(),
  saveConnection: vi.fn(),
  deleteConnection: vi.fn(),
  testConnection: vi.fn(),
  supabaseProjectInfo: vi.fn(),
  supabaseListBuckets: vi.fn(),
  supabaseListUsers: vi.fn(),
  supabaseBanUser: vi.fn(),
  supabaseDeleteUser: vi.fn(),
  createPolicySql: vi.fn(),
  alterPolicySql: vi.fn(),
  dropPolicySql: vi.fn(),
  rlsSql: vi.fn(),
  executeDdl: vi.fn(),
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
    vi.mocked(api.supabaseListUsers).mockResolvedValue([
      { id: "u1", email: "a@example.com", createdAt: "2026-01-01" },
    ]);

    const w = mount(App);
    await flushPromises();
    await w.find(".connection-list button.name").trigger("click");
    await flushPromises();

    expect(api.supabaseProjectInfo).toHaveBeenCalledWith("sb");
    expect(api.supabaseListUsers).toHaveBeenCalledWith("sb", 1, 50);
    await w.findAll(".toolbar button")[3].trigger("click");
    expect(w.text()).toContain("abcdefgh");
    expect(w.text()).toContain("avatars");
    expect(w.text()).toContain("a@example.com");
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
    await w.findAll(".toolbar button")[3].trigger("click");
    expect(w.text()).toContain("401 Unauthorized");
  });

  describe("RLS policy DDL", () => {
    beforeEach(() => {
      vi.mocked(api.listPolicies).mockResolvedValue([]);
      vi.mocked(api.listAuthUsers).mockResolvedValue([]);
    });

    async function openPolicyForm() {
      const w = mount(App);
      await flushPromises();
      await w.find(".connection-list button.name").trigger("click");
      await flushPromises();
      await w.findAll(".toolbar button")[3].trigger("click");
      await w.find("input[placeholder='table']").setValue("todos");
      await w.find("input[placeholder='policy name']").setValue("own rows");
      return w;
    }

    it("executes a confirmed create-policy statement and refreshes policies", async () => {
      window.confirm = vi.fn().mockReturnValue(true);
      vi.mocked(api.createPolicySql).mockResolvedValue("CREATE POLICY ...;");

      const w = await openPolicyForm();
      await w.find(".supabase-panel form").trigger("submit");
      await flushPromises();

      expect(api.createPolicySql).toHaveBeenCalledWith(
        expect.objectContaining({ table: "todos", name: "own rows" }),
      );
      expect(api.executeDdl).toHaveBeenCalledWith("c1", "CREATE POLICY ...;");
      expect(api.listPolicies).toHaveBeenCalledTimes(2); // initial select + post-DDL refresh
    });

    it("does not execute when the user cancels the confirm dialog", async () => {
      window.confirm = vi.fn().mockReturnValue(false);
      vi.mocked(api.createPolicySql).mockResolvedValue("CREATE POLICY ...;");

      const w = await openPolicyForm();
      await w.find(".supabase-panel form").trigger("submit");
      await flushPromises();

      expect(api.executeDdl).not.toHaveBeenCalled();
    });

    it("shows the ddl error when execution fails", async () => {
      window.confirm = vi.fn().mockReturnValue(true);
      vi.mocked(api.createPolicySql).mockResolvedValue("CREATE POLICY ...;");
      vi.mocked(api.executeDdl).mockRejectedValue(new Error("permission denied"));

      const w = await openPolicyForm();
      await w.find(".supabase-panel form").trigger("submit");
      await flushPromises();

      expect(w.text()).toContain("permission denied");
    });
  });

  describe("table structure tab", () => {
    beforeEach(() => {
      vi.mocked(api.listPolicies).mockResolvedValue([]);
      vi.mocked(api.listAuthUsers).mockResolvedValue([]);
      vi.mocked(api.tableRows).mockResolvedValue([]);
    });

    it("loads structure on table select and shows it under the Structure tab", async () => {
      vi.mocked(api.tableStructure).mockResolvedValue({
        columns: [{ name: "id", dataType: "integer", nullable: false }],
        indexes: [{ name: "todos_pkey", definition: "CREATE UNIQUE INDEX...", isUnique: true, isPrimary: true }],
        constraints: [{ name: "todos_pkey", kind: "PRIMARY KEY", columns: "id" }],
      });

      const w = mount(App);
      await flushPromises();
      await w.find(".connection-list button.name").trigger("click");
      await flushPromises();
      await w.find(".table-list button").trigger("click");
      await flushPromises();

      expect(api.tableStructure).toHaveBeenCalledWith("c1", "main", "todos");
      await w.findAll(".toolbar button")[1].trigger("click");
      expect(w.text()).toContain("todos_pkey");
      expect(w.text()).toContain("PRIMARY KEY");
    });

    it("shows the structure error when the read fails", async () => {
      vi.mocked(api.tableStructure).mockRejectedValue(
        new Error("table structure is a Postgres/Supabase feature"),
      );

      const w = mount(App);
      await flushPromises();
      await w.find(".connection-list button.name").trigger("click");
      await flushPromises();
      await w.find(".table-list button").trigger("click");
      await flushPromises();
      await w.findAll(".toolbar button")[1].trigger("click");

      expect(w.text()).toContain("Postgres/Supabase feature");
    });
  });

  describe("admin user actions", () => {
    const sbConnection = {
      id: "sb",
      name: "supabase prod",
      kind: "postgres" as const,
      host: "db.abcdefgh.supabase.co",
      port: 5432,
      user: "postgres",
      database: "postgres",
      supabaseUrl: "https://abcdefgh.supabase.co",
    };
    const user = { id: "u1", email: "a@example.com", createdAt: "2026-01-01" };

    beforeEach(() => {
      vi.mocked(api.listConnections).mockResolvedValue([sbConnection]);
      vi.mocked(api.listPolicies).mockResolvedValue([]);
      vi.mocked(api.listAuthUsers).mockResolvedValue([]);
      vi.mocked(api.supabaseProjectInfo).mockResolvedValue({
        projectRef: "abcdefgh",
        url: "https://abcdefgh.supabase.co",
        title: "PostgREST API",
        description: "standard public schema",
        restVersion: "12.2.0",
      });
      vi.mocked(api.supabaseListBuckets).mockResolvedValue([]);
      vi.mocked(api.supabaseListUsers).mockResolvedValue([user]);
    });

    async function openAdminUsers() {
      const w = mount(App);
      await flushPromises();
      await w.find(".connection-list button.name").trigger("click");
      await flushPromises();
      await w.findAll(".toolbar button")[3].trigger("click");
      return w;
    }

    it("loads the next page on load-users", async () => {
      const w = await openAdminUsers();
      await w.findAll("button").find((b) => b.text() === "Next")!.trigger("click");
      expect(api.supabaseListUsers).toHaveBeenCalledWith("sb", 2, 50);
    });

    it("bans an active user after a duration prompt and refreshes the list", async () => {
      window.prompt = vi.fn().mockReturnValue("24h");
      const w = await openAdminUsers();
      await w.findAll("button").find((b) => b.text() === "Ban")!.trigger("click");
      await flushPromises();
      expect(api.supabaseBanUser).toHaveBeenCalledWith("sb", "u1", "24h");
      expect(api.supabaseListUsers).toHaveBeenCalledTimes(2);
    });

    it("does not ban when the duration prompt is cancelled", async () => {
      window.prompt = vi.fn().mockReturnValue(null);
      const w = await openAdminUsers();
      await w.findAll("button").find((b) => b.text() === "Ban")!.trigger("click");
      await flushPromises();
      expect(api.supabaseBanUser).not.toHaveBeenCalled();
    });

    it("deletes a user after confirmation and refreshes the list", async () => {
      window.confirm = vi.fn().mockReturnValue(true);
      const w = await openAdminUsers();
      await w.findAll("button").find((b) => b.text() === "Delete")!.trigger("click");
      await flushPromises();
      expect(api.supabaseDeleteUser).toHaveBeenCalledWith("sb", "u1");
      expect(api.supabaseListUsers).toHaveBeenCalledTimes(2);
    });

    it("shows the admin users error when a ban call fails", async () => {
      window.prompt = vi.fn().mockReturnValue("24h");
      vi.mocked(api.supabaseBanUser).mockRejectedValue(new Error("insufficient permissions"));
      const w = await openAdminUsers();
      await w.findAll("button").find((b) => b.text() === "Ban")!.trigger("click");
      await flushPromises();
      expect(w.text()).toContain("insufficient permissions");
    });
  });
});
