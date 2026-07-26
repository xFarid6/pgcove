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
  it("submits a Postgres connection by default", async () => {
    const w = mount(ConnectionForm);
    await w.find("input[placeholder^='Name']").setValue("prod");
    await w.find("form").trigger("submit");
    const [info, password] = w.emitted("save")![0] as [ConnectionInfo, string | undefined];
    expect(info.kind).toBe("postgres");
    expect(info.host).toBe("localhost");
    expect(password).toBeUndefined();
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
});
