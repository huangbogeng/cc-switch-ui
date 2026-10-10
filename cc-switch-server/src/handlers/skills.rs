//! Skills management handlers

use super::super::state::AppState;
use axum::response::IntoResponse;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use cc_switch_lib::database::SkillRecord;
use serde_json::json;
use std::sync::Arc;

use super::MANAGED_MUTATIONS as MUTATIONS;

const APP_TYPE: &str = "claude_code";

pub async fn list_skills(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    match state.db.get_all_skills(APP_TYPE) {
        Ok(skills) => Json(json!({ "skills": skills })).into_response(),
        Err(e) => {
            log::error!("Failed to list skills: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": e.to_string()})),
            )
                .into_response()
        }
    }
}

pub async fn save_skill(
    State(state): State<Arc<AppState>>,
    Json(body): Json<SkillRecord>,
) -> impl IntoResponse {
    super::file_mutation(move || {
        let _guard = MUTATIONS.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(response) = super::managed_idle(&state.db) {
            return response;
        }
        let mut skill = body;
        skill.app_type = APP_TYPE.to_string();
        log::info!("[Skills] save_skill id={} name={}", skill.id, skill.name);
        if let Err(e) = state
            .db
            .validate_skill(&skill)
            .and_then(|()| cc_switch_lib::skills::ensure_skill_ssot(&skill))
        {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({"error": e.to_string()})),
            )
                .into_response();
        }
        match state.db.save_skill(&skill) {
            Ok(()) => match cc_switch_lib::skills::sync_enabled_to_claude(&state.db, APP_TYPE) {
                Ok(()) => Json(json!({"success": true})).into_response(),
                Err(e) => super::sync_failure("Skills", &e),
            },
            Err(e) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": e.to_string()})),
            )
                .into_response(),
        }
    })
    .await
}

pub async fn delete_skill(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    super::file_mutation(move || {
        let _guard = MUTATIONS.lock().unwrap_or_else(|e| e.into_inner());
        super::managed_deletion::delete_response(&state.db, "skill", &id)
    })
    .await
}

pub async fn sync_skills(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    log::info!("[Skills] Manual sync requested");
    let state = state.clone();
    match tokio::task::spawn_blocking(move || {
        let _guard = MUTATIONS.lock().unwrap_or_else(|e| e.into_inner());
        cc_switch_lib::managed_deletion::ensure_idle(&state.db)?;
        cc_switch_lib::skills::sync_report(&state.db, APP_TYPE)
    })
    .await
    {
        Ok(Ok(report)) => Json(report).into_response(),
        Ok(Err(e)) => {
            log::error!("Failed to sync skills: {}", e);
            (
                if matches!(e, cc_switch_lib::AppError::ProviderInUse(_)) {
                    StatusCode::CONFLICT
                } else {
                    StatusCode::INTERNAL_SERVER_ERROR
                },
                Json(json!({"error": e.to_string()})),
            )
                .into_response()
        }
        Err(e) => {
            log::error!("Skills sync task panicked: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "Internal error"})),
            )
                .into_response()
        }
    }
}

pub async fn import_skills(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    log::info!("[Skills] Import from Claude Code skills/plugins requested");
    let state = state.clone();
    match tokio::task::spawn_blocking(move || {
        let _guard = MUTATIONS.lock().unwrap_or_else(|e| e.into_inner());
        cc_switch_lib::managed_deletion::ensure_idle(&state.db)?;
        cc_switch_lib::skills::import_from_claude(&state.db, APP_TYPE)
    })
    .await
    {
        Ok(Ok(report)) => Json(report).into_response(),
        Ok(Err(e)) => {
            log::error!("Failed to import skills: {}", e);
            (
                if matches!(e, cc_switch_lib::AppError::ProviderInUse(_)) {
                    StatusCode::CONFLICT
                } else {
                    StatusCode::INTERNAL_SERVER_ERROR
                },
                Json(json!({"error": e.to_string()})),
            )
                .into_response()
        }
        Err(e) => {
            log::error!("Skills import task panicked: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "Internal error"})),
            )
                .into_response()
        }
    }
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CollectionState {
    collection: String,
    enabled: bool,
}

pub async fn set_collection_enabled(
    State(state): State<Arc<AppState>>,
    Json(body): Json<CollectionState>,
) -> impl IntoResponse {
    super::file_mutation(move || {
        let _guard = MUTATIONS.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(response) = super::managed_idle(&state.db) {
            return response;
        }
        match state
            .db
            .set_skill_collection_enabled(APP_TYPE, &body.collection, body.enabled)
        {
            Ok(0) => (
                StatusCode::NOT_FOUND,
                Json(json!({"error": "Skill collection no longer exists"})),
            )
                .into_response(),
            Ok(updated) => match cc_switch_lib::skills::sync_enabled_to_claude(&state.db, APP_TYPE)
            {
                Ok(()) => {
                    Json(json!({"success": true, "enabled": body.enabled, "updated": updated}))
                        .into_response()
                }
                Err(e) => super::sync_failure("Skills", &e),
            },
            Err(e) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": e.to_string()})),
            )
                .into_response(),
        }
    })
    .await
}

pub async fn toggle_skill(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    super::file_mutation(move || {
        let _guard = MUTATIONS.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(response) = super::managed_idle(&state.db) {
            return response;
        }
        log::info!("[Skills] Toggle enabled for id={}", id);
        match state.db.get_all_skills(APP_TYPE) {
            Ok(skills) => {
                if let Some(skill) = skills.into_iter().find(|s| s.id == id) {
                    let mut toggled = skill.clone();
                    toggled.enabled = !toggled.enabled;
                    match state.db.save_skill(&toggled) {
                        Ok(()) => {
                            match cc_switch_lib::skills::sync_enabled_to_claude(&state.db, APP_TYPE)
                            {
                                Ok(()) => {
                                    Json(json!({ "success": true, "enabled": toggled.enabled }))
                                        .into_response()
                                }
                                Err(e) => super::sync_failure("Skills", &e),
                            }
                        }
                        Err(e) => (
                            StatusCode::INTERNAL_SERVER_ERROR,
                            Json(json!({"error": e.to_string()})),
                        )
                            .into_response(),
                    }
                } else {
                    (
                        StatusCode::NOT_FOUND,
                        Json(json!({"error": "Skill not found"})),
                    )
                        .into_response()
                }
            }
            Err(e) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": e.to_string()})),
            )
                .into_response(),
        }
    })
    .await
}

pub async fn list_skill_backups(State(_state): State<Arc<AppState>>) -> impl IntoResponse {
    super::file_mutation(move || {
        let _guard = MUTATIONS.lock().unwrap_or_else(|e| e.into_inner());
        match cc_switch_lib::skills::list_backups() {
            Ok(backups) => Json(json!({"backups": backups})).into_response(),
            Err(e) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": e.to_string()})),
            )
                .into_response(),
        }
    })
    .await
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RestoreBackupRequest {
    backup_id: String,
}

pub async fn restore_skill_backup(
    State(state): State<Arc<AppState>>,
    Json(body): Json<RestoreBackupRequest>,
) -> impl IntoResponse {
    super::file_mutation(move || {
        let _guard = MUTATIONS.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(response) = super::managed_idle(&state.db) {
            return response;
        }
        restore_backup_response(&state.db, &body.backup_id)
    })
    .await
}

fn restore_backup_response(
    db: &cc_switch_lib::database::Database,
    backup_id: &str,
) -> axum::response::Response {
    let backup = match cc_switch_lib::skills::list_backups() {
        Ok(backups) => backups.into_iter().find(|backup| backup.id == backup_id),
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": e.to_string()})),
            )
                .into_response()
        }
    };
    let Some(backup) = backup else {
        return (
            StatusCode::NOT_FOUND,
            Json(json!({"error": "Backup not found"})),
        )
            .into_response();
    };
    let records = match db.get_all_skills(APP_TYPE) {
        Ok(records) => records,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": e.to_string()})),
            )
                .into_response()
        }
    };
    let skill = records
        .into_iter()
        .find(|skill| skill.directory == backup.directory)
        .unwrap_or_else(|| SkillRecord {
            id: format!("recovered:{}", backup_id),
            name: backup.directory.clone(),
            description: None,
            directory: backup.directory,
            app_type: APP_TYPE.into(),
            enabled: true,
            collection: Some("Recovered".into()),
            installed_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs() as i64,
            repo_owner: None,
            repo_name: None,
            repo_branch: None,
            readme_url: None,
        });
    if let Err(e) = db
        .validate_skill(&skill)
        .and_then(|()| cc_switch_lib::skills::restore_backup(backup_id).map(|_| ()))
    {
        let status = if matches!(e, cc_switch_lib::error::AppError::Conflict { .. }) {
            StatusCode::CONFLICT
        } else {
            StatusCode::BAD_REQUEST
        };
        return (status, Json(json!({"error": e.to_string()}))).into_response();
    }
    if let Err(e) = db.save_skill(&skill) {
        return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": format!("Source restored, but record save failed: {e}. Retry Restore."), "saved": true}))).into_response();
    }
    match cc_switch_lib::skills::sync_enabled_to_claude(db, APP_TYPE) {
        Ok(()) => {
            Json(json!({"success": true, "directory": skill.directory, "enabled": skill.enabled}))
                .into_response()
        }
        Err(e) => super::sync_failure("Skills restoration", &e),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cc_switch_lib::database::Database;
    use std::fs;

    struct TestHome {
        path: std::path::PathBuf,
        previous: Option<std::ffi::OsString>,
    }
    impl Drop for TestHome {
        fn drop(&mut self) {
            match &self.previous {
                Some(value) => unsafe { std::env::set_var("CC_SWITCH_TEST_HOME", value) },
                None => unsafe { std::env::remove_var("CC_SWITCH_TEST_HOME") },
            }
            let _ = fs::remove_dir_all(&self.path);
        }
    }

    async fn response_json(response: impl IntoResponse) -> (StatusCode, serde_json::Value) {
        let response = response.into_response();
        let status = response.status();
        let bytes = axum::body::to_bytes(response.into_body(), 1_000_000)
            .await
            .unwrap();
        (status, serde_json::from_slice(&bytes).unwrap())
    }

    #[tokio::test]
    async fn collection_state_is_atomic_scoped_and_synced_once() {
        use crate::state::{AppState, TestHome as SharedHome};
        use cc_switch_lib::oauth::{codex::CodexOAuthManager, copilot::CopilotAuthManager};
        let _env = crate::state::TEST_ENV_LOCK.lock().await;
        let home = SharedHome::new();
        let db = Arc::new(Database::init().unwrap());
        let state = Arc::new(AppState {
            db: db.clone(),
            token: "test".into(),
            codex_oauth: Arc::new(CodexOAuthManager::new(home.path.clone())),
            copilot_oauth: Arc::new(CopilotAuthManager::new(home.path.clone())),
            proxy_server: Arc::new(tokio::sync::RwLock::new(None)),
            proxy_host: "127.0.0.1".into(),
            proxy_listen_port: 0,
        });
        let live = home.path.join(".claude/skills");
        for name in ["one", "two", "other"] {
            fs::create_dir_all(live.join(name)).unwrap();
            fs::write(live.join(name).join("SKILL.md"), "instructions").unwrap();
        }
        cc_switch_lib::skills::import_from_claude(&db, APP_TYPE).unwrap();
        let conn = rusqlite::Connection::open(home.path.join(".cc-switch/cc-switch.db")).unwrap();
        conn.execute_batch("UPDATE skills SET collection='Team' WHERE directory IN ('one','two'); UPDATE skills SET enabled=0 WHERE directory='two';").unwrap();
        let (status, body) = response_json(
            set_collection_enabled(
                State(state.clone()),
                Json(CollectionState {
                    collection: "Team".into(),
                    enabled: false,
                }),
            )
            .await,
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["updated"], 2);
        assert!(!live.join("one").exists());
        assert!(!live.join("two").exists());
        assert!(live.join("other/SKILL.md").exists());
        // Repeated requests set the target state, rather than toggling it again.
        let (status, _) = response_json(
            set_collection_enabled(
                State(state.clone()),
                Json(CollectionState {
                    collection: "Team".into(),
                    enabled: false,
                }),
            )
            .await,
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        conn.execute_batch("CREATE TRIGGER reject_group BEFORE UPDATE ON skills WHEN NEW.directory='two' BEGIN SELECT RAISE(ABORT, 'group failed'); END;").unwrap();
        let (status, _) = response_json(
            set_collection_enabled(
                State(state.clone()),
                Json(CollectionState {
                    collection: "Team".into(),
                    enabled: true,
                }),
            )
            .await,
        )
        .await;
        assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
        assert!(db
            .get_all_skills(APP_TYPE)
            .unwrap()
            .iter()
            .filter(|s| s.collection.as_deref() == Some("Team"))
            .all(|s| !s.enabled));
        assert!(!live.join("one").exists());
        conn.execute_batch("DROP TRIGGER reject_group").unwrap();
        let (status, _) = response_json(
            set_collection_enabled(
                State(state.clone()),
                Json(CollectionState {
                    collection: "Team".into(),
                    enabled: true,
                }),
            )
            .await,
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert!(live.join("one/SKILL.md").exists());
        assert!(live.join("two/SKILL.md").exists());
        let (status, _) = response_json(
            set_collection_enabled(
                State(state),
                Json(CollectionState {
                    collection: "Missing".into(),
                    enabled: false,
                }),
            )
            .await,
        )
        .await;
        assert_eq!(status, StatusCode::NOT_FOUND);
        conn.execute_batch("UPDATE skills SET collection=NULL WHERE directory='other'")
            .unwrap();
        assert_eq!(
            db.set_skill_collection_enabled(APP_TYPE, "Other", false)
                .unwrap(),
            1
        );
    }

    #[tokio::test]
    async fn batch_import_preserves_partial_progress_and_retries_retained_source_safely() {
        use crate::state::{AppState, TestHome as SharedHome};
        use cc_switch_lib::oauth::{codex::CodexOAuthManager, copilot::CopilotAuthManager};
        let _env = crate::state::TEST_ENV_LOCK.lock().await;
        let home = SharedHome::new();
        let db = Arc::new(Database::init().unwrap());
        let state = Arc::new(AppState {
            db: db.clone(),
            token: "test".into(),
            codex_oauth: Arc::new(CodexOAuthManager::new(home.path.clone())),
            copilot_oauth: Arc::new(CopilotAuthManager::new(home.path.clone())),
            proxy_server: Arc::new(tokio::sync::RwLock::new(None)),
            proxy_host: "127.0.0.1".into(),
            proxy_listen_port: 0,
        });
        let live = home.path.join(".claude/skills");
        for name in ["blocked", "good"] {
            fs::create_dir_all(live.join(name)).unwrap();
            fs::write(live.join(name).join("SKILL.md"), "original").unwrap();
        }
        let conn = rusqlite::Connection::open(home.path.join(".cc-switch/cc-switch.db")).unwrap();
        conn.execute_batch("CREATE TRIGGER reject_skill_import BEFORE INSERT ON skills WHEN NEW.directory='blocked' BEGIN SELECT RAISE(ABORT, 'Test write failure'); END;").unwrap();
        let (status, report) = response_json(import_skills(State(state.clone())).await).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(report["success"], false);
        assert_eq!(report["imported"], 1);
        assert_eq!(report["failed"], 1);
        assert_eq!(db.get_all_skills(APP_TYPE).unwrap().len(), 1);
        let source = home.path.join(".cc-switch/skills/blocked/SKILL.md");
        assert_eq!(fs::read_to_string(&source).unwrap(), "original");
        conn.execute_batch("DROP TRIGGER reject_skill_import")
            .unwrap();
        fs::write(live.join("blocked/SKILL.md"), "changed after failure").unwrap();
        let (_, report) = response_json(import_skills(State(state.clone())).await).await;
        assert_eq!(report["failed"], 1);
        assert_eq!(fs::read_to_string(&source).unwrap(), "original");
        fs::write(live.join("blocked/SKILL.md"), "original").unwrap();
        let (_, report) = response_json(import_skills(State(state.clone())).await).await;
        assert_eq!(report["success"], true);
        assert_eq!(report["imported"], 1);
        assert_eq!(report["skipped"], 1);
        fs::remove_file(&source).unwrap();
        let (_, report) = response_json(sync_skills(State(state.clone())).await).await;
        assert_eq!(report["success"], false);
        assert_eq!(report["skipped"], 1);
        assert_eq!(report["failed"], 1);
        assert_eq!(
            fs::read_to_string(live.join("blocked/SKILL.md")).unwrap(),
            "original"
        );
        let mcp_path = cc_switch_lib::config::get_claude_mcp_path();
        fs::write(&mcp_path, r#"{"mcpServers":{"valid":{"command":"npx","env":{"TOKEN":"do-not-return"}},"invalid":null}}"#).unwrap();
        let (_, report) =
            response_json(crate::handlers::mcp::import_mcp_servers(State(state.clone())).await)
                .await;
        assert_eq!(report["imported"], 1);
        assert_eq!(report["failed"], 1);
        assert!(!report.to_string().contains("do-not-return"));
        fs::write(&mcp_path, "[]").unwrap();
        let (_, report) =
            response_json(crate::handlers::mcp::sync_mcp_servers(State(state.clone())).await).await;
        assert_eq!(report["success"], false);
        assert_eq!(report["applied"], 0);
        assert_eq!(report["failed"], 1);
        assert_eq!(fs::read_to_string(&mcp_path).unwrap(), "[]");
        fs::write(&mcp_path, "{}").unwrap();
        let (_, report) =
            response_json(crate::handlers::mcp::sync_mcp_servers(State(state)).await).await;
        assert_eq!(report["success"], true);
        assert_eq!(report["applied"], 1);
    }

    #[tokio::test]
    async fn restores_deleted_records_preserves_disabled_state_and_reports_partial_failure() {
        let _env = crate::state::TEST_ENV_LOCK.lock().await;
        let home = TestHome {
            path: std::env::temp_dir()
                .join(format!("cc-skill-recovery-test-{}", rand::random::<u64>())),
            previous: std::env::var_os("CC_SWITCH_TEST_HOME"),
        };
        fs::create_dir_all(&home.path).unwrap();
        unsafe {
            std::env::set_var("CC_SWITCH_TEST_HOME", &home.path);
        }
        let db = Database::init().unwrap();
        let name = "test-00000000-0000-0000-0000-000000000000";
        let live_root = home.path.join(".claude/skills");
        let backup = live_root.join(".cc-switch-backups").join(name);
        fs::create_dir_all(&backup).unwrap();
        fs::write(backup.join("SKILL.md"), "restored instructions").unwrap();
        let id = format!("live:{name}");
        assert_eq!(cc_switch_lib::skills::list_backups().unwrap().len(), 1);
        let (status, _) = response_json(restore_backup_response(&db, "live:../escape")).await;
        assert_eq!(status, StatusCode::NOT_FOUND);
        assert!(db.get_all_skills(APP_TYPE).unwrap().is_empty());
        let (status, body) = response_json(restore_backup_response(&db, &id)).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["enabled"], true);
        assert_eq!(
            fs::read_to_string(live_root.join("test/SKILL.md")).unwrap(),
            "restored instructions"
        );
        let record = db.get_all_skills(APP_TYPE).unwrap().pop().unwrap();
        assert_eq!(record.directory, "test");
        let mut disabled = record.clone();
        disabled.enabled = false;
        db.save_skill(&disabled).unwrap();
        cc_switch_lib::skills::sync_enabled_to_claude(&db, APP_TYPE).unwrap();
        let (status, body) = response_json(restore_backup_response(&db, &id)).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["enabled"], false);
        assert!(!live_root.join("test").exists());
        assert!(backup.join("SKILL.md").exists());

        #[cfg(unix)]
        {
            let mut record = record;
            record.enabled = true;
            db.save_skill(&record).unwrap();
            let external = home.path.join("external");
            fs::create_dir(&external).unwrap();
            fs::write(external.join("SKILL.md"), "user instructions").unwrap();
            std::os::unix::fs::symlink(&external, live_root.join("test")).unwrap();
            fs::write(
                home.path.join(".cc-switch/skills/test/SKILL.md"),
                "changed instructions",
            )
            .unwrap();
            let (status, body) = response_json(restore_backup_response(&db, &id)).await;
            assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
            assert_eq!(body["saved"], true);
            assert!(body["error"].as_str().unwrap().contains("Retry Sync"));
            assert_eq!(
                fs::read_to_string(external.join("SKILL.md")).unwrap(),
                "user instructions"
            );
            assert_eq!(
                fs::read_to_string(home.path.join(".cc-switch/skills/test/SKILL.md")).unwrap(),
                "restored instructions"
            );
        }
    }
}
