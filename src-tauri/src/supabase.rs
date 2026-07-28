//! Supabase project APIs reachable with a project URL + service-role key
//! (issue #5), plus edge function listing via the Management API with a
//! personal/management access token (issue #30). The service-role key is a
//! data-plane credential (talks to `https://<ref>.supabase.co`); the
//! management token is control-plane (talks to `api.supabase.com`) and is
//! optional — edge functions are only listed when it's provided.
//!
//! reqwest is configured with rustls (no native-tls), matching the sqlx TLS
//! choice in Cargo.toml so the binary has no OpenSSL dependency.

use serde::{Deserialize, Serialize};

const USER_AGENT: &str = concat!("pgcove/", env!("CARGO_PKG_VERSION"));

/// Normalize a pasted project URL: trim whitespace/trailing slashes, and
/// default to https when the user pasted a bare host.
pub fn normalize_project_url(raw: &str) -> Result<String, String> {
    let trimmed = raw.trim().trim_end_matches('/').trim();
    if trimmed.is_empty() {
        return Err("Supabase project URL is empty".to_string());
    }
    if trimmed.starts_with("http://") || trimmed.starts_with("https://") {
        Ok(trimmed.to_string())
    } else {
        Ok(format!("https://{trimmed}"))
    }
}

/// `https://abcdefgh.supabase.co` -> `abcdefgh`. `None` for anything that
/// isn't a `<ref>.supabase.co` host (self-hosted, custom domain, pooler...).
pub fn project_ref(url: &str) -> Option<String> {
    let host = url.split_once("://").map_or(url, |(_, rest)| rest);
    let host = host.split('/').next().unwrap_or_default();
    let (first, rest) = host.split_once('.')?;
    (rest == "supabase.co" && !first.is_empty()).then(|| first.to_string())
}

/// What the Supabase panel shows about the project itself.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectInfo {
    /// Empty for self-hosted/custom-domain projects.
    pub project_ref: String,
    pub url: String,
    /// PostgREST's OpenAPI `info.title`, e.g. "standard public schema".
    pub title: String,
    pub description: String,
    /// PostgREST version string, e.g. "12.2.0 (a1b2c3d)".
    pub rest_version: String,
}

/// A Storage bucket as returned by `GET /storage/v1/bucket`. Supabase sends
/// snake_case timestamps; the aliases let the same struct serialize to the
/// camelCase the frontend expects.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StorageBucket {
    pub id: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub public: bool,
    #[serde(default, alias = "created_at")]
    pub created_at: String,
    #[serde(default, alias = "updated_at")]
    pub updated_at: String,
}

#[derive(Debug, Default, Deserialize)]
struct RestRootInfo {
    #[serde(default)]
    title: String,
    #[serde(default)]
    description: String,
    #[serde(default)]
    version: String,
}

#[derive(Debug, Deserialize)]
struct RestRoot {
    #[serde(default)]
    info: RestRootInfo,
}

/// Parse the PostgREST OpenAPI root into `ProjectInfo`. Split out from the
/// HTTP call so it is unit-testable without a live project.
pub fn parse_project_info(base_url: &str, body: &str) -> Result<ProjectInfo, String> {
    let root: RestRoot =
        serde_json::from_str(body).map_err(|e| format!("unexpected /rest/v1/ response: {e}"))?;
    Ok(ProjectInfo {
        project_ref: project_ref(base_url).unwrap_or_default(),
        url: base_url.to_string(),
        title: root.info.title,
        description: root.info.description,
        rest_version: root.info.version,
    })
}

/// Parse `GET /storage/v1/bucket`. Split out for the same reason.
pub fn parse_buckets(body: &str) -> Result<Vec<StorageBucket>, String> {
    serde_json::from_str(body).map_err(|e| format!("unexpected /storage/v1/bucket response: {e}"))
}

/// A `GET /auth/v1/admin/users` row (issue #7). GoTrue's admin API has no
/// documented server-side email filter, so search is done client-side over
/// whatever page is loaded — see SupabasePanel.vue.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdminUser {
    pub id: String,
    #[serde(default)]
    pub email: String,
    #[serde(default, alias = "created_at")]
    pub created_at: String,
    #[serde(default, alias = "banned_until")]
    pub banned_until: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
struct AdminUsersResponse {
    #[serde(default)]
    users: Vec<AdminUser>,
}

/// Parse `GET /auth/v1/admin/users`. Split out for the same testability reason.
pub fn parse_admin_users(body: &str) -> Result<Vec<AdminUser>, String> {
    let resp: AdminUsersResponse = serde_json::from_str(body)
        .map_err(|e| format!("unexpected /auth/v1/admin/users response: {e}"))?;
    Ok(resp.users)
}

/// An edge function as returned by `GET /v1/projects/<ref>/functions` on the
/// Management API. Supabase sends snake_case names; the aliases let the same
/// struct serialize to camelCase the frontend expects.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EdgeFunction {
    pub id: String,
    #[serde(default)]
    pub name: String,
    pub slug: String,
    #[serde(default)]
    pub status: String,
    #[serde(default, alias = "created_at")]
    pub created_at: String,
    #[serde(default, alias = "updated_at")]
    pub updated_at: String,
}

/// Parse `GET /v1/projects/<ref>/functions` from the Management API.
pub fn parse_edge_functions(body: &str) -> Result<Vec<EdgeFunction>, String> {
    serde_json::from_str(body)
        .map_err(|e| format!("unexpected Management API /functions response: {e}"))
}

/// Small HTTP client for one Supabase project. Not a generic API framework —
/// just base URL + key + the two calls the panel needs.
pub struct SupabaseClient {
    base_url: String,
    service_key: String,
    mgmt_token: Option<String>,
    http: reqwest::Client,
}

impl SupabaseClient {
    pub fn new(project_url: &str, service_key: &str) -> Result<Self, String> {
        Self::new_with_mgmt_token(project_url, service_key, None)
    }

    pub fn new_with_mgmt_token(
        project_url: &str,
        service_key: &str,
        mgmt_token: Option<String>,
    ) -> Result<Self, String> {
        if service_key.trim().is_empty() {
            return Err("Supabase service-role key is empty".to_string());
        }
        Ok(Self {
            base_url: normalize_project_url(project_url)?,
            service_key: service_key.trim().to_string(),
            mgmt_token: mgmt_token.map(|t| t.trim().to_string()),
            http: reqwest::Client::builder()
                .user_agent(USER_AGENT)
                .build()
                .map_err(|e| e.to_string())?,
        })
    }

    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    /// GET builder for `path`, carrying both headers the Supabase gateway
    /// expects: `apikey` (Kong routing) and `Authorization: Bearer`
    /// (PostgREST/Storage authorization).
    pub fn get(&self, path: &str) -> reqwest::RequestBuilder {
        self.http
            .get(format!("{}{path}", self.base_url))
            .header("apikey", &self.service_key)
            .bearer_auth(&self.service_key)
    }

    async fn get_text(&self, path: &str) -> Result<String, String> {
        let resp = self.get(path).send().await.map_err(|e| e.to_string())?;
        let status = resp.status();
        let body = resp.text().await.map_err(|e| e.to_string())?;
        if !status.is_success() {
            return Err(format!(
                "Supabase {path} returned {status}: {}",
                body.trim()
            ));
        }
        Ok(body)
    }

    /// PUT builder, same auth headers as `get`. Used for the admin ban call.
    pub fn put(&self, path: &str) -> reqwest::RequestBuilder {
        self.http
            .put(format!("{}{path}", self.base_url))
            .header("apikey", &self.service_key)
            .bearer_auth(&self.service_key)
    }

    /// DELETE builder, same auth headers as `get`. Used for the admin delete call.
    pub fn delete(&self, path: &str) -> reqwest::RequestBuilder {
        self.http
            .delete(format!("{}{path}", self.base_url))
            .header("apikey", &self.service_key)
            .bearer_auth(&self.service_key)
    }

    async fn check_status(resp: reqwest::Response) -> Result<(), String> {
        let status = resp.status();
        if status.is_success() {
            return Ok(());
        }
        let body = resp.text().await.unwrap_or_default();
        Err(format!(
            "Supabase admin API returned {status}: {}",
            body.trim()
        ))
    }

    /// Project self-check: the PostgREST OpenAPI root is the lightest
    /// endpoint a service-role key can reach on the project host.
    pub async fn project_info(&self) -> Result<ProjectInfo, String> {
        let body = self.get_text("/rest/v1/").await?;
        parse_project_info(&self.base_url, &body)
    }

    pub async fn list_buckets(&self) -> Result<Vec<StorageBucket>, String> {
        parse_buckets(&self.get_text("/storage/v1/bucket").await?)
    }

    pub async fn list_users(&self, page: u32, per_page: u32) -> Result<Vec<AdminUser>, String> {
        parse_admin_users(
            &self
                .get_text(&format!(
                    "/auth/v1/admin/users?page={page}&per_page={per_page}"
                ))
                .await?,
        )
    }

    /// `ban_duration` is a GoTrue duration string like `"24h"` or
    /// `"876000h"` for effectively permanent; pass `"none"` to unban.
    pub async fn ban_user(&self, user_id: &str, ban_duration: &str) -> Result<(), String> {
        let resp = self
            .put(&format!("/auth/v1/admin/users/{user_id}"))
            .json(&serde_json::json!({ "ban_duration": ban_duration }))
            .send()
            .await
            .map_err(|e| e.to_string())?;
        Self::check_status(resp).await
    }

    pub async fn delete_user(&self, user_id: &str) -> Result<(), String> {
        let resp = self
            .delete(&format!("/auth/v1/admin/users/{user_id}"))
            .send()
            .await
            .map_err(|e| e.to_string())?;
        Self::check_status(resp).await
    }

    /// GET builder for the Management API (`api.supabase.com`), carrying the
    /// authorization header for the personal/management access token.
    fn get_mgmt(&self, path: &str) -> Result<reqwest::RequestBuilder, String> {
        let token = self
            .mgmt_token
            .as_ref()
            .ok_or_else(|| "Supabase management token is not set".to_string())?;
        Ok(self
            .http
            .get(format!("https://api.supabase.com{path}"))
            .bearer_auth(token))
    }

    async fn mgmt_get_text(&self, path: &str) -> Result<String, String> {
        let resp = self
            .get_mgmt(path)?
            .send()
            .await
            .map_err(|e| e.to_string())?;
        let status = resp.status();
        let body = resp.text().await.map_err(|e| e.to_string())?;
        if !status.is_success() {
            return Err(format!(
                "Supabase Management API {path} returned {status}: {}",
                body.trim()
            ));
        }
        Ok(body)
    }

    pub async fn list_edge_functions(&self) -> Result<Vec<EdgeFunction>, String> {
        let project_ref = project_ref(&self.base_url).ok_or_else(|| {
            "cannot list edge functions: project URL is not a Supabase project".to_string()
        })?;
        parse_edge_functions(
            &self
                .mgmt_get_text(&format!("/v1/projects/{project_ref}/functions"))
                .await?,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_project_url_defaults_to_https_and_trims() {
        assert_eq!(
            normalize_project_url("  https://abc.supabase.co/  ").unwrap(),
            "https://abc.supabase.co"
        );
        assert_eq!(
            normalize_project_url("abc.supabase.co").unwrap(),
            "https://abc.supabase.co"
        );
        // A local/self-hosted stack over plain http keeps its scheme.
        assert_eq!(
            normalize_project_url("http://localhost:54321").unwrap(),
            "http://localhost:54321"
        );
        assert!(normalize_project_url("   ").is_err());
    }

    #[test]
    fn project_ref_extracts_only_real_project_hosts() {
        assert_eq!(
            project_ref("https://abcdefgh.supabase.co").as_deref(),
            Some("abcdefgh")
        );
        assert_eq!(
            project_ref("https://abcdefgh.supabase.co/rest/v1/").as_deref(),
            Some("abcdefgh")
        );
        assert_eq!(project_ref("http://localhost:54321"), None);
        assert_eq!(project_ref("https://db.example.com"), None);
        // The direct-database host is not a project API host.
        assert_eq!(project_ref("https://db.abcdefgh.supabase.co"), None);
    }

    #[test]
    fn client_rejects_a_blank_service_key() {
        assert!(SupabaseClient::new("https://abc.supabase.co", "  ").is_err());
    }

    #[test]
    fn get_builds_the_url_and_both_auth_headers() {
        let client = SupabaseClient::new("https://abc.supabase.co/", "service-key-123").unwrap();
        let req = client.get("/storage/v1/bucket").build().unwrap();
        assert_eq!(
            req.url().as_str(),
            "https://abc.supabase.co/storage/v1/bucket"
        );
        assert_eq!(req.headers()["apikey"], "service-key-123");
        assert_eq!(req.headers()["authorization"], "Bearer service-key-123");
    }

    #[test]
    fn parses_the_postgrest_root_into_project_info() {
        // Shape of a real GET /rest/v1/ response, trimmed to the fields read.
        let body = r#"{
            "swagger": "2.0",
            "info": {
                "description": "standard public schema",
                "title": "PostgREST API",
                "version": "12.2.0 (a1b2c3d)"
            },
            "paths": {}
        }"#;
        let info = parse_project_info("https://abcdefgh.supabase.co", body).unwrap();
        assert_eq!(
            info,
            ProjectInfo {
                project_ref: "abcdefgh".into(),
                url: "https://abcdefgh.supabase.co".into(),
                title: "PostgREST API".into(),
                description: "standard public schema".into(),
                rest_version: "12.2.0 (a1b2c3d)".into(),
            }
        );
    }

    #[test]
    fn project_info_tolerates_a_root_without_an_info_block() {
        let info = parse_project_info("http://localhost:54321", "{}").unwrap();
        assert_eq!(info.project_ref, "");
        assert_eq!(info.rest_version, "");
    }

    #[test]
    fn project_info_rejects_non_json_bodies() {
        // e.g. an HTML error page from a proxy in front of the project.
        let err = parse_project_info("https://abc.supabase.co", "<html>nope</html>").unwrap_err();
        assert!(err.contains("/rest/v1/"), "unexpected error: {err}");
    }

    #[test]
    fn parses_storage_buckets_with_snake_case_timestamps() {
        let body = r#"[
            {
              "id": "avatars",
              "name": "avatars",
              "owner": "",
              "public": true,
              "file_size_limit": null,
              "allowed_mime_types": null,
              "created_at": "2026-01-02T03:04:05.000Z",
              "updated_at": "2026-01-02T03:04:05.000Z"
            }
        ]"#;
        let buckets = parse_buckets(body).unwrap();
        assert_eq!(
            buckets,
            vec![StorageBucket {
                id: "avatars".into(),
                name: "avatars".into(),
                public: true,
                created_at: "2026-01-02T03:04:05.000Z".into(),
                updated_at: "2026-01-02T03:04:05.000Z".into(),
            }]
        );
    }

    #[test]
    fn buckets_serialize_to_camel_case_for_the_frontend() {
        let json = serde_json::to_value(StorageBucket {
            id: "avatars".into(),
            name: "avatars".into(),
            public: false,
            created_at: "2026-01-02".into(),
            updated_at: "2026-01-03".into(),
        })
        .unwrap();
        assert_eq!(json["createdAt"], "2026-01-02");
        assert_eq!(json["updatedAt"], "2026-01-03");
    }

    #[test]
    fn empty_bucket_list_is_not_an_error() {
        assert_eq!(parse_buckets("[]").unwrap(), vec![]);
    }

    #[test]
    fn parses_admin_users_response() {
        let body = r#"{
            "users": [
                {"id": "u1", "email": "a@example.com", "created_at": "2026-01-02T00:00:00Z", "banned_until": null},
                {"id": "u2", "email": "b@example.com", "created_at": "2026-01-03T00:00:00Z", "banned_until": "2099-01-01T00:00:00Z"}
            ],
            "aud": "authenticated"
        }"#;
        let users = parse_admin_users(body).unwrap();
        assert_eq!(users.len(), 2);
        assert_eq!(users[0].email, "a@example.com");
        assert_eq!(users[0].banned_until, None);
        assert_eq!(
            users[1].banned_until.as_deref(),
            Some("2099-01-01T00:00:00Z")
        );
    }

    #[test]
    fn empty_admin_users_list_is_not_an_error() {
        assert_eq!(parse_admin_users(r#"{"users":[]}"#).unwrap(), vec![]);
    }

    #[test]
    fn admin_users_serialize_to_camel_case() {
        let json = serde_json::to_value(AdminUser {
            id: "u1".into(),
            email: "a@example.com".into(),
            created_at: "2026-01-02".into(),
            banned_until: None,
        })
        .unwrap();
        assert_eq!(json["createdAt"], "2026-01-02");
        assert!(json["bannedUntil"].is_null());
    }

    #[test]
    fn put_and_delete_carry_the_same_auth_headers_as_get() {
        let client = SupabaseClient::new("https://abc.supabase.co", "service-key-123").unwrap();
        let put_req = client.put("/auth/v1/admin/users/u1").build().unwrap();
        assert_eq!(put_req.method(), reqwest::Method::PUT);
        assert_eq!(put_req.headers()["apikey"], "service-key-123");
        let del_req = client.delete("/auth/v1/admin/users/u1").build().unwrap();
        assert_eq!(del_req.method(), reqwest::Method::DELETE);
        assert_eq!(del_req.headers()["authorization"], "Bearer service-key-123");
    }

    #[test]
    fn parses_edge_functions_from_management_api() {
        let body = r#"[
            {
              "id": "func-1",
              "name": "hello",
              "slug": "hello",
              "status": "active",
              "created_at": "2026-01-02T00:00:00Z",
              "updated_at": "2026-01-02T00:00:00Z"
            },
            {
              "id": "func-2",
              "name": "goodbye",
              "slug": "goodbye",
              "status": "active",
              "created_at": "2026-01-03T00:00:00Z",
              "updated_at": "2026-01-03T00:00:00Z"
            }
        ]"#;
        let funcs = parse_edge_functions(body).unwrap();
        assert_eq!(funcs.len(), 2);
        assert_eq!(funcs[0].name, "hello");
        assert_eq!(funcs[1].slug, "goodbye");
    }

    #[test]
    fn edge_functions_serialize_to_camel_case() {
        let json = serde_json::to_value(EdgeFunction {
            id: "f1".into(),
            name: "my-func".into(),
            slug: "my-func".into(),
            status: "active".into(),
            created_at: "2026-01-02".into(),
            updated_at: "2026-01-03".into(),
        })
        .unwrap();
        assert_eq!(json["createdAt"], "2026-01-02");
        assert_eq!(json["updatedAt"], "2026-01-03");
    }

    #[test]
    fn empty_edge_functions_list_is_not_an_error() {
        assert_eq!(parse_edge_functions("[]").unwrap(), vec![]);
    }

    #[test]
    fn client_without_mgmt_token_cannot_list_functions() {
        let client = SupabaseClient::new("https://abc.supabase.co", "service-key-123").unwrap();
        assert!(client.get_mgmt("/v1/test").is_err());
    }

    #[test]
    fn client_with_mgmt_token_builds_management_api_request() {
        let client = SupabaseClient::new_with_mgmt_token(
            "https://abc.supabase.co/",
            "service-key-123",
            Some("mgmt-token-456".into()),
        )
        .unwrap();
        let req = client
            .get_mgmt("/v1/projects/abcdefgh/functions")
            .unwrap()
            .build()
            .unwrap();
        assert_eq!(
            req.url().as_str(),
            "https://api.supabase.com/v1/projects/abcdefgh/functions"
        );
        assert_eq!(req.headers()["authorization"], "Bearer mgmt-token-456");
    }
}
