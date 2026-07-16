# CLAUDE.md — pgcove

## What this is

Native Postgres/Supabase-first DB client (Tauri v2 + Vue 3 + TS + Rust,
sqlx). Scaffolded sibling of proxmox-desktop, dockshell and hopline — same
stack on purpose, see ARCHITECTURE.md. License is FSL-1.1-MIT (see
LICENSING.md) — don't add code under incompatible licenses.

## Workflow

- One branch + one PR per issue. Small, focused commits.
- CI (`secrets`, `frontend`, `rust`) must be green before merge. Branch
  protection can't be enforced on a free-plan private repo — treat it as
  enforced anyway.
- Board: GitHub Project "pgcove", columns Backlog → To Do → In Progress →
  In Review → Done. Move the issue as you work it.
- Issues state the "why" in the body; keep that when editing scope.

## Windows toolchain quirks (this dev machine)

- cargo/rustc are NOT on PATH in a fresh shell. PowerShell:
  `$env:Path = "$env:USERPROFILE\.cargo\bin;$env:Path"` before any
  cargo/tauri command.
- Package manager is pnpm (v11+). Never npm/yarn.
- Local PostgreSQL 17 service installed (postgres/postgres on 5432):
  `PGCOVE_TEST_URL=postgres://postgres:postgres@localhost:5432/postgres cargo test -- --ignored`
  runs the live-DB tests (verified passing at scaffold time).

## Testing rules

- Backend: `cargo test` in `src-tauri/` runs DB-free unit tests; live-DB
  tests need `PGCOVE_TEST_URL` and `-- --ignored`; keyring roundtrip too.
- Frontend: `pnpm test` (Vitest, happy-dom).
- Every new feature lands with real tests; deferred work gets a
  `test.todo(...)` / `#[ignore = "...issue #N"]` stub naming its issue.
- SQL identifiers must go through `db::quote_ident` — never interpolate raw.
- Lint gates: `cargo fmt --check`, `cargo clippy -- -D warnings`,
  `pnpm lint`, `pnpm typecheck`.
