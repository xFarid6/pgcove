//! Supabase project APIs reachable with a project URL + service-role key.
//!
//! Scope note (issue #5): a service-role key is a *data-plane* credential —
//! it talks to the project's own host (`https://<ref>.supabase.co`), which
//! gets us the PostgREST root (used here as the project self-check) and the
//! Storage API's bucket list. Listing edge functions is deliberately NOT
//! here: that endpoint lives on the Management API
//! (`api.supabase.com/v1/projects/<ref>/functions`) and authenticates with a
//! personal/management access token, a different credential the user would
//! have to add separately. Stretching the service-role key to cover it would
//! just 401. Adding that second token is the fast-follow to #5 — see the
//! `lists_edge_functions_via_management_api` stub in tests/deferred.rs.
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

/// Small HTTP client for one Supabase project. Not a generic API framework —
/// just base URL + key + the two calls the panel needs.
pub struct SupabaseClient {
    base_url: String,
    service_key: String,
    http: reqwest::Client,
}

impl SupabaseClient {
    pub fn new(project_url: &str, service_key: &str) -> Result<Self, String> {
        if service_key.trim().is_empty() {
            return Err("Supabase service-role key is empty".to_string());
        }
        Ok(Self {
            base_url: normalize_project_url(project_url)?,
            service_key: service_key.trim().to_string(),
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

    /// Project self-check: the PostgREST OpenAPI root is the lightest
    /// endpoint a service-role key can reach on the project host.
    pub async fn project_info(&self) -> Result<ProjectInfo, String> {
        let body = self.get_text("/rest/v1/").await?;
        parse_project_info(&self.base_url, &body)
    }

    pub async fn list_buckets(&self) -> Result<Vec<StorageBucket>, String> {
        parse_buckets(&self.get_text("/storage/v1/bucket").await?)
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
}
