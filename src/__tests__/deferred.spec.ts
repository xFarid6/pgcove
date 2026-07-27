// Deferred-feature test surface for the frontend. Each todo names the GitHub
// issue that will implement it — mirrors src-tauri/tests/deferred.rs.
import { describe, test } from "vitest";

describe("deferred features", () => {
  test.todo("query editor syntax highlighting (issue #1 follow-up)");
  test.todo("in-grid cell editing with type-aware inputs (issue #2)");
  test.todo("MySQL connection form variant (issue #3)");
  test.todo("SQLite native file-picker dialog (issue #4 follow-up — text path input ships now)");
  // Issue #5 shipped: the project/storage sections of SupabasePanel and the
  // Supabase connection form variant are covered in components.spec.ts,
  // api.spec.ts and App.spec.ts. Only the edge-function half is still out of
  // reach — it needs a management access token, not the service-role key.
  test.todo("Supabase edge function list (fast-follow to #5 — needs a management access token)");
  test.todo("RLS policy create/edit dialog (issue #6)");
  test.todo("auth users search/pagination/actions (issue #7)");
  test.todo("table structure/DDL tab (issue #8)");
  test.todo("data grid pagination + sorting (issue #9)");
  test.todo("export results as CSV/JSON (issue #12)");
});
