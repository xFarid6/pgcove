# Architecture

Same shape as [proxmox-desktop](https://github.com/xFarid6/proxmox-desktop)
and [dockshell](https://github.com/xFarid6/dockshell), deliberately — the
connection-manager/keyring pattern is copy-adapted so it can be extracted
into a shared crate later (issue #14).

```
src/                      Vue 3 + TS frontend
  api.ts                  typed invoke() wrappers — the only IPC touchpoint
  App.vue                 sidebar (connections + tables) · main (Data | Supabase tabs)
  components/
    ConnectionList.vue    saved DBs, select/delete
    ConnectionForm.vue    host/port/user/database/password
    TableList.vue         schema browser (tables + views)
    DataGrid.vue          rows as JSON objects → columns from row keys
    SupabasePanel.vue     RLS policies (pg_policies) + auth.users
  __tests__/              Vitest (happy-dom); deferred.spec.ts = todo surface

src-tauri/                Rust backend
  src/connections.rs      profile store (JSON in app config dir) + OS keyring
  src/db.rs               sqlx pool, information_schema/pg_policies/auth.users queries
  src/commands.rs         #[tauri::command] IPC surface
  tests/live_db.rs        #[ignore]d tests against a real Postgres (PGCOVE_TEST_URL)
  tests/deferred.rs       #[ignore] stubs naming deferred-feature issues
```

## Key decisions

- **sqlx (postgres + tls-rustls + json)**, not tokio-postgres: TLS needed for
  Supabase comes as a feature flag, and MySQL/SQLite (issues #3/#4) stay in
  the same crate family.
- **Server-side serialization**: table data is fetched as
  `json_agg(row_to_json(x))`, catalog reads cast `::text`. Zero client-side
  Postgres type mapping in the scaffold; revisit for the query editor.
- **`quote_ident()` for identifiers** — schema/table names are interpolated
  into SQL (no placeholders for identifiers in Postgres), so they're
  double-quoted and escaped; unit-tested.
- **Connection pattern copied from dockshell**: profiles in
  `connections.json`, secret (DB password / Supabase service key) only in
  the OS keyring, store functions take `&Path` for testability.
- **Pool-per-command** (max 2 conns) — simple and correct at scaffold scale;
  cache per connection id when latency matters.
- **CI split** identical to dockshell: cheap ubuntu `ci.yml` on push,
  tag-only cross-platform `release.yml`.

## Shared vs diverged (vs pxx-dex/dockshell)

| Piece | Status |
|---|---|
| Connection manager + keyring | reused pattern (see dockshell) |
| Task/log panel | not present yet — pgcove's equivalent is the status line; revisit with query history |
| API layer | diverged: sqlx over Postgres wire protocol instead of HTTP |
| Supabase panel | new, unique to pgcove |
