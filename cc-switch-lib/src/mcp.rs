//! MCP (Model Context Protocol) sync module
//!
//! Handles syncing MCP server configurations from the CC Switch database
//! to Claude Code's live config file (~/.claude.json).

use crate::config;
use crate::database::{Database, McpServerRecord};
use crate::error::AppError;
use crate::operation_report::OperationReport;
use serde_json::Value;
use std::collections::HashMap;

/// Read ~/.claude.json as a JSON Value.
/// Returns an empty object if the file doesn't exist.
fn read_claude_mcp_json() -> Result<Value, AppError> {
    let path = config::get_claude_mcp_path();
    if !path.exists() {
        return Ok(serde_json::json!({}));
    }
    config::read_json_file(&path)
}

/// Sync all enabled MCP servers from the database to ~/.claude.json.
///
/// Merges managed entries into `mcpServers`; unrelated servers are preserved.
/// Disabled managed entries are removed.
/// Other root fields (e.g. `hasCompletedOnboarding`) are preserved.
pub fn sync_enabled_to_claude(db: &Database, app_type: &str) -> Result<(), AppError> {
    let servers = db.get_all_mcp_servers(app_type)?;

    config::update_json_file(&config::get_claude_mcp_path(), |root| {
        merge_managed(root, &servers)
    })?;

    log::info!(
        "Synced {} MCP servers to ~/.claude.json for app_type={}",
        servers.len(),
        app_type
    );
    Ok(())
}

fn merge_managed(root: &mut Value, servers: &[McpServerRecord]) -> Result<(), AppError> {
    let root_obj = root
        .as_object_mut()
        .ok_or_else(|| AppError::Config("~/.claude.json root is not an object".into()))?;
    let node = root_obj
        .entry("mcpServers")
        .or_insert_with(|| serde_json::json!({}));
    let map = node.as_object_mut().ok_or_else(|| {
        AppError::Config("mcpServers is not an object; refusing to overwrite".into())
    })?;
    for server in servers {
        validate_server(server)?;
        if server.enabled {
            map.insert(
                server.id.clone(),
                strip_internal_fields(&server.server_spec),
            );
        } else {
            map.remove(&server.id);
        }
    }

    Ok(())
}

/// Remove one managed server without changing external entries.
pub fn remove_from_claude(id: &str) -> Result<(), AppError> {
    config::update_json_file(&config::get_claude_mcp_path(), |root| {
        let obj = root.as_object_mut().expect("validated by writer");
        if let Some(node) = obj.get_mut("mcpServers") {
            let map = node.as_object_mut().ok_or_else(|| {
                AppError::Config("mcpServers is not an object; refusing to overwrite".into())
            })?;
            map.remove(id);
        }
        Ok(())
    })
}

pub fn validate_server(server: &McpServerRecord) -> Result<(), AppError> {
    if server.id.trim().is_empty() || !server.server_spec.is_object() {
        return Err(AppError::InvalidInput(
            "MCP ID must be nonempty and serverSpec must be an object".into(),
        ));
    }
    Ok(())
}

/// Remove internal CC Switch fields from a server spec before writing to live config.
///
/// These fields are used internally by CC Switch for UI management and should
/// never appear in the Claude Code mcpServers config.
fn strip_internal_fields(spec: &Value) -> Value {
    let obj = match spec.as_object() {
        Some(o) => o.clone(),
        None => return spec.clone(),
    };

    let mut cleaned = serde_json::Map::new();
    for (key, value) in obj.iter() {
        match key.as_str() {
            "enabled" | "source" | "id" | "name" | "description" | "tags" | "homepage" | "docs"
            | "apps" => continue,
            _ => {
                cleaned.insert(key.clone(), value.clone());
            }
        }
    }
    Value::Object(cleaned)
}

/// Read the current mcpServers from ~/.claude.json.
pub fn read_claude_mcp_servers() -> Result<HashMap<String, Value>, AppError> {
    let root = read_claude_mcp_json()?;
    parse_mcp_servers(root)
}

fn parse_mcp_servers(root: Value) -> Result<HashMap<String, Value>, AppError> {
    if !root.is_object() {
        return Err(AppError::Config(
            "MCP configuration root must be an object".into(),
        ));
    }
    match root.get("mcpServers") {
        None => Ok(HashMap::new()),
        Some(Value::Object(obj)) => Ok(obj.iter().map(|(k, v)| (k.clone(), v.clone())).collect()),
        Some(_) => Err(AppError::Config("mcpServers must be an object".into())),
    }
}

/// Import valid new records independently; preserve existing managed records.
pub fn import_from_claude(db: &Database, app_type: &str) -> Result<OperationReport, AppError> {
    import_servers(db, app_type, read_claude_mcp_servers()?)
}
fn import_servers(
    db: &Database,
    app_type: &str,
    current: HashMap<String, Value>,
) -> Result<OperationReport, AppError> {
    let existing = db.get_all_mcp_servers(app_type)?;
    let mut current: Vec<_> = current.into_iter().collect();
    current.sort_by(|a, b| a.0.cmp(&b.0));
    let mut report = OperationReport::new();
    for (id, spec) in current {
        if existing.iter().any(|s| s.id == id)
            || crate::managed_items::is_removed(db, app_type, "mcp", &id)?
        {
            report.record(id, "skipped", None);
            continue;
        }
        let server = McpServerRecord {
            id: id.clone(),
            name: id.clone(),
            server_spec: spec,
            app_type: app_type.into(),
            enabled: true,
        };
        match validate_server(&server).and_then(|_| db.save_mcp_server(&server)) {
            Ok(()) => report.record(id, "imported", None),
            Err(error) => report.record(id, "failed", Some(error.to_string())),
        }
    }
    Ok(report)
}

/// MCP is published as a single JSON patch, so every entry shares the write outcome.
pub fn sync_report(db: &Database, app_type: &str) -> Result<OperationReport, AppError> {
    let servers = db.get_all_mcp_servers(app_type)?;
    sync_servers_report(servers)
}

/// Publish only newly imported servers, preserving other managed and external entries.
pub fn sync_selected_report(
    db: &Database,
    app_type: &str,
    ids: &[String],
) -> Result<OperationReport, AppError> {
    let servers: Vec<_> = db
        .get_all_mcp_servers(app_type)?
        .into_iter()
        .filter(|s| ids.contains(&s.id))
        .collect();
    if servers.is_empty() {
        return Ok(OperationReport::new());
    }
    sync_servers_report(servers)
}

fn sync_servers_report(servers: Vec<McpServerRecord>) -> Result<OperationReport, AppError> {
    let result = config::update_json_file(&config::get_claude_mcp_path(), |root| {
        merge_managed(root, &servers)
    });
    let mut report = OperationReport::new();
    let ids: Vec<_> = if servers.is_empty() {
        vec!["MCP configuration".into()]
    } else {
        servers.into_iter().map(|s| s.id).collect()
    };
    for id in ids {
        match &result {
            Ok(()) => report.record(id, "applied", None),
            Err(error) => report.record(id, "failed", Some(error.to_string())),
        }
    }
    Ok(report)
}

pub fn discover_imports(
    db: &Database,
    app_type: &str,
) -> Result<crate::managed_items::Discovery, AppError> {
    let existing = db.get_all_mcp_servers(app_type)?;
    let mut current: Vec<_> = read_claude_mcp_servers()?.into_keys().collect();
    current.sort();
    let mut items = Vec::new();
    for id in current {
        if !existing.iter().any(|s| s.id == id)
            && !crate::managed_items::is_removed(db, app_type, "mcp", &id)?
        {
            items.push(crate::managed_items::Candidate {
                name: id.clone(),
                id,
                source: "Claude Code".into(),
            });
        }
    }
    Ok(crate::managed_items::Discovery {
        items,
        report: OperationReport::new(),
    })
}

pub fn import_selected(
    db: &Database,
    app_type: &str,
    ids: &[String],
) -> Result<OperationReport, AppError> {
    let current = read_claude_mcp_servers()?;
    let mut report = OperationReport::new();
    for id in ids.iter().collect::<std::collections::BTreeSet<_>>() {
        match current.get(id) {
            Some(spec) => {
                let item = import_servers(
                    db,
                    app_type,
                    [(id.to_string(), spec.clone())].into_iter().collect(),
                )?;
                for entry in item.items {
                    report.record(entry.id, entry.outcome, entry.error);
                }
            }
            None => report.record(
                id,
                "failed",
                Some("No longer available. Scan again.".into()),
            ),
        }
    }
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn server(id: &str, enabled: bool) -> McpServerRecord {
        McpServerRecord {
            id: id.into(),
            name: id.into(),
            app_type: "claude_code".into(),
            enabled,
            server_spec: json!({"command": "new", "name": "internal"}),
        }
    }

    #[test]
    fn import_continues_after_validation_and_database_failures_and_retries_safely() {
        let db = Database::test_memory();
        db.conn().execute_batch("CREATE TRIGGER reject_import BEFORE INSERT ON mcp_servers WHEN NEW.id = 'blocked' BEGIN SELECT RAISE(ABORT, 'Test write failure'); END;").unwrap();
        let source = parse_mcp_servers(json!({"mcpServers": {
            "invalid": null, "blocked": {"command":"npx"}, "good": {"command":"ok","env":{"TOKEN":"secret-test"}}
        }})).unwrap();
        let report = import_servers(&db, "claude_code", source.clone()).unwrap();
        assert!(!report.success);
        assert_eq!((report.imported, report.failed), (1, 2));
        assert_eq!(db.get_all_mcp_servers("claude_code").unwrap().len(), 1);
        assert!(!serde_json::to_string(&report)
            .unwrap()
            .contains("secret-test"));
        db.conn()
            .execute_batch("DROP TRIGGER reject_import")
            .unwrap();
        let report = import_servers(&db, "claude_code", source).unwrap();
        assert_eq!((report.imported, report.skipped, report.failed), (1, 1, 1));
        assert_eq!(db.get_all_mcp_servers("claude_code").unwrap().len(), 2);
    }

    #[test]
    fn import_rejects_malformed_nodes_instead_of_claiming_zero_imports() {
        for value in [
            json!([]),
            json!({"mcpServers": null}),
            json!({"mcpServers": []}),
        ] {
            assert!(parse_mcp_servers(value).is_err());
        }
        assert!(parse_mcp_servers(json!({"other": true}))
            .unwrap()
            .is_empty());
    }

    #[test]
    fn invalid_record_cannot_be_applied_to_live() {
        let mut record = server("invalid", true);
        record.server_spec = serde_json::Value::Null;
        assert!(validate_server(&record).is_err());
        record.server_spec = json!({});
        record.id = " ".into();
        assert!(validate_server(&record).is_err());
    }

    #[test]
    fn sync_preserves_external_servers_and_root_fields() {
        let mut root = json!({"mcpServers": {"external": {"command": "keep"}, "enabled": {"command": "old"}, "disabled": {}}, "other": 42});
        merge_managed(
            &mut root,
            &[server("enabled", true), server("disabled", false)],
        )
        .unwrap();
        assert_eq!(root["mcpServers"]["external"], json!({"command": "keep"}));
        assert_eq!(root["mcpServers"]["enabled"], json!({"command": "new"}));
        assert!(root["mcpServers"].get("disabled").is_none());
        assert_eq!(root["other"], 42);
    }

    #[test]
    fn sync_refuses_malformed_config_without_overwriting_it() {
        for mut root in [json!([]), json!({"mcpServers": "invalid"})] {
            let original = root.clone();
            assert!(merge_managed(&mut root, &[server("enabled", true)]).is_err());
            assert_eq!(root, original);
        }
    }

    #[test]
    fn strip_internal_fields_removes_ui_only_keys() {
        let spec = json!({
            "command": "npx",
            "args": ["-y", "@upstash/context7-mcp"],
            "enabled": true,
            "source": "claude",
            "name": "Context7",
            "description": "A test MCP server",
            "tags": ["test"],
            "homepage": "https://example.com",
            "docs": "https://docs.example.com",
            "apps": {"claude": true}
        });

        let cleaned = strip_internal_fields(&spec);
        let obj = cleaned.as_object().unwrap();

        assert_eq!(obj["command"], json!("npx"));
        assert!(obj.get("enabled").is_none());
        assert!(obj.get("source").is_none());
        assert!(obj.get("name").is_none());
    }
}
