//! Commit record deletion and its cleanup intent together; never overwrite changed live files.
use crate::{config, database::Database, error::AppError, skills, DEFAULT_APP_TYPE};
use rusqlite::OptionalExtension;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::PathBuf;

#[derive(Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
enum Target {
    Mcp {
        expected: Option<Value>,
    },
    Skill {
        directory: String,
        expected: Option<Vec<u8>>,
    },
}
#[derive(Serialize, Deserialize)]
struct Intent {
    operation_id: String,
    id: String,
    name: String,
    path: PathBuf,
    target: Target,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PendingStatus {
    pub operation_id: String,
    pub id: String,
    pub name: String,
    pub kind: &'static str,
}
fn read(conn: &rusqlite::Connection) -> Result<Option<Intent>, AppError> {
    let value: Option<String> = conn
        .query_row(
            "SELECT intent_json FROM managed_deletion WHERE id=1",
            [],
            |row| row.get(0),
        )
        .optional()?;
    value
        .map(|value| {
            serde_json::from_str(&value)
                .map_err(|e| AppError::Config(format!("Invalid deletion intent: {e}")))
        })
        .transpose()
}
pub fn pending(db: &Database) -> Result<Option<PendingStatus>, AppError> {
    Ok(read(&db.conn())?.map(|intent| PendingStatus {
        operation_id: intent.operation_id,
        id: intent.id,
        name: intent.name,
        kind: match intent.target {
            Target::Mcp { .. } => "mcp",
            Target::Skill { .. } => "skill",
        },
    }))
}
pub fn ensure_idle(db: &Database) -> Result<(), AppError> {
    if pending(db)?.is_some() {
        return Err(AppError::ProviderInUse(
            "Finish the pending managed deletion before editing, importing or syncing MCP/Skills"
                .into(),
        ));
    }
    Ok(())
}
/// A single transaction protects both the record and durable cleanup intent.
pub fn begin(db: &Database, kind: &str, id: &str) -> Result<bool, AppError> {
    ensure_idle(db)?;
    if crate::database::backups::restore_pending()? {
        return Err(AppError::ProviderInUse(
            "Cancel the planned database restore before deleting managed items".into(),
        ));
    }
    let intent = match kind {
        "mcp" => {
            let records = db.get_all_mcp_servers(DEFAULT_APP_TYPE)?;
            let Some(record) = records.into_iter().find(|record| record.id == id) else {
                return Ok(false);
            };
            let expected = crate::mcp::read_claude_mcp_servers()?.remove(id);
            Intent {
                operation_id: uuid::Uuid::new_v4().to_string(),
                id: id.into(),
                name: record.name,
                path: config::get_claude_mcp_path(),
                target: Target::Mcp { expected },
            }
        }
        "skill" => {
            let records = db.get_all_skills(DEFAULT_APP_TYPE)?;
            let Some(record) = records.into_iter().find(|record| record.id == id) else {
                return Ok(false);
            };
            let (path, expected) = skills::deletion_snapshot(&record.directory)?;
            Intent {
                operation_id: uuid::Uuid::new_v4().to_string(),
                id: id.into(),
                name: record.name,
                path,
                target: Target::Skill {
                    directory: record.directory,
                    expected,
                },
            }
        }
        _ => {
            return Err(AppError::InvalidInput(
                "Unknown managed deletion kind".into(),
            ))
        }
    };
    let bytes =
        serde_json::to_string(&intent).map_err(|source| AppError::JsonSerialize { source })?;
    let mut conn = db.conn();
    let transaction = conn.transaction()?;
    if read(&transaction)?.is_some() {
        return Err(AppError::ProviderInUse(
            "A managed deletion is already pending".into(),
        ));
    }
    let affected = match kind {
        "mcp" => transaction.execute(
            "DELETE FROM mcp_servers WHERE id=?1 AND app_type=?2",
            rusqlite::params![id, DEFAULT_APP_TYPE],
        )?,
        _ => transaction.execute(
            "DELETE FROM skills WHERE id=?1 AND app_type=?2",
            rusqlite::params![id, DEFAULT_APP_TYPE],
        )?,
    };
    if affected == 0 {
        return Ok(false);
    }
    transaction.execute(
        "INSERT INTO managed_deletion (id,intent_json) VALUES (1,?1)",
        [&bytes],
    )?;
    transaction.commit()?;
    Ok(true)
}
/// Idempotent retry: missing live targets mean cleanup already completed.
pub fn finish(db: &Database, operation_id: Option<&str>, keep_files: bool) -> Result<(), AppError> {
    let Some(intent) = read(&db.conn())? else {
        return Ok(());
    };
    if operation_id.is_some_and(|id| id != intent.operation_id) {
        return Err(AppError::ProviderInUse(
            "Deletion changed; refresh recovery status before retrying".into(),
        ));
    }
    if !keep_files {
        match intent.target {
            Target::Mcp { expected } => {
                let path = config::get_claude_mcp_path();
                if path != intent.path {
                    return Err(AppError::Config(
                        "MCP path changed since deletion; review files before finishing".into(),
                    ));
                }
                config::update_json_file(&path, |root| {
                    let Some(node) = root.get_mut("mcpServers") else {
                        return Ok(());
                    };
                    let map = node.as_object_mut().ok_or_else(|| {
                        AppError::Config("mcpServers is not an object; target preserved".into())
                    })?;
                    let Some(current) = map.get(&intent.id) else {
                        return Ok(());
                    };
                    if Some(current) != expected.as_ref() {
                        return Err(AppError::Conflict {
                            path: path.display().to_string(),
                        });
                    }
                    map.remove(&intent.id);
                    Ok(())
                })?;
            }
            Target::Skill {
                directory,
                expected,
            } => {
                let (path, _) = skills::deletion_snapshot(&directory)?;
                if path != intent.path {
                    return Err(AppError::Config(
                        "Skills path changed since deletion; review files before finishing".into(),
                    ));
                }
                skills::remove_captured(&directory, &expected)?;
            }
        }
    }
    // If this commit fails, the intent stays retryable even after live cleanup.
    db.conn()
        .execute("DELETE FROM managed_deletion WHERE id=1", [])?;
    Ok(())
}

/// Older databases may not yet contain the additive intent table.
pub(crate) fn pending_on(conn: &rusqlite::Connection) -> Result<bool, AppError> {
    let exists: bool = conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name='managed_deletion')",
        [],
        |r| r.get(0),
    )?;
    if !exists {
        return Ok(false);
    }
    let count: u32 = conn.query_row("SELECT COUNT(*) FROM managed_deletion", [], |r| r.get(0))?;
    Ok(count != 0)
}
