//! Query history: persisted record of executed queries for quick re-run.

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

const MAX_HISTORY_SIZE: usize = 100;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QueryRecord {
    /// Unique ID (timestamp-based for simplicity).
    pub id: String,
    /// The SQL text executed.
    pub sql: String,
    /// Connection ID used to run this query.
    pub connection_id: String,
    /// ISO 8601 timestamp when the query was executed.
    pub timestamp: String,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct QueriesHistory {
    pub queries: Vec<QueryRecord>,
}

fn history_file(dir: &Path) -> PathBuf {
    dir.join("queries-history.json")
}

/// Load query history from disk; returns empty history if the file doesn't exist.
pub fn load(dir: &Path) -> Result<QueriesHistory, String> {
    let path = history_file(dir);
    if !path.exists() {
        return Ok(QueriesHistory::default());
    }
    let json = fs::read_to_string(&path).map_err(|e| e.to_string())?;
    serde_json::from_str(&json).map_err(|e| e.to_string())
}

/// Save query history to disk, creating parent directory if needed.
pub fn save(dir: &Path, history: &QueriesHistory) -> Result<(), String> {
    fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    let path = history_file(dir);
    let json = serde_json::to_string_pretty(history).map_err(|e| e.to_string())?;
    fs::write(path, json).map_err(|e| e.to_string())
}

/// Add a query to history, capping at MAX_HISTORY_SIZE (newest first).
pub fn add_query(dir: &Path, sql: String, connection_id: String) -> Result<(), String> {
    let mut history = load(dir)?;

    let timestamp = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map_err(|e| e.to_string())?
        .as_millis()
        .to_string();
    let id = timestamp.clone();

    let record = QueryRecord {
        id,
        sql,
        connection_id,
        timestamp,
    };

    history.queries.insert(0, record);
    if history.queries.len() > MAX_HISTORY_SIZE {
        history.queries.truncate(MAX_HISTORY_SIZE);
    }

    save(dir, &history)
}

/// Get all queries in history (newest first).
pub fn list_queries(dir: &Path) -> Result<Vec<QueryRecord>, String> {
    load(dir).map(|h| h.queries)
}

/// Delete a query from history by ID.
pub fn delete_query(dir: &Path, id: &str) -> Result<(), String> {
    let mut history = load(dir)?;
    history.queries.retain(|q| q.id != id);
    save(dir, &history)
}

/// Clear all query history.
pub fn clear_queries(dir: &Path) -> Result<(), String> {
    save(dir, &QueriesHistory::default())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn add_and_list_queries() {
        let dir = tempfile::tempdir().unwrap();
        add_query(dir.path(), "SELECT 1".to_string(), "conn-1".to_string()).unwrap();
        add_query(dir.path(), "SELECT 2".to_string(), "conn-2".to_string()).unwrap();

        let queries = list_queries(dir.path()).unwrap();
        assert_eq!(queries.len(), 2);
        assert_eq!(queries[0].sql, "SELECT 2");
        assert_eq!(queries[1].sql, "SELECT 1");
    }

    #[test]
    fn newest_first() {
        let dir = tempfile::tempdir().unwrap();
        add_query(dir.path(), "a".to_string(), "conn-1".to_string()).unwrap();
        add_query(dir.path(), "b".to_string(), "conn-1".to_string()).unwrap();

        let queries = list_queries(dir.path()).unwrap();
        assert_eq!(queries[0].sql, "b");
        assert_eq!(queries[1].sql, "a");
    }

    #[test]
    fn enforces_max_history_size() {
        let dir = tempfile::tempdir().unwrap();
        for i in 0..150 {
            add_query(dir.path(), format!("SELECT {}", i), "conn-1".to_string()).unwrap();
        }

        let queries = list_queries(dir.path()).unwrap();
        assert_eq!(queries.len(), MAX_HISTORY_SIZE);
        assert_eq!(queries[0].sql, "SELECT 149");
        assert_eq!(queries[MAX_HISTORY_SIZE - 1].sql, "SELECT 50");
    }

    #[test]
    fn delete_query_removes_it() {
        let dir = tempfile::tempdir().unwrap();
        add_query(dir.path(), "SELECT 1".to_string(), "conn-1".to_string()).unwrap();
        let initial_queries = list_queries(dir.path()).unwrap();
        let id = initial_queries[0].id.clone();

        delete_query(dir.path(), &id).unwrap();

        assert_eq!(list_queries(dir.path()).unwrap(), vec![]);
    }

    #[test]
    fn clear_removes_all() {
        let dir = tempfile::tempdir().unwrap();
        add_query(dir.path(), "SELECT 1".to_string(), "conn-1".to_string()).unwrap();
        add_query(dir.path(), "SELECT 2".to_string(), "conn-1".to_string()).unwrap();

        clear_queries(dir.path()).unwrap();

        assert_eq!(list_queries(dir.path()).unwrap(), vec![]);
    }

    #[test]
    fn missing_file_returns_empty() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(list_queries(dir.path()).unwrap(), vec![]);
    }

    #[test]
    fn queries_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        add_query(dir.path(), "SELECT 1".to_string(), "conn-1".to_string()).unwrap();

        let loaded = load(dir.path()).unwrap();
        assert_eq!(loaded.queries.len(), 1);
        assert_eq!(loaded.queries[0].sql, "SELECT 1");
    }
}
