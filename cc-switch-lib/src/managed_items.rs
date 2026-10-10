//! Discovery is read-only; removal decisions survive later imports and restarts.
use crate::{
    database::{Database, McpServerRecord, SkillRecord},
    operation_report::OperationReport,
    AppError,
};
use rusqlite::{params, OptionalExtension};
use serde::Serialize;

#[derive(Serialize)]
pub struct Candidate {
    pub id: String,
    pub name: String,
    pub source: String,
}
#[derive(Serialize)]
pub struct Discovery {
    pub items: Vec<Candidate>,
    pub report: OperationReport,
}

pub fn validate_kind(kind: &str) -> Result<(), AppError> {
    if matches!(kind, "skill" | "mcp") {
        Ok(())
    } else {
        Err(AppError::InvalidInput("Unknown managed item kind".into()))
    }
}
pub fn is_removed(db: &Database, app: &str, kind: &str, key: &str) -> Result<bool, AppError> {
    Ok(db.conn().query_row("SELECT EXISTS(SELECT 1 FROM managed_removed WHERE app_type=?1 AND kind=?2 AND item_key=?3)", params![app,kind,key], |r| r.get(0))?)
}
pub fn removed(db: &Database, app: &str, kind: &str) -> Result<Vec<Candidate>, AppError> {
    validate_kind(kind)?;
    let conn = db.conn();
    let mut statement = conn.prepare("SELECT item_key,name FROM managed_removed WHERE app_type=?1 AND kind=?2 ORDER BY name,item_key")?;
    let rows = statement.query_map(params![app, kind], |row| {
        Ok(Candidate {
            id: row.get(0)?,
            name: row.get(1)?,
            source: "Removed".into(),
        })
    })?;
    Ok(rows.collect::<Result<_, _>>()?)
}
pub fn restore(
    db: &Database,
    app: &str,
    kind: &str,
    ids: &[String],
) -> Result<OperationReport, AppError> {
    validate_kind(kind)?;
    crate::managed_deletion::ensure_idle(db)?;
    if crate::database::backups::restore_pending()? {
        return Err(AppError::ProviderInUse(
            "Cancel the planned database restore before restoring managed items".into(),
        ));
    }
    let mut report = OperationReport::new();
    for id in ids.iter().collect::<std::collections::BTreeSet<_>>() {
        let result = (|| -> Result<bool, AppError> {
            let json: Option<String> = db.conn().query_row("SELECT record_json FROM managed_removed WHERE app_type=?1 AND kind=?2 AND item_key=?3", params![app,kind,id], |r| r.get(0)).optional()?;
            let Some(json) = json else {
                return Ok(false);
            };
            if kind == "skill" {
                let mut record: SkillRecord =
                    serde_json::from_str(&json).map_err(|e| AppError::Config(e.to_string()))?;
                if record.app_type != app || record.directory != *id {
                    return Err(AppError::Config("Removed record identity mismatch".into()));
                }
                crate::skills::require_retained_source(&record.directory)?;
                record.enabled = false;
                db.save_skill(&record)?;
            } else {
                let mut record: McpServerRecord =
                    serde_json::from_str(&json).map_err(|e| AppError::Config(e.to_string()))?;
                if record.app_type != app || record.id != *id {
                    return Err(AppError::Config("Removed record identity mismatch".into()));
                }
                crate::mcp::validate_server(&record)?;
                record.enabled = false;
                db.save_mcp_server(&record)?;
            }
            Ok(true)
        })();
        match result {
            Ok(true) => report.record(id, "imported", None),
            Ok(false) => report.record(id, "skipped", None),
            Err(error) => report.record(id, "failed", Some(error.to_string())),
        }
    }
    Ok(report)
}
