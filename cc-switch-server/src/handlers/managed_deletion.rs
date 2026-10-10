use crate::state::AppState;
use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use cc_switch_lib::{database::Database, error::AppError, managed_deletion};
use std::sync::Arc;
fn error(error: AppError, deleted: bool) -> Response {
    (if matches!(error, AppError::Conflict { .. } | AppError::ProviderInUse(_)) { StatusCode::CONFLICT } else { StatusCode::INTERNAL_SERVER_ERROR },
    Json(serde_json::json!({"error": if deleted { format!("Record deleted, but live cleanup is unfinished: {error}. Use the deletion recovery controls.") } else {error.to_string()}, "saved":deleted, "deleted":deleted, "recoveryRequired":deleted}))).into_response()
}
pub(super) fn delete_response(db: &Database, kind: &str, id: &str) -> Response {
    match managed_deletion::begin(db, kind, id) {
        Ok(false) => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({"error":"Managed record not found"})),
        )
            .into_response(),
        Err(e) => error(e, false),
        Ok(true) => match managed_deletion::finish(db, None, false) {
            Ok(()) => Json(serde_json::json!({"success":true,"deleted":true})).into_response(),
            Err(e) => error(e, true),
        },
    }
}
pub async fn status(State(state): State<Arc<AppState>>) -> Response {
    super::file_mutation(move || match managed_deletion::pending(&state.db) {
        Ok(pending) => Json(serde_json::json!({"pending":pending})).into_response(),
        Err(e) => error(e, false),
    })
    .await
}
async fn finish(state: Arc<AppState>, operation_id: String, keep_files: bool) -> Response {
    super::file_mutation(move || {
        let _guard = super::MANAGED_MUTATIONS
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        match managed_deletion::finish(&state.db, Some(&operation_id), keep_files) {
            Ok(()) => {
                Json(serde_json::json!({"success":true,"keptFiles":keep_files})).into_response()
            }
            Err(e) => error(e, true),
        }
    })
    .await
}
#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RecoveryRequest {
    operation_id: String,
}
pub async fn retry(
    State(state): State<Arc<AppState>>,
    Json(request): Json<RecoveryRequest>,
) -> Response {
    finish(state, request.operation_id, false).await
}
pub async fn keep_files(
    State(state): State<Arc<AppState>>,
    Json(request): Json<RecoveryRequest>,
) -> Response {
    finish(state, request.operation_id, true).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::{TestHome, TEST_ENV_LOCK};
    use cc_switch_lib::{
        database::{McpServerRecord, SkillRecord},
        oauth::{codex::CodexOAuthManager, copilot::CopilotAuthManager},
    };
    use serde_json::{json, Value};
    async fn body(response: impl IntoResponse) -> (StatusCode, Value) {
        let response = response.into_response();
        let status = response.status();
        let bytes = axum::body::to_bytes(response.into_body(), 1000000)
            .await
            .unwrap();
        (status, serde_json::from_slice(&bytes).unwrap())
    }
    fn state(home: &TestHome) -> Arc<AppState> {
        Arc::new(AppState {
            db: Arc::new(Database::init().unwrap()),
            token: "test".into(),
            codex_oauth: Arc::new(CodexOAuthManager::new(home.path.clone())),
            copilot_oauth: Arc::new(CopilotAuthManager::new(home.path.clone())),
            proxy_server: Arc::new(tokio::sync::RwLock::new(None)),
            proxy_host: "127.0.0.1".into(),
            proxy_listen_port: 0,
        })
    }
    fn server(id: &str) -> McpServerRecord {
        McpServerRecord {
            id: id.into(),
            name: id.into(),
            app_type: cc_switch_lib::DEFAULT_APP_TYPE.into(),
            enabled: true,
            server_spec: json!({"command":"npx"}),
        }
    }
    #[tokio::test]
    async fn database_delete_failure_leaves_live_untouched_and_cleanup_failure_survives_restart() {
        let _env = TEST_ENV_LOCK.lock().await;
        let home = TestHome::new();
        let state = state(&home);
        let db = &state.db;
        db.save_mcp_server(&server("test")).unwrap();
        let path = cc_switch_lib::config::get_claude_mcp_path();
        let original = r#"{"other":42,"mcpServers":{"test":{"command":"npx","env":{"KEY":"private-key"}},"external":{"command":"keep"}}}"#;
        std::fs::write(&path, original).unwrap();
        let conn = rusqlite::Connection::open(home.path.join(".cc-switch/cc-switch.db")).unwrap();
        conn.execute_batch("CREATE TRIGGER reject_record_delete BEFORE DELETE ON mcp_servers BEGIN SELECT RAISE(ABORT, 'Test deletion failure'); END;").unwrap();
        let (code, report) = body(delete_response(db, "mcp", "test")).await;
        assert_eq!(code, StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(report["deleted"], false);
        assert_eq!(std::fs::read_to_string(&path).unwrap(), original);
        assert!(managed_deletion::pending(db).unwrap().is_none());
        conn.execute_batch("DROP TRIGGER reject_record_delete; CREATE TRIGGER reject_intent_insert BEFORE INSERT ON managed_deletion BEGIN SELECT RAISE(ABORT, 'Test intent insert failure'); END;").unwrap();
        assert_eq!(
            body(delete_response(db, "mcp", "test")).await.0,
            StatusCode::INTERNAL_SERVER_ERROR
        );
        assert_eq!(
            db.get_all_mcp_servers(cc_switch_lib::DEFAULT_APP_TYPE)
                .unwrap()
                .len(),
            1
        );
        assert_eq!(std::fs::read_to_string(&path).unwrap(), original);
        assert!(managed_deletion::pending(db).unwrap().is_none());
        conn.execute_batch("DROP TRIGGER reject_intent_insert; CREATE TRIGGER reject_intent_clear BEFORE DELETE ON managed_deletion BEGIN SELECT RAISE(ABORT, 'Test intent clear failure'); END;").unwrap();
        let (_, report) = body(delete_response(db, "mcp", "test")).await;
        assert_eq!(report["deleted"], true);
        assert_eq!(report["saved"], true);
        assert!(db
            .get_all_mcp_servers(cc_switch_lib::DEFAULT_APP_TYPE)
            .unwrap()
            .is_empty());
        let after: Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
        assert!(after["mcpServers"].get("test").is_none());
        assert_eq!(after["other"], 42);
        assert_eq!(after["mcpServers"]["external"]["command"], "keep");
        // A new connection sees the durable intent without replaying the deleted record.
        let reopened = Database::init().unwrap();
        let pending = managed_deletion::pending(&reopened).unwrap().unwrap();
        assert!(!serde_json::to_string(&pending)
            .unwrap()
            .contains("private-key"));
        assert!(cc_switch_lib::database::backups::create(&reopened).is_err());
        assert_eq!(
            body(crate::handlers::mcp::sync_mcp_servers(State(state.clone())).await)
                .await
                .0,
            StatusCode::CONFLICT
        );
        conn.execute_batch("DROP TRIGGER reject_intent_clear")
            .unwrap();
        managed_deletion::finish(&reopened, Some(&pending.operation_id), false).unwrap();
        assert!(managed_deletion::pending(&reopened).unwrap().is_none());
        assert_eq!(
            serde_json::from_slice::<Value>(&std::fs::read(&path).unwrap()).unwrap(),
            after
        );
    }
    #[tokio::test]
    async fn external_changes_and_stale_recovery_requests_preserve_files() {
        let _env = TEST_ENV_LOCK.lock().await;
        let home = TestHome::new();
        let state = state(&home);
        state.db.save_mcp_server(&server("test")).unwrap();
        let path = cc_switch_lib::config::get_claude_mcp_path();
        std::fs::write(&path, r#"{"mcpServers":{"test":{"command":"original"}}}"#).unwrap();
        managed_deletion::begin(&state.db, "mcp", "test").unwrap();
        let pending = managed_deletion::pending(&state.db).unwrap().unwrap();
        let external = r#"{"mcpServers":{"test":{"command":"external"}},"other":"keep"}"#;
        std::fs::write(&path, external).unwrap();
        let (code, _) = body(
            retry(
                State(state.clone()),
                Json(RecoveryRequest {
                    operation_id: pending.operation_id.clone(),
                }),
            )
            .await,
        )
        .await;
        assert_eq!(code, StatusCode::CONFLICT);
        assert_eq!(std::fs::read_to_string(&path).unwrap(), external);
        let (code, _) = body(
            keep_files(
                State(state.clone()),
                Json(RecoveryRequest {
                    operation_id: "stale-id".into(),
                }),
            )
            .await,
        )
        .await;
        assert_eq!(code, StatusCode::CONFLICT);
        assert!(managed_deletion::pending(&state.db).unwrap().is_some());
        let (code, _) = body(
            keep_files(
                State(state.clone()),
                Json(RecoveryRequest {
                    operation_id: pending.operation_id,
                }),
            )
            .await,
        )
        .await;
        assert_eq!(code, StatusCode::OK);
        assert!(managed_deletion::pending(&state.db).unwrap().is_none());
        assert_eq!(std::fs::read_to_string(&path).unwrap(), external);
        assert!(state
            .db
            .get_all_mcp_servers(cc_switch_lib::DEFAULT_APP_TYPE)
            .unwrap()
            .is_empty());
    }
    #[tokio::test]
    async fn skills_delete_failures_and_success_retain_source_and_backup_history() {
        let _env = TEST_ENV_LOCK.lock().await;
        let home = TestHome::new();
        let state = state(&home);
        let record:SkillRecord = serde_json::from_value(json!({"id":"test","name":"Test","directory":"test","appType":"claude_code","enabled":true})).unwrap();
        state.db.save_skill(&record).unwrap();
        let live = home.path.join(".claude/skills/test");
        let source = home.path.join(".cc-switch/skills/test");
        for dir in [&live, &source] {
            std::fs::create_dir_all(dir).unwrap();
            std::fs::write(dir.join("SKILL.md"), "original").unwrap();
        }
        let conn = rusqlite::Connection::open(home.path.join(".cc-switch/cc-switch.db")).unwrap();
        conn.execute_batch("CREATE TRIGGER reject_skill_delete BEFORE DELETE ON skills BEGIN SELECT RAISE(ABORT, 'Test deletion failure'); END;").unwrap();
        assert_eq!(
            body(delete_response(&state.db, "skill", "test")).await.0,
            StatusCode::INTERNAL_SERVER_ERROR
        );
        assert!(live.join("SKILL.md").is_file());
        assert!(managed_deletion::pending(&state.db).unwrap().is_none());
        conn.execute_batch("DROP TRIGGER reject_skill_delete")
            .unwrap();
        managed_deletion::begin(&state.db, "skill", "test").unwrap();
        let pending = managed_deletion::pending(&state.db).unwrap().unwrap();
        std::fs::write(live.join("SKILL.md"), "external").unwrap();
        assert!(managed_deletion::finish(&state.db, Some(&pending.operation_id), false).is_err());
        assert_eq!(
            std::fs::read_to_string(live.join("SKILL.md")).unwrap(),
            "external"
        );
        std::fs::write(live.join("SKILL.md"), "original").unwrap();
        managed_deletion::finish(&state.db, Some(&pending.operation_id), false).unwrap();
        assert!(!live.exists());
        assert!(source.join("SKILL.md").is_file());
        assert_eq!(cc_switch_lib::skills::list_backups().unwrap().len(), 1);
        assert!(managed_deletion::pending(&state.db).unwrap().is_none());
        assert_eq!(
            body(delete_response(&state.db, "skill", "test")).await.0,
            StatusCode::NOT_FOUND
        );
    }
    #[tokio::test]
    async fn removed_plugin_skills_stay_removed_across_import_sync_and_restart() {
        let _env = TEST_ENV_LOCK.lock().await;
        let home = TestHome::new();
        let state = state(&home);
        let record: SkillRecord = serde_json::from_value(json!({"id":"local:test","name":"Test","directory":"test","appType":"claude_code","enabled":true})).unwrap();
        state.db.save_skill(&record).unwrap();
        for dir in [
            home.path.join(".claude/skills/test"),
            home.path.join(".cc-switch/skills/test"),
            home.path.join(".claude/plugins/example/skills/test"),
        ] {
            std::fs::create_dir_all(&dir).unwrap();
            std::fs::write(dir.join("SKILL.md"), "original").unwrap();
        }
        managed_deletion::begin(&state.db, "skill", "local:test").unwrap();
        managed_deletion::finish(&state.db, None, false).unwrap();
        let db = Database::init().unwrap();
        let report =
            cc_switch_lib::skills::import_from_claude(&db, cc_switch_lib::DEFAULT_APP_TYPE)
                .unwrap();
        assert_eq!(report.imported, 0);
        assert_eq!(report.skipped, 1);
        assert!(
            cc_switch_lib::skills::discover_imports(&db, cc_switch_lib::DEFAULT_APP_TYPE)
                .unwrap()
                .items
                .is_empty()
        );
        cc_switch_lib::skills::sync_report(&db, cc_switch_lib::DEFAULT_APP_TYPE)
            .unwrap()
            .ensure_success()
            .unwrap();
        assert!(!home.path.join(".claude/skills/test").exists());
        let restored = cc_switch_lib::managed_items::restore(
            &db,
            cc_switch_lib::DEFAULT_APP_TYPE,
            "skill",
            &["test".into()],
        )
        .unwrap();
        assert_eq!(restored.imported, 1);
        assert!(!db.get_all_skills(cc_switch_lib::DEFAULT_APP_TYPE).unwrap()[0].enabled);
        assert!(!cc_switch_lib::managed_items::is_removed(
            &db,
            cc_switch_lib::DEFAULT_APP_TYPE,
            "skill",
            "test"
        )
        .unwrap());
        assert!(!home.path.join(".claude/skills/test").exists());
    }

    #[tokio::test]
    async fn removed_mcp_ignores_external_reappearance_and_restores_disabled_without_exposing_credentials(
    ) {
        let _env = TEST_ENV_LOCK.lock().await;
        let home = TestHome::new();
        let state = state(&home);
        let mut record = server("test");
        record.server_spec = json!({"command":"npx","env":{"TOKEN":"private-secret"}});
        state.db.save_mcp_server(&record).unwrap();
        let path = cc_switch_lib::config::get_claude_mcp_path();
        std::fs::write(
            &path,
            serde_json::to_vec(&json!({"mcpServers":{"test":record.server_spec}})).unwrap(),
        )
        .unwrap();
        managed_deletion::begin(&state.db, "mcp", "test").unwrap();
        managed_deletion::finish(&state.db, None, false).unwrap();
        std::fs::write(&path, r#"{"mcpServers":{"test":{"command":"external"}}}"#).unwrap();
        assert_eq!(
            cc_switch_lib::mcp::import_from_claude(&state.db, cc_switch_lib::DEFAULT_APP_TYPE)
                .unwrap()
                .imported,
            0
        );
        assert!(
            cc_switch_lib::mcp::discover_imports(&state.db, cc_switch_lib::DEFAULT_APP_TYPE)
                .unwrap()
                .items
                .is_empty()
        );
        let removed = cc_switch_lib::managed_items::removed(
            &state.db,
            cc_switch_lib::DEFAULT_APP_TYPE,
            "mcp",
        )
        .unwrap();
        assert!(!serde_json::to_string(&removed)
            .unwrap()
            .contains("private-secret"));
        let before = std::fs::read(&path).unwrap();
        let conn = rusqlite::Connection::open(home.path.join(".cc-switch/cc-switch.db")).unwrap();
        conn.execute_batch("CREATE TRIGGER block_marker_clear BEFORE DELETE ON managed_removed BEGIN SELECT RAISE(ABORT,'Cannot clear removal'); END;").unwrap();
        let failed = cc_switch_lib::managed_items::restore(
            &state.db,
            cc_switch_lib::DEFAULT_APP_TYPE,
            "mcp",
            &["test".into()],
        )
        .unwrap();
        assert_eq!(failed.failed, 1);
        assert!(state
            .db
            .get_all_mcp_servers(cc_switch_lib::DEFAULT_APP_TYPE)
            .unwrap()
            .is_empty());
        assert!(cc_switch_lib::managed_items::is_removed(
            &state.db,
            cc_switch_lib::DEFAULT_APP_TYPE,
            "mcp",
            "test"
        )
        .unwrap());
        conn.execute_batch("DROP TRIGGER block_marker_clear")
            .unwrap();
        cc_switch_lib::managed_items::restore(
            &state.db,
            cc_switch_lib::DEFAULT_APP_TYPE,
            "mcp",
            &["test".into()],
        )
        .unwrap()
        .ensure_success()
        .unwrap();
        assert!(
            !state
                .db
                .get_all_mcp_servers(cc_switch_lib::DEFAULT_APP_TYPE)
                .unwrap()[0]
                .enabled
        );
        assert_eq!(std::fs::read(&path).unwrap(), before);
    }

    #[tokio::test]
    async fn removal_marker_failure_rolls_back_record_and_keeps_live_files() {
        let _env = TEST_ENV_LOCK.lock().await;
        let home = TestHome::new();
        let state = state(&home);
        state.db.save_mcp_server(&server("test")).unwrap();
        let path = cc_switch_lib::config::get_claude_mcp_path();
        std::fs::write(&path, r#"{"mcpServers":{"test":{"command":"npx"}}}"#).unwrap();
        let before = std::fs::read(&path).unwrap();
        let conn = rusqlite::Connection::open(home.path.join(".cc-switch/cc-switch.db")).unwrap();
        conn.execute_batch("CREATE TRIGGER reject_removed BEFORE INSERT ON managed_removed BEGIN SELECT RAISE(ABORT,'Cannot retain removal'); END;").unwrap();
        assert!(managed_deletion::begin(&state.db, "mcp", "test").is_err());
        assert_eq!(
            state
                .db
                .get_all_mcp_servers(cc_switch_lib::DEFAULT_APP_TYPE)
                .unwrap()
                .len(),
            1
        );
        assert!(managed_deletion::pending(&state.db).unwrap().is_none());
        assert_eq!(std::fs::read(&path).unwrap(), before);
    }

    #[tokio::test]
    async fn discovery_is_read_only_and_import_adds_only_selected_items() {
        let _env = TEST_ENV_LOCK.lock().await;
        let home = TestHome::new();
        let state = state(&home);
        for name in ["one", "two"] {
            let dir = home.path.join(".claude/plugins/example/skills").join(name);
            std::fs::create_dir_all(&dir).unwrap();
            std::fs::write(dir.join("SKILL.md"), name).unwrap();
        }
        let discovery =
            cc_switch_lib::skills::discover_imports(&state.db, cc_switch_lib::DEFAULT_APP_TYPE)
                .unwrap();
        assert_eq!(discovery.items.len(), 2);
        assert!(state
            .db
            .get_all_skills(cc_switch_lib::DEFAULT_APP_TYPE)
            .unwrap()
            .is_empty());
        assert!(!home.path.join(".cc-switch/skills").exists());
        let report = cc_switch_lib::skills::import_selected(
            &state.db,
            cc_switch_lib::DEFAULT_APP_TYPE,
            &["one".into(), "../../escape".into()],
        )
        .unwrap();
        assert_eq!(report.imported, 1);
        assert_eq!(report.failed, 1);
        assert_eq!(
            state
                .db
                .get_all_skills(cc_switch_lib::DEFAULT_APP_TYPE)
                .unwrap()[0]
                .directory,
            "one"
        );
        assert!(!home.path.join(".cc-switch/skills/two").exists());
    }
}
