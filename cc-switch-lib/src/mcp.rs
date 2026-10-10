//! MCP (Model Context Protocol) sync module
//!
//! Handles syncing MCP server configurations from the CC Switch database
//! to Claude Code's live config file (~/.claude.json).

use crate::config;
use crate::database::{Database, McpServerRecord};
use crate::error::AppError;
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
    let servers = root
        .get("mcpServers")
        .and_then(|v| v.as_object())
        .map(|obj| obj.iter().map(|(k, v)| (k.clone(), v.clone())).collect())
        .unwrap_or_default();
    Ok(servers)
}

/// Import MCP servers from ~/.claude.json into the database.
///
/// Scans the existing `mcpServers` node in ~/.claude.json and creates
/// database records for any servers not already managed by CC Switch.
/// Returns the count of newly imported servers.
pub fn import_from_claude(db: &Database, app_type: &str) -> Result<usize, AppError> {
    let current = read_claude_mcp_servers()?;
    if current.is_empty() {
        return Ok(0);
    }

    let mut imported = 0;
    for (id, spec) in &current {
        let existing = db.get_all_mcp_servers(app_type)?;
        if existing.iter().any(|s| s.id == *id) {
            continue;
        }

        let server = McpServerRecord {
            id: id.clone(),
            name: id.clone(),
            server_spec: spec.clone(),
            app_type: app_type.to_string(),
            enabled: true,
        };
        db.save_mcp_server(&server)?;
        imported += 1;
        log::info!("Imported MCP server '{}' from ~/.claude.json", id);
    }

    log::info!(
        "MCP import complete: {} new servers imported for app_type={}",
        imported,
        app_type
    );
    Ok(imported)
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
