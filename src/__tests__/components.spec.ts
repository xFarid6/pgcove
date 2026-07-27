import { describe, expect, it } from "vitest";
import { mount } from "@vue/test-utils";
import ConnectionForm from "../components/ConnectionForm.vue";
import ConnectionList from "../components/ConnectionList.vue";
import DataGrid from "../components/DataGrid.vue";
import QueryEditor from "../components/QueryEditor.vue";
import SupabasePanel from "../components/SupabasePanel.vue";
import TableList from "../components/TableList.vue";
import type { ConnectionInfo, PolicyInfo, TableInfo } from "../api";

const conns: ConnectionInfo[] = [
  {
    id: "1",
    name: "supabase prod",
    host: "aws-0-eu-central-1.pooler.supabase.com",
    port: 6543,
    user: "postgres.abc",
    database: "postgres",
  },
];

describe("ConnectionList", () => {
  it("renders connections with detail line", () => {
    const w = mount(ConnectionList, {
      props: { connections: conns, activeId: null },
    });
    expect(w.text()).toContain("supabase prod");
    expect(w.text()).toContain("postgres.abc@aws-0-eu-central-1.pooler.supabase.com:6543/postgres");
  });

  it("emits select", async () => {
    const w = mount(ConnectionList, {
      props: { connections: conns, activeId: null },
    });
    await w.find("button.name").trigger("click");
    expect(w.emitted("select")).toEqual([["1"]]);
  });

  it("shows just the file path for a SQLite connection", () => {
    const sqliteConn: ConnectionInfo = {
      id: "2",
      name: "local.db",
      kind: "sqlite",
      host: "",
      port: 0,
      user: "",
      database: "/home/me/local.db",
    };
    const w = mount(ConnectionList, {
      props: { connections: [sqliteConn], activeId: null },
    });
    expect(w.text()).toContain("/home/me/local.db");
    expect(w.text()).not.toContain("@:0");
  });
});

describe("ConnectionForm", () => {
  it("submits a plain Postgres connection", async () => {
    const w = mount(ConnectionForm);
    await w.find("select").setValue("postgres");
    await w.find("input[placeholder^='Name']").setValue("prod");
    await w.find("form").trigger("submit");
    const [info, password] = w.emitted("save")![0] as [ConnectionInfo, string | undefined];
    expect(info.kind).toBe("postgres");
    expect(info.host).toBe("localhost");
    expect(info.supabaseUrl).toBeUndefined();
    expect(password).toBeUndefined();
  });

  it("defaults to the Supabase project variant and carries url + service key", async () => {
    const w = mount(ConnectionForm);
    expect((w.find("select").element as HTMLSelectElement).value).toBe("supabase");
    await w.find("input[placeholder^='Name']").setValue("supabase prod");
    await w.find("input[placeholder^='Project URL']").setValue("https://abcdefgh.supabase.co");
    await w.find("input[placeholder^='Service-role key']").setValue("sk-service");
    await w.find("form").trigger("submit");
    const [info, password, serviceKey] = w.emitted("save")![0] as [
      ConnectionInfo,
      string | undefined,
      string | undefined,
    ];
    expect(info.kind).toBe("postgres");
    expect(info.supabaseUrl).toBe("https://abcdefgh.supabase.co");
    // Project URL fills in the direct database host Supabase gives every project.
    expect(info.host).toBe("db.abcdefgh.supabase.co");
    expect(info.user).toBe("postgres");
    expect(password).toBeUndefined();
    expect(serviceKey).toBe("sk-service");
  });

  it("does not submit a Supabase connection without a project URL", async () => {
    const w = mount(ConnectionForm);
    await w.find("input[placeholder^='Name']").setValue("supabase prod");
    await w.find("form").trigger("submit");
    expect(w.emitted("save")).toBeUndefined();
  });

  it("never sends a service key on a non-Supabase connection", async () => {
    const w = mount(ConnectionForm);
    await w.find("input[placeholder^='Project URL']").setValue("https://abcdefgh.supabase.co");
    await w.find("input[placeholder^='Service-role key']").setValue("sk-service");
    await w.find("select").setValue("postgres");
    await w.find("input[placeholder^='Name']").setValue("prod");
    await w.find("form").trigger("submit");
    const [info, , serviceKey] = w.emitted("save")![0] as [
      ConnectionInfo,
      string | undefined,
      string | undefined,
    ];
    expect(info.supabaseUrl).toBeUndefined();
    expect(serviceKey).toBeUndefined();
  });

  it("switches to a file-path field and submits a SQLite connection", async () => {
    const w = mount(ConnectionForm);
    await w.find("select").setValue("sqlite");
    await w.find("input[placeholder='Name (e.g. supabase prod)']").setValue("local.db");
    await w.find("input[placeholder^='File path']").setValue("/tmp/local.db");
    await w.find("form").trigger("submit");
    const [info] = w.emitted("save")![0] as [ConnectionInfo, string | undefined];
    expect(info.kind).toBe("sqlite");
    expect(info.database).toBe("/tmp/local.db");
  });

  it("does not submit a SQLite connection without a file path", async () => {
    const w = mount(ConnectionForm);
    await w.find("select").setValue("sqlite");
    await w.find("input[placeholder='Name (e.g. supabase prod)']").setValue("local.db");
    await w.find("form").trigger("submit");
    expect(w.emitted("save")).toBeUndefined();
  });
});

const tables: TableInfo[] = [
  { schema: "public", name: "todos", kind: "BASE TABLE" },
  { schema: "public", name: "todo_view", kind: "VIEW" },
];

describe("TableList", () => {
  it("renders tables, marks views, emits select", async () => {
    const w = mount(TableList, { props: { tables, active: null } });
    expect(w.text()).toContain("todos");
    expect(w.text()).toContain("view");
    await w.find("button").trigger("click");
    expect(w.emitted("select")).toEqual([[tables[0]]]);
  });
});

describe("DataGrid", () => {
  it("derives columns from row keys and renders values", () => {
    const w = mount(DataGrid, {
      props: {
        rows: [
          { id: 1, title: "buy milk", done: false, meta: { a: 1 }, gone: null },
        ],
      },
    });
    expect(w.findAll("th").map((t) => t.text())).toEqual([
      "id",
      "title",
      "done",
      "meta",
      "gone",
    ]);
    expect(w.text()).toContain("buy milk");
    expect(w.text()).toContain('{"a":1}'); // objects JSON-stringified
    expect(w.text()).toContain("∅"); // null marker
  });

  it("shows an empty state", () => {
    const w = mount(DataGrid, { props: { rows: [] } });
    expect(w.text()).toContain("No rows");
  });
});

describe("QueryEditor", () => {
  it("emits run with the textarea contents", async () => {
    const w = mount(QueryEditor, { props: { running: false } });
    await w.find("textarea").setValue("select 1");
    await w.find("button").trigger("click");
    expect(w.emitted("run")).toEqual([["select 1"]]);
  });

  it("does not emit run for blank input", async () => {
    const w = mount(QueryEditor, { props: { running: false } });
    await w.find("textarea").setValue("   ");
    await w.find("button").trigger("click");
    expect(w.emitted("run")).toBeUndefined();
  });

  it("disables the run button while running", () => {
    const w = mount(QueryEditor, { props: { running: true } });
    expect(w.find("button").attributes("disabled")).toBeDefined();
    expect(w.find("button").text()).toBe("Running…");
  });

  it("runs on ctrl/cmd+enter", async () => {
    const w = mount(QueryEditor, { props: { running: false } });
    await w.find("textarea").setValue("select 2");
    await w.find("textarea").trigger("keydown", { key: "Enter", ctrlKey: true });
    expect(w.emitted("run")).toEqual([["select 2"]]);
  });
});

const policies: PolicyInfo[] = [
  {
    schema: "public",
    table: "todos",
    name: "own rows",
    command: "SELECT",
    roles: "{authenticated}",
    expression: "(auth.uid() = user_id)",
  },
];

describe("SupabasePanel", () => {
  it("renders policies and auth users", () => {
    const w = mount(SupabasePanel, {
      props: {
        policies,
        authUsers: [{ id: "u1", email: "a@b.c", createdAt: "2026-01-01" }],
        authError: "",
      },
    });
    expect(w.text()).toContain("own rows");
    expect(w.text()).toContain("auth.uid() = user_id");
    expect(w.text()).toContain("a@b.c");
  });

  it("shows the auth error instead of the users table", () => {
    const w = mount(SupabasePanel, {
      props: {
        policies: [],
        authUsers: [],
        authError: "auth.users not readable — not a Supabase database?",
      },
    });
    expect(w.text()).toContain("not a Supabase database");
  });

  it("renders project info and storage buckets", () => {
    const w = mount(SupabasePanel, {
      props: {
        policies: [],
        authUsers: [],
        authError: "",
        projectInfo: {
          projectRef: "abcdefgh",
          url: "https://abcdefgh.supabase.co",
          title: "PostgREST API",
          description: "standard public schema",
          restVersion: "12.2.0",
        },
        buckets: [
          { id: "avatars", name: "avatars", public: true, createdAt: "2026-01-02", updatedAt: "2026-01-02" },
          { id: "invoices", name: "invoices", public: false, createdAt: "2026-01-03", updatedAt: "2026-01-03" },
        ],
      },
    });
    expect(w.text()).toContain("abcdefgh");
    expect(w.text()).toContain("PostgREST 12.2.0");
    expect(w.text()).toContain("avatars");
    expect(w.text()).toContain("public");
    expect(w.text()).toContain("private");
  });

  it("falls back to a hint when the connection is not a Supabase project", () => {
    const w = mount(SupabasePanel, {
      props: { policies: [], authUsers: [], authError: "" },
    });
    expect(w.text()).toContain("Not a Supabase project connection");
    expect(w.text()).toContain("No storage buckets");
  });

  it("shows the project error when the HTTP API call failed", () => {
    const w = mount(SupabasePanel, {
      props: {
        policies: [],
        authUsers: [],
        authError: "",
        projectError: "Supabase /rest/v1/ returned 401 Unauthorized",
      },
    });
    expect(w.text()).toContain("401 Unauthorized");
  });

  it("says edge functions need a management token instead of faking rows", () => {
    const w = mount(SupabasePanel, {
      props: { policies: [], authUsers: [], authError: "" },
    });
    expect(w.text()).toContain("Edge functions");
    expect(w.text()).toContain("management access token");
  });
});
