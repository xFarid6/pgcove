// Deferred-feature test surface for the frontend. Each todo names the GitHub
// issue that will implement it — mirrors src-tauri/tests/deferred.rs.
import { describe, test } from "vitest";

describe("deferred features", () => {
  test.todo("query editor syntax highlighting (issue #1 follow-up)");
  test.todo("SQLite native file-picker dialog (issue #4 follow-up — text path input ships now)");
  // Issue #5 shipped: the project/storage sections of SupabasePanel and the
  // Supabase connection form variant are covered in components.spec.ts,
  // api.spec.ts and App.spec.ts. Only the edge-function half is still out of
  // reach — it needs a management access token, not the service-role key.
  test.todo("Supabase edge function list (fast-follow to #5 — needs a management access token)");
  // Issue #6 shipped: RLS policy create/alter/drop + RLS toggle form is
  // covered in components.spec.ts (SupabasePanel emits) and App.spec.ts
  // (confirm-then-execute-then-refresh wiring).
  // Issue #7 shipped: admin-API search/pagination/ban/delete is covered in
  // components.spec.ts (SupabasePanel emits) and App.spec.ts (load/ban/delete
  // wiring). SQL fallback for pooler-only connections stays read-only.
  // Issue #8 shipped: the Structure tab (columns/indexes/constraints,
  // reusing DataGrid) is covered in components.spec.ts and App.spec.ts.
  // DDL editing from that tab is a separate fast-follow, not this issue.
  // Issue #9 shipped: paging/sorting/filtering is covered in
  // components.spec.ts (DataGrid emits) and App.spec.ts (state threading +
  // reload-on-change wiring).
  // Issue #10 shipped: the Migrations tab (status + run pending) is covered
  // in components.spec.ts (MigrationsPanel emits) and App.spec.ts (load/run
  // wiring). Folder picking is a text path input, same call as issue #4's
  // SQLite path field — no native file-picker dialog dependency added.
  // Issue #12 shipped: CSV/JSON export of the grid's current rows is covered
  // in export.spec.ts (CSV escaping, JSON round-trip). Export is a Blob +
  // anchor download in the browser — same "no native dialog dependency"
  // pattern as issues #4/#10 — so there is nothing to add on the Rust side.
});
