# pgcove

A native database client for Postgres with **first-class Supabase awareness**
— RLS policies and auth users surfaced natively, not "it's just Postgres".
For backend/full-stack developers, especially Supabase users who find the web
dashboard slow for routine table work. SQLite is also supported (issue #4);
MySQL is planned (issue #3).

Built with **Tauri v2 + Vue 3 + TypeScript + Rust**
([sqlx](https://crates.io/crates/sqlx) with rustls). Same architecture as
[proxmox-desktop](https://github.com/xFarid6/proxmox-desktop) and
[dockshell](https://github.com/xFarid6/dockshell) — see
[ARCHITECTURE.md](ARCHITECTURE.md).

**Status: scaffold.** Working today: connection manager (profiles on disk,
password/service key in the OS keyring), schema browser (tables/views), table
data view (`SELECT * … LIMIT 100` rendered in a grid), a query editor (below),
and a Supabase panel reading real data from `pg_policies`, `auth.users` and
the project's HTTP API (below). Everything else is a filed issue — see the
repo issues for the v1 plan.

## Supabase project connections

Pick **"Supabase project"** in the connection form (the default) and paste the
project URL and service-role key alongside the database credentials. The
project URL fills in the direct database host (`db.<ref>.supabase.co`) —
overwrite it with the session pooler host if that's what your network needs.
The service-role key goes to the OS keyring on its own entry, next to the
database password, never to `connections.json`.

With that in place the Supabase panel adds:

- **Project** — a live self-check against the project's PostgREST root
  (`/rest/v1/`): project ref, URL and PostgREST version. Reachability plus a
  version string is the most a service-role key can actually tell you about
  the project itself.
- **Storage buckets** — `GET /storage/v1/bucket`, name/visibility/created.
- **Edge functions** — *not* listed yet, and the panel says so rather than
  showing anything invented. That endpoint is on the Management API proper
  (`api.supabase.com/v1/projects/<ref>/functions`) and authenticates with a
  personal/management access token, which is a different credential from the
  service-role key; adding that second token is the fast-follow.

Plain Postgres and SQLite connections are unchanged — a Supabase project is
still a Postgres connection, it just also carries a project URL.

## Query editor

A plain textarea in the "Query" tab runs a single SELECT-shaped statement and
renders the result in the same grid the table view uses. **⌘/Ctrl + Enter**
runs the query; there's also a Run button.

- **What works**: any single `SELECT` (including CTEs/`WITH`) — the backend
  wraps it as `SELECT row_to_json(t) FROM (<query>) t` server-side, so
  arbitrary column types decode for free with no client-side type mapping.
  Zero-row results come back as an empty array, not `null`. A query whose
  output has duplicate column names (e.g. an unaliased join on two tables
  that both have an `id` column) is rejected up front with a clear error
  instead of silently dropping one of the duplicate columns' data — the
  `row_to_json`/JSON-decode path can't tell two same-named columns apart
  otherwise.
- **Known limitation**: `INSERT`/`UPDATE`/`DELETE`/DDL aren't runnable from
  here yet — the wrapping subquery only accepts SELECT-shaped statements, so
  those fail with a (readable) Postgres syntax error rather than executing.
  A separate execute path returning rows-affected is a follow-up.
- **Not yet**: SQL syntax highlighting (CodeMirror) — deferred to keep the
  first increment small; the editor is a plain textarea for now.

## Quickstart (dev)

Prereqs: Node ≥ 22, pnpm ≥ 11, Rust stable (on this dev machine cargo is not
on PATH in a fresh shell — run
`$env:Path = "$env:USERPROFILE\.cargo\bin;$env:Path"` first in PowerShell).

```sh
pnpm install
pnpm tauri dev
```

Add a connection: any reachable Postgres works — pick "Postgres" in the form.
For Supabase, prefer the "Supabase project" variant above; if you only have
database credentials, the **session pooler** ones work too (host
`aws-0-<region>.pooler.supabase.com`, port 5432/6543, user
`postgres.<project-ref>`); TLS is negotiated automatically
(`sslmode=prefer`). The password goes to the OS keyring, never to disk.

For SQLite, pick "SQLite" in the connection form and give it a file path
(created on first connect if missing) or `:memory:` for a scratch database.
No password, no keyring entry. The query editor accepts full SQL there —
`CREATE`/`INSERT`/`UPDATE`/`DELETE` all run, not just `SELECT` (SQLite rows
are converted to JSON in Rust instead of the Postgres `row_to_json` subquery
trick, so it isn't limited to SELECT-shaped statements). RLS policies and
`auth.users` are Postgres/Supabase-only and error clearly on a SQLite
connection — the Supabase panel just shows it as empty.

### Tests

```sh
pnpm test                       # frontend (Vitest)
cd src-tauri; cargo test        # backend unit tests (no DB needed)
PGCOVE_TEST_URL=postgres://u:p@host:5432/db cargo test -- --ignored  # live-DB + keyring tests
# live Supabase project APIs (optional, also --ignored):
PGCOVE_TEST_SUPABASE_URL=https://<ref>.supabase.co PGCOVE_TEST_SUPABASE_KEY=<service-role key> cargo test -- --ignored
```

The Supabase HTTP client's URL/header construction and response parsing are
covered by ordinary `cargo test` unit tests — no network needed.

## Open questions for a human

- **sqlx over tokio-postgres**: chosen for built-in rustls TLS (Supabase
  requires TLS) and a future path to MySQL/SQLite with the same crate.
  Heavier compile, less code.
- **All reads are cast to text/JSON server-side** (`row_to_json`,
  `::text`) so the scaffold needs zero client-side type mapping. The query
  editor (issue #1) and in-grid editing (issue #2) will need real type
  handling — this was a deliberate scaffold shortcut, not the final design.
- **Supabase = Postgres + a project URL, not its own driver.** Issue #5 added
  the project URL + service-role key style as an optional `supabaseUrl` on a
  normal Postgres connection rather than a third `DbKind`: the wire protocol
  really is Postgres, and the HTTP APIs can't replace it (there is no SQL
  endpoint reachable with a service-role key). Revisit if a future Supabase
  API makes a URL-only connection genuinely useful.
- **No live-DB test in CI**: Postgres wire protocol isn't mockable with
  wiremock; live tests are `#[ignore]`d behind `PGCOVE_TEST_URL`. Consider a
  Postgres service container in CI later.
- **Branch protection**: not enforceable on a free-plan private repo (GitHub
  Pro feature). Treat "CI green before merge" as policy; enable real
  protection if the repo goes public or the account upgrades.
