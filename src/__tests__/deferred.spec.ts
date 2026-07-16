// Deferred-feature test surface for the frontend. Each todo names the GitHub
// issue that will implement it — mirrors src-tauri/tests/deferred.rs.
import { describe, test } from "vitest";

describe("deferred features", () => {
  test.todo("query editor with highlighting + run (issue #1)");
  test.todo("in-grid cell editing with type-aware inputs (issue #2)");
  test.todo("MySQL connection form variant (issue #3)");
  test.todo("SQLite file picker connection (issue #4)");
  test.todo("Supabase Management API project panel (issue #5)");
  test.todo("RLS policy create/edit dialog (issue #6)");
  test.todo("auth users search/pagination/actions (issue #7)");
  test.todo("table structure/DDL tab (issue #8)");
  test.todo("data grid pagination + sorting (issue #9)");
  test.todo("export results as CSV/JSON (issue #12)");
});
