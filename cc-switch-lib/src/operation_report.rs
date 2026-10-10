//! Batch outcomes contain identifiers and errors, never source configuration contents.
use crate::AppError;
use serde::Serialize;
#[derive(Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OperationReport {
    pub success: bool,
    pub imported: usize,
    pub updated: usize,
    pub applied: usize,
    pub skipped: usize,
    pub failed: usize,
    pub items: Vec<OperationItem>,
}
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OperationItem {
    pub id: String,
    pub outcome: &'static str,
    pub error: Option<String>,
}
impl OperationReport {
    pub fn new() -> Self {
        Self {
            success: true,
            ..Self::default()
        }
    }
    pub fn record(&mut self, id: impl Into<String>, outcome: &'static str, error: Option<String>) {
        match outcome {
            "imported" => self.imported += 1,
            "updated" => self.updated += 1,
            "applied" => self.applied += 1,
            "skipped" => self.skipped += 1,
            "failed" => {
                self.failed += 1;
                self.success = false;
            }
            _ => unreachable!("unknown batch outcome"),
        }
        self.items.push(OperationItem {
            id: id.into(),
            outcome,
            error,
        });
    }
    pub fn ensure_success(self) -> Result<(), AppError> {
        if self.success {
            return Ok(());
        }
        Err(AppError::Config(
            self.items
                .iter()
                .filter(|item| item.outcome == "failed")
                .map(|item| format!("{}: {}", item.id, item.error.as_deref().unwrap_or("Failed")))
                .collect::<Vec<_>>()
                .join("; "),
        ))
    }
}
