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

static MUTATIONS: std::sync::Mutex<()> = std::sync::Mutex::new(());

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
        log::info!("[Skills] delete_skill id={}", id);
        let skill = match state.db.get_all_skills(APP_TYPE) {
            Ok(skills) => skills.into_iter().find(|skill| skill.id == id),
            Err(e) => {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(json!({"error": e.to_string()})),
                )
                    .into_response()
            }
        };
        let Some(skill) = skill else {
            return (
                StatusCode::NOT_FOUND,
                Json(json!({"error": "Skill not found"})),
            )
                .into_response();
        };
        if let Err(e) = cc_switch_lib::skills::remove_from_claude(&skill.directory) {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": e.to_string()})),
            )
                .into_response();
        }
        match state.db.delete_skill(&id) {
            Ok(true) => Json(json!({ "success": true })).into_response(),
            Ok(false) => (
                StatusCode::NOT_FOUND,
                Json(json!({"error": "Skill not found"})),
            )
                .into_response(),
            Err(e) => {
                log::error!("Failed to delete skill: {}", e);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(json!({"error": e.to_string()})),
                )
                    .into_response()
            }
        }
    })
    .await
}

pub async fn sync_skills(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    log::info!("[Skills] Manual sync requested");
    let state = state.clone();
    match tokio::task::spawn_blocking(move || {
        let _guard = MUTATIONS.lock().unwrap_or_else(|e| e.into_inner());
        cc_switch_lib::skills::sync_enabled_to_claude(&state.db, APP_TYPE)
    })
    .await
    {
        Ok(Ok(())) => Json(json!({ "success": true })).into_response(),
        Ok(Err(e)) => {
            log::error!("Failed to sync skills: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
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
        cc_switch_lib::skills::import_from_claude(&state.db, APP_TYPE)
    })
    .await
    {
        Ok(Ok(count)) => Json(json!({ "success": true, "imported": count })).into_response(),
        Ok(Err(e)) => {
            log::error!("Failed to import skills: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
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

pub async fn toggle_skill(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    super::file_mutation(move || {
        let _guard = MUTATIONS.lock().unwrap_or_else(|e| e.into_inner());
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
        let backup = match cc_switch_lib::skills::list_backups() {
            Ok(backups) => backups.into_iter().find(|backup| backup.id == body.backup_id),
            Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": e.to_string()}))).into_response(),
        };
        let Some(backup) = backup else {
            return (StatusCode::NOT_FOUND, Json(json!({"error": "Backup not found"}))).into_response();
        };
        let records = match state.db.get_all_skills(APP_TYPE) {
            Ok(records) => records,
            Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": e.to_string()}))).into_response(),
        };
        let skill = records.into_iter().find(|skill| skill.directory == backup.directory).unwrap_or_else(|| SkillRecord {
            id: format!("recovered:{}", body.backup_id),
            name: backup.directory.clone(), description: None, directory: backup.directory,
            app_type: APP_TYPE.into(), enabled: true, collection: Some("Recovered".into()),
            installed_at: std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs() as i64,
            repo_owner: None, repo_name: None, repo_branch: None, readme_url: None,
        });
        if let Err(e) = state.db.validate_skill(&skill).and_then(|()| cc_switch_lib::skills::restore_backup(&body.backup_id).map(|_| ())) {
            let status = if matches!(e, cc_switch_lib::error::AppError::Conflict { .. }) { StatusCode::CONFLICT } else { StatusCode::BAD_REQUEST };
            return (status, Json(json!({"error": e.to_string()}))).into_response();
        }
        if let Err(e) = state.db.save_skill(&skill) {
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": format!("Source restored, but record save failed: {e}. Retry Restore."), "saved": true}))).into_response();
        }
        match cc_switch_lib::skills::sync_enabled_to_claude(&state.db, APP_TYPE) {
            Ok(()) => Json(json!({"success": true, "directory": skill.directory, "enabled": skill.enabled})).into_response(),
            Err(e) => super::sync_failure("Skills restoration", &e),
        }
    }).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use cc_switch_lib::database::Database;
    use cc_switch_lib::oauth::{CodexOAuthManager, CopilotAuthManager};
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
    async fn restores_deleted_records_preserves_disabled_state_and_reports_partial_failure() {
        let home = TestHome {
            path: std::env::temp_dir()
                .join(format!("cc-skill-recovery-test-{}", rand::random::<u64>())),
            previous: std::env::var_os("CC_SWITCH_TEST_HOME"),
        };
        fs::create_dir_all(&home.path).unwrap();
        unsafe {
            std::env::set_var("CC_SWITCH_TEST_HOME", &home.path);
        }
        let state = Arc::new(AppState {
            db: Arc::new(Database::init().unwrap()),
            token: "test".into(),
            proxy_listen_port: 15721,
            codex_oauth: Arc::new(CodexOAuthManager::new(home.path.clone())),
            copilot_oauth: Arc::new(CopilotAuthManager::new(home.path.clone())),
            proxy_server: Arc::new(tokio::sync::RwLock::new(None)),
        });
        let name = "test-00000000-0000-0000-0000-000000000000";
        let live_root = home.path.join(".claude/skills");
        let backup = live_root.join(".cc-switch-backups").join(name);
        fs::create_dir_all(&backup).unwrap();
        fs::write(backup.join("SKILL.md"), "restored instructions").unwrap();
        let id = format!("live:{name}");
        let (status, body) = response_json(list_skill_backups(State(state.clone())).await).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["backups"].as_array().unwrap().len(), 1);
        let (status, _) = response_json(
            restore_skill_backup(
                State(state.clone()),
                Json(RestoreBackupRequest {
                    backup_id: "live:../escape".into(),
                }),
            )
            .await,
        )
        .await;
        assert_eq!(status, StatusCode::NOT_FOUND);
        assert!(state.db.get_all_skills(APP_TYPE).unwrap().is_empty());
        let (status, body) = response_json(
            restore_skill_backup(
                State(state.clone()),
                Json(RestoreBackupRequest {
                    backup_id: id.clone(),
                }),
            )
            .await,
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["enabled"], true);
        assert_eq!(
            fs::read_to_string(live_root.join("test/SKILL.md")).unwrap(),
            "restored instructions"
        );
        let record = state.db.get_all_skills(APP_TYPE).unwrap().pop().unwrap();
        assert_eq!(record.directory, "test");
        let (status, _) =
            response_json(toggle_skill(State(state.clone()), Path(record.id.clone())).await).await;
        assert_eq!(status, StatusCode::OK);
        let (status, body) = response_json(
            restore_skill_backup(
                State(state.clone()),
                Json(RestoreBackupRequest {
                    backup_id: id.clone(),
                }),
            )
            .await,
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["enabled"], false);
        assert!(!live_root.join("test").exists());
        assert!(backup.join("SKILL.md").exists());

        #[cfg(unix)]
        {
            let mut record = record;
            record.enabled = true;
            state.db.save_skill(&record).unwrap();
            let external = home.path.join("external");
            fs::create_dir(&external).unwrap();
            fs::write(external.join("SKILL.md"), "user instructions").unwrap();
            std::os::unix::fs::symlink(&external, live_root.join("test")).unwrap();
            fs::write(
                home.path.join(".cc-switch/skills/test/SKILL.md"),
                "changed instructions",
            )
            .unwrap();
            let (status, body) = response_json(
                restore_skill_backup(State(state), Json(RestoreBackupRequest { backup_id: id }))
                    .await,
            )
            .await;
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
