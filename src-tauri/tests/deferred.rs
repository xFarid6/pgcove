//! Deferred-feature test surface. Each stub names the GitHub issue that will
//! implement it — visible via `cargo test -- --ignored --list`.

#[test]
#[ignore = "not implemented — in-grid data editing, see pgcove issue #2"]
fn updates_a_cell_value_in_grid() {}

#[test]
#[ignore = "not implemented — MySQL support, see pgcove issue #3"]
fn connects_to_mysql() {}

#[test]
#[ignore = "not implemented — SQLite support, see pgcove issue #4"]
fn opens_a_sqlite_file() {}

#[test]
#[ignore = "not implemented — Supabase Management API, see pgcove issue #5"]
fn fetches_project_info_from_management_api() {}

#[test]
#[ignore = "not implemented — RLS policy editor, see pgcove issue #6"]
fn creates_and_drops_an_rls_policy() {}

#[test]
#[ignore = "not implemented — data grid pagination/sorting, see pgcove issue #9"]
fn paginates_table_rows() {}

#[test]
#[ignore = "not implemented — SSH tunnel connections, see pgcove issue #11"]
fn connects_through_ssh_tunnel() {}
