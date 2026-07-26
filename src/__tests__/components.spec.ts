import { describe, expect, it } from "vitest";
import { mount } from "@vue/test-utils";
import ConnectionForm from "../components/ConnectionForm.vue";
import ConnectionList from "../components/ConnectionList.vue";
import DataGrid from "../components/DataGrid.vue";
import MigrationsPanel from "../components/MigrationsPanel.vue";
import QueryEditor from "../components/QueryEditor.vue";
import SupabasePanel from "../components/SupabasePanel.vue";
import TableList from "../components/TableList.vue";
import type { ConnectionInfo, MigrationInfo, PolicyInfo, TableInfo } from "../api";

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

  it("carries an SSH tunnel config and secret when the section is filled in", async () => {
    const w = mount(ConnectionForm);
    await w.find("select").setValue("postgres");
    await w.find("input[placeholder^='Name']").setValue("prod via bastion");
    await w.find("input[type=checkbox]").setValue(true);
    await w.find("input[placeholder='SSH host (bastion)']").setValue("bastion.example.com");
    await w.find("input[placeholder='SSH user']").setValue("deploy");
    await w.find("input[placeholder^='Private key path']").setValue("~/.ssh/id_ed25519");
    await w.find("input[type=password][placeholder*='passphrase']").setValue("s3cret");
    await w.find("form").trigger("submit");
    const [info, , , sshSecret] = w.emitted("save")![0] as [
      ConnectionInfo,
      string | undefined,
      string | undefined,
      string | undefined,
    ];
    expect(info.sshTunnel).toEqual({
      host: "bastion.example.com",
      port: 22,
      user: "deploy",
      auth: { method: "key", keyPath: "~/.ssh/id_ed25519" },
    });
    expect(sshSecret).toBe("s3cret");
  });

  it("leaves sshTunnel unset when the section is left collapsed", async () => {
    const w = mount(ConnectionForm);
    await w.find("select").setValue("postgres");
    await w.find("input[placeholder^='Name']").setValue("prod");
    await w.find("form").trigger("submit");
    const [info, , , sshSecret] = w.emitted("save")![0] as [
      ConnectionInfo,
      string | undefined,
      string | undefined,
      string | undefined,
    ];
    expect(info.sshTunnel).toBeUndefined();
    expect(sshSecret).toBeUndefined();
  });
});

describe("ConnectionForm", () => {
  it("defaults the port to postgres's before an engine is picked", () => {
    const w = mount(ConnectionForm);
    expect((w.find("input[type=number]").element as HTMLInputElement).valueAsNumber).toBe(5432);
  });

  it("switches the default port when the engine changes to mysql", async () => {
    const w = mount(ConnectionForm);
    await w.find("select").setValue("mysql");
    expect((w.find("input[type=number]").element as HTMLInputElement).valueAsNumber).toBe(3306);
  });

  it("emits save with the selected engine kind", async () => {
    const w = mount(ConnectionForm);
    await w.find("select").setValue("mysql");
    await w.find("input[placeholder^='Name']").setValue("my mysql box");
    await w.find("form").trigger("submit");
    const [[info]] = w.emitted("save") as [[ConnectionInfo, string | undefined]];
    expect(info.kind).toBe("mysql");
    expect(info.port).toBe(3306);
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

  it("emits sort when a header is clicked, without needing pageable", async () => {
    const w = mount(DataGrid, { props: { rows: [{ id: 1, name: "a" }] } });
    await w.findAll("th").find((t) => t.text().startsWith("name"))!.trigger("click");
    expect(w.emitted("sort")).toEqual([["name"]]);
  });

  it("shows a sort indicator on the active sort column", () => {
    const w = mount(DataGrid, {
      props: { rows: [{ id: 1, name: "a" }], sortColumn: "name", sortDesc: true },
    });
    const header = w.findAll("th").find((t) => t.text().startsWith("name"))!;
    expect(header.text()).toContain("▼");
  });

  it("hides pager/filter controls unless pageable", () => {
    const w = mount(DataGrid, { props: { rows: [] } });
    expect(w.find(".controls").exists()).toBe(false);
  });

  it("emits page for prev/next and clamps at the edges", async () => {
    const w = mount(DataGrid, {
      props: { rows: [], pageable: true, page: 2, pageSize: 10, approxTotal: 25 },
    });
    expect(w.text()).toContain("Page 2 / 3");
    await w.findAll("button").find((b) => b.text() === "Prev")!.trigger("click");
    await w.findAll("button").find((b) => b.text() === "Next")!.trigger("click");
    expect(w.emitted("page")).toEqual([[1], [3]]);
  });

  it("emits filter with the column and value", async () => {
    const w = mount(DataGrid, {
      props: { rows: [{ id: 1, name: "a" }], pageable: true },
    });
    await w.find("input[placeholder='filter column']").setValue("name");
    await w.find("input[placeholder='filter value']").setValue("a");
    await w.findAll("button").find((b) => b.text() === "Filter")!.trigger("click");
    expect(w.emitted("filter")).toEqual([["name", "a"]]);
  });

  it("edits a cell on double-click + enter when editable with a primary key", async () => {
    const w = mount(DataGrid, {
      props: {
        rows: [{ id: 1, title: "buy milk" }],
        editable: true,
        pkColumns: ["id"],
      },
    });
    await w.findAll("td")[1].trigger("dblclick");
    const input = w.find("input.cell-input");
    expect(input.exists()).toBe(true);
    await input.setValue("buy bread");
    await input.trigger("keydown", { key: "Enter" });
    expect(w.emitted("edit")).toEqual([[0, "title", "buy bread"]]);
  });

  it("cancels an edit on escape without emitting", async () => {
    const w = mount(DataGrid, {
      props: { rows: [{ id: 1, title: "buy milk" }], editable: true, pkColumns: ["id"] },
    });
    await w.findAll("td")[1].trigger("dblclick");
    await w.find("input.cell-input").trigger("keydown", { key: "Escape" });
    expect(w.find("input.cell-input").exists()).toBe(false);
    expect(w.emitted("edit")).toBeUndefined();
  });

  it("does not enter edit mode without primary-key columns", async () => {
    const w = mount(DataGrid, {
      props: { rows: [{ id: 1, title: "buy milk" }], editable: true, pkColumns: [] },
    });
    await w.findAll("td")[1].trigger("dblclick");
    expect(w.find("input.cell-input").exists()).toBe(false);
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

  it("emits create-policy with the form fields", async () => {
    const w = mount(SupabasePanel, {
      props: { policies: [], authUsers: [], authError: "" },
    });
    await w.find("input[placeholder='table']").setValue("todos");
    await w.find("input[placeholder='policy name']").setValue("own rows");
    await w.find("select").setValue("SELECT");
    await w.find("input[placeholder^='roles']").setValue("authenticated, service_role");
    await w.find("textarea[placeholder='USING expression']").setValue("auth.uid() = user_id");
    await w.find("form").trigger("submit");
    expect(w.emitted("create-policy")).toEqual([
      [
        {
          schema: "public",
          table: "todos",
          name: "own rows",
          command: "SELECT",
          roles: ["authenticated", "service_role"],
          usingExpr: "auth.uid() = user_id",
          checkExpr: undefined,
        },
      ],
    ]);
  });

  it("does not emit create-policy without a table and name", async () => {
    const w = mount(SupabasePanel, {
      props: { policies: [], authUsers: [], authError: "" },
    });
    await w.find("form").trigger("submit");
    expect(w.emitted("create-policy")).toBeUndefined();
  });

  it("emits toggle-rls with the current schema/table", async () => {
    const w = mount(SupabasePanel, {
      props: { policies: [], authUsers: [], authError: "" },
    });
    await w.find("input[placeholder='table']").setValue("todos");
    await w.findAll("button").find((b) => b.text() === "Enable RLS")!.trigger("click");
    expect(w.emitted("toggle-rls")).toEqual([["public", "todos", true]]);
  });

  it("emits drop-policy for a policy row", async () => {
    const w = mount(SupabasePanel, {
      props: { policies, authUsers: [], authError: "" },
    });
    await w.findAll("button").find((b) => b.text() === "Drop")!.trigger("click");
    expect(w.emitted("drop-policy")).toEqual([[policies[0]]]);
  });

  it("shows the ddl error", () => {
    const w = mount(SupabasePanel, {
      props: { policies: [], authUsers: [], authError: "", ddlError: "syntax error at or near" },
    });
    expect(w.text()).toContain("syntax error at or near");
  });

  it("marks the SQL auth-users fallback as read-only when there is no project", () => {
    const w = mount(SupabasePanel, {
      props: {
        policies: [],
        authUsers: [{ id: "u1", email: "a@b.c", createdAt: "2026-01-01" }],
        authError: "",
      },
    });
    expect(w.text()).toContain("Read-only");
  });

  const projectInfo = {
    projectRef: "abcdefgh",
    url: "https://abcdefgh.supabase.co",
    title: "PostgREST API",
    description: "standard public schema",
    restVersion: "12.2.0",
  };

  it("renders admin users and filters them by email client-side", async () => {
    const w = mount(SupabasePanel, {
      props: {
        policies: [],
        authUsers: [],
        authError: "",
        projectInfo,
        adminUsers: [
          { id: "u1", email: "a@example.com", createdAt: "2026-01-01" },
          { id: "u2", email: "b@example.com", createdAt: "2026-01-02", bannedUntil: "2099-01-01" },
        ],
      },
    });
    expect(w.text()).toContain("a@example.com");
    expect(w.text()).toContain("banned until 2099-01-01");
    await w.find("input[placeholder^='filter by email']").setValue("a@");
    expect(w.text()).toContain("a@example.com");
    expect(w.text()).not.toContain("b@example.com");
  });

  it("emits load-users for prev/next", async () => {
    const w = mount(SupabasePanel, {
      props: { policies: [], authUsers: [], authError: "", projectInfo, adminPage: 2 },
    });
    await w.findAll("button").find((b) => b.text() === "Next")!.trigger("click");
    await w.findAll("button").find((b) => b.text() === "Prev")!.trigger("click");
    expect(w.emitted("load-users")).toEqual([[3], [1]]);
  });

  it("emits ban-user and delete-user for an admin user row", async () => {
    const user = { id: "u1", email: "a@example.com", createdAt: "2026-01-01" };
    const w = mount(SupabasePanel, {
      props: { policies: [], authUsers: [], authError: "", projectInfo, adminUsers: [user] },
    });
    await w.findAll("button").find((b) => b.text() === "Ban")!.trigger("click");
    await w.findAll("button").find((b) => b.text() === "Delete")!.trigger("click");
    expect(w.emitted("ban-user")).toEqual([[user]]);
    expect(w.emitted("delete-user")).toEqual([[user]]);
  });
});

describe("MigrationsPanel", () => {
  const migrations: MigrationInfo[] = [
    { version: "1", name: "create_users", applied: true },
    { version: "2", name: "add_index", applied: false },
  ];

  it("renders migrations with status", () => {
    const w = mount(MigrationsPanel, { props: { migrations } });
    expect(w.text()).toContain("create_users");
    expect(w.text()).toContain("applied");
    expect(w.text()).toContain("pending");
  });

  it("shows an empty state with no migrations loaded", () => {
    const w = mount(MigrationsPanel, { props: { migrations: [] } });
    expect(w.text()).toContain("No migrations loaded");
  });

  it("emits refresh with the folder and table fields", async () => {
    const w = mount(MigrationsPanel, { props: { migrations: [] } });
    await w.find("input[placeholder^='Migrations folder']").setValue("/tmp/migrations");
    await w.find("input[placeholder^='Tracking table']").setValue("my_migrations");
    await w.find("button").trigger("click");
    expect(w.emitted("refresh")).toEqual([["/tmp/migrations", "my_migrations"]]);
  });

  it("disables run when there is nothing pending, enables it otherwise", async () => {
    const allApplied = migrations.map((m) => ({ ...m, applied: true }));
    const w = mount(MigrationsPanel, { props: { migrations: allApplied } });
    await w.find("input[placeholder^='Migrations folder']").setValue("/tmp/migrations");
    const runButton = w.findAll("button").find((b) => b.text().startsWith("Run pending"))!;
    expect(runButton.attributes("disabled")).toBeDefined();

    const w2 = mount(MigrationsPanel, { props: { migrations } });
    await w2.find("input[placeholder^='Migrations folder']").setValue("/tmp/migrations");
    const runButton2 = w2.findAll("button").find((b) => b.text().startsWith("Run pending"))!;
    expect(runButton2.attributes("disabled")).toBeUndefined();
    await runButton2.trigger("click");
    expect(w2.emitted("run")).toEqual([["/tmp/migrations", ""]]);
  });
});
