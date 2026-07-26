//! Saved database connections. Non-secret fields live in a JSON file in the
//! app config dir; the password (or Supabase service key) lives only in the
//! OS keyring — never on disk, never logged.
//!
//! Same pattern as proxmox-desktop/dockshell `connections.rs`; extraction
//! into a shared crate is issue #14.

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

const KEYRING_SERVICE: &str = "pgcove";

/// Which wire protocol/driver a connection uses.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DbKind {
    #[default]
    Postgres,
    Sqlite,
}

/// One saved connection = one database. For Supabase this is the project's
/// Postgres pooler (host like `aws-0-<region>.pooler.supabase.com`, user like
/// `postgres.<project-ref>`); first-class Management API support is issue #5.
///
/// For `DbKind::Sqlite`, only `database` is meaningful — it holds the file
/// path (or `:memory:`); `host`/`port`/`user` are ignored and the password
/// is never written to the keyring.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConnectionInfo {
    pub id: String,
    pub name: String,
    /// Defaults to Postgres so existing `connections.json` entries (written
    /// before SQLite support existed) still deserialize.
    #[serde(default)]
    pub kind: DbKind,
    pub host: String,
    pub port: u16,
    pub user: String,
    pub database: String,
}

fn store_file(dir: &Path) -> PathBuf {
    dir.join("connections.json")
}

pub fn load(dir: &Path) -> Result<Vec<ConnectionInfo>, String> {
    let path = store_file(dir);
    if !path.exists() {
        return Ok(Vec::new());
    }
    let raw = fs::read_to_string(path).map_err(|e| e.to_string())?;
    serde_json::from_str(&raw).map_err(|e| e.to_string())
}

fn save_all(dir: &Path, conns: &[ConnectionInfo]) -> Result<(), String> {
    fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    let raw = serde_json::to_string_pretty(conns).map_err(|e| e.to_string())?;
    fs::write(store_file(dir), raw).map_err(|e| e.to_string())
}

pub fn get(dir: &Path, id: &str) -> Result<ConnectionInfo, String> {
    load(dir)?
        .into_iter()
        .find(|c| c.id == id)
        .ok_or_else(|| format!("unknown connection: {id}"))
}

fn secret_entry(id: &str) -> Result<keyring::Entry, String> {
    keyring::Entry::new(KEYRING_SERVICE, id).map_err(|e| e.to_string())
}

pub fn get_password(id: &str) -> Result<String, String> {
    secret_entry(id)?.get_password().map_err(|e| e.to_string())
}

/// Upsert a connection; `password` is written to the keyring when provided
/// (add, or edit that changes it).
pub fn save(dir: &Path, info: ConnectionInfo, password: Option<String>) -> Result<(), String> {
    if let Some(p) = password {
        secret_entry(&info.id)?
            .set_password(&p)
            .map_err(|e| e.to_string())?;
    }
    let mut conns = load(dir)?;
    match conns.iter_mut().find(|c| c.id == info.id) {
        Some(existing) => *existing = info,
        None => conns.push(info),
    }
    save_all(dir, &conns)
}

pub fn delete(dir: &Path, id: &str) -> Result<(), String> {
    // Best effort — the entry may already be gone.
    if let Ok(entry) = secret_entry(id) {
        let _ = entry.delete_credential();
    }
    let mut conns = load(dir)?;
    conns.retain(|c| c.id != id);
    save_all(dir, &conns)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn conn(id: &str) -> ConnectionInfo {
        ConnectionInfo {
            id: id.into(),
            name: format!("db {id}"),
            kind: DbKind::Postgres,
            host: "localhost".into(),
            port: 5432,
            user: "postgres".into(),
            database: "postgres".into(),
        }
    }

    #[test]
    fn save_load_delete_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(load(dir.path()).unwrap(), vec![]);

        save(dir.path(), conn("a"), None).unwrap();
        save(dir.path(), conn("b"), None).unwrap();
        assert_eq!(load(dir.path()).unwrap().len(), 2);

        // Upsert replaces, not duplicates.
        let mut edited = conn("a");
        edited.name = "renamed".into();
        save(dir.path(), edited, None).unwrap();
        assert_eq!(load(dir.path()).unwrap().len(), 2);
        assert_eq!(get(dir.path(), "a").unwrap().name, "renamed");

        delete(dir.path(), "a").unwrap();
        assert!(get(dir.path(), "a").is_err());
    }

    // Real OS keyring roundtrip. Ignored in CI (headless ubuntu has no Secret
    // Service); run locally with `cargo test -- --ignored`.
    #[test]
    #[ignore = "requires a real OS keyring; run locally with --ignored"]
    fn keyring_password_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let id = "pgcove-test-keyring";
        save(dir.path(), conn(id), Some("s3cret".into())).unwrap();
        assert_eq!(get_password(id).unwrap(), "s3cret");
        delete(dir.path(), id).unwrap();
        assert!(get_password(id).is_err());
    }
}
