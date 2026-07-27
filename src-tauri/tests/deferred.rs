//! Deferred-feature test surface. Each stub names the GitHub issue that will
//! implement it — visible via `cargo test -- --ignored --list`.

#[test]
#[ignore = "not implemented — in-grid data editing, see pgcove issue #2"]
fn updates_a_cell_value_in_grid() {}

#[test]
#[ignore = "not implemented — MySQL support, see pgcove issue #3"]
fn connects_to_mysql() {}

// Issue #5 (Supabase project connections) shipped: project info and storage
// bucket listing over the project URL + service-role key are covered for real
// by the unit tests in src/supabase.rs (URL/header construction and response
// parsing) plus the env-gated end-to-end test below. What is left deferred is
// only the part a service-role key genuinely cannot do — see src/supabase.rs.
#[tokio::test]
#[ignore = "requires a live Supabase project — set PGCOVE_TEST_SUPABASE_URL and PGCOVE_TEST_SUPABASE_KEY"]
async fn fetches_project_info_and_buckets_from_a_live_project() {
    let url = std::env::var("PGCOVE_TEST_SUPABASE_URL")
        .expect("set PGCOVE_TEST_SUPABASE_URL=https://<ref>.supabase.co");
    let key = std::env::var("PGCOVE_TEST_SUPABASE_KEY")
        .expect("set PGCOVE_TEST_SUPABASE_KEY=<service-role key>");
    let client = pgcove_lib::supabase::SupabaseClient::new(&url, &key).unwrap();
    let info = client.project_info().await.unwrap();
    assert!(
        !info.rest_version.is_empty(),
        "no PostgREST version: {info:?}"
    );
    // Empty on a project with no buckets — not erroring is the point.
    client.list_buckets().await.unwrap();
}

#[test]
#[ignore = "not implemented — edge function listing needs a Supabase management access token (api.supabase.com), not the service-role key; fast-follow to pgcove issue #5"]
fn lists_edge_functions_via_management_api() {}

#[test]
#[ignore = "not implemented — SSH tunnel connections, see pgcove issue #11"]
fn connects_through_ssh_tunnel() {}
