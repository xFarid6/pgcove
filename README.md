# pgcove

A native database client for Postgres with **first-class Supabase awareness**
— RLS policies and auth users surfaced natively, not "it's just Postgres".
For backend/full-stack developers, especially Supabase users who find the web
dashboard slow for routine table work. MySQL and SQLite are planned (issues
#3/#4).

Built with **Tauri v2 + Vue 3 + TypeScript + Rust**
([sqlx](https://crates.io/crates/sqlx) with rustls). Same architecture as
[proxmox-desktop](https://github.com/xFarid6/proxmox-desktop) and
[dockshell](https://github.com/xFarid6/dockshell) — see
[ARCHITECTURE.md](ARCHITECTURE.md).

**Status: scaffold.** Working today: connection manager (profiles on disk,
password/service key in the OS keyring), schema browser (tables/views), table
data view (`SELECT * … LIMIT 100` rendered in a grid), and a Supabase panel
reading real data from `pg_policies` and `auth.users`. Everything else is a
filed issue — see the repo issues for the v1 plan.

## Quickstart (dev)

Prereqs: Node ≥ 22, pnpm ≥ 11, Rust stable (on this dev machine cargo is not
on PATH in a fresh shell — run
`$env:Path = "$env:USERPROFILE\.cargo\bin;$env:Path"` first in PowerShell).

```sh
pnpm install
pnpm tauri dev
```

Add a connection: any reachable Postgres works. For Supabase use the
**session pooler** credentials from the dashboard (host
`aws-0-<region>.pooler.supabase.com`, port 5432/6543, user
`postgres.<project-ref>`); TLS is negotiated automatically
(`sslmode=prefer`). The password goes to the OS keyring, never to disk.

### Tests

```sh
pnpm test                       # frontend (Vitest)
cd src-tauri; cargo test        # backend unit tests (no DB needed)
PGCOVE_TEST_URL=postgres://u:p@host:5432/db cargo test -- --ignored  # live-DB + keyring tests
```

## Open questions for a human

- **sqlx over tokio-postgres**: chosen for built-in rustls TLS (Supabase
  requires TLS) and a future path to MySQL/SQLite with the same crate.
  Heavier compile, less code.
- **All reads are cast to text/JSON server-side** (`row_to_json`,
  `::text`) so the scaffold needs zero client-side type mapping. The query
  editor (issue #1) and in-grid editing (issue #2) will need real type
  handling — this was a deliberate scaffold shortcut, not the final design.
- **Supabase = pooler connection for now.** The "Supabase project URL +
  service key" connection style from the plan needs the Management API
  (issue #5); the scaffold connects with plain Postgres credentials instead
  so the panel shows real data from day one.
- **No live-DB test in CI**: Postgres wire protocol isn't mockable with
  wiremock; live tests are `#[ignore]`d behind `PGCOVE_TEST_URL`. Consider a
  Postgres service container in CI later.
- **Branch protection**: not enforceable on a free-plan private repo (GitHub
  Pro feature). Treat "CI green before merge" as policy; enable real
  protection if the repo goes public or the account upgrades.
