//! Metadata-only inventory of first-write snapshots for known Claude files.
use super::*;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileBackup {
    pub id: String,
    pub label: String,
    pub source_path: String,
    pub marker_path: String,
    pub original_path: Option<String>,
    pub status: String,
    pub size_bytes: Option<u64>,
    /// Marker modification time; not a guaranteed creation timestamp.
    pub modified_at: Option<u64>,
    pub error: Option<String>,
}

/// Never accepts paths from the client, scans arbitrary directories, or returns contents.
pub fn list() -> Result<Vec<FileBackup>, AppError> {
    let root = get_app_config_dir().join("backups/live-first-write");
    let dir = get_claude_config_dir();
    let targets = [
        (
            "claude-settings",
            "Claude settings",
            dir.join("settings.json"),
        ),
        (
            "claude-legacy",
            "Legacy Claude settings",
            dir.join("claude.json"),
        ),
        (
            "claude-mcp",
            "Claude MCP configuration",
            get_claude_mcp_path(),
        ),
    ];
    targets
        .into_iter()
        .map(|(id, label, path)| inspect(id, label, &path, &root))
        .collect()
}

fn inspect(id: &str, label: &str, path: &Path, root: &Path) -> Result<FileBackup, AppError> {
    let path = absolute_path(path)?;
    let key = first_write_key(&path);
    let marker = root.join(format!("{key}.source.json"));
    let original = root.join(format!("{key}.original"));
    let mut item = FileBackup {
        id: id.into(),
        label: label.into(),
        source_path: path.display().to_string(),
        marker_path: marker.display().to_string(),
        original_path: None,
        status: "missing".into(),
        size_bytes: None,
        modified_at: None,
        error: None,
    };
    let checked = (|| -> Result<(), AppError> {
        // Validate source binding and original JSON without exposing any values.
        let Some(value) = read_first_write_backup_at(&path, root)? else {
            return Ok(());
        };
        if !value.is_object() {
            return Err(AppError::Config(
                "Backup configuration must be a JSON object".into(),
            ));
        }
        let bytes = fs::read(&marker).map_err(|e| AppError::io(&marker, e))?;
        let metadata: serde_json::Value =
            serde_json::from_slice(&bytes).map_err(|e| AppError::json(&marker, e))?;
        let existed = metadata["existed"]
            .as_bool()
            .ok_or_else(|| AppError::Config("Invalid backup metadata".into()))?;
        item.modified_at = fs::metadata(&marker)
            .map_err(|e| AppError::io(&marker, e))?
            .modified()
            .ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_secs());
        if existed {
            item.size_bytes = Some(
                fs::metadata(&original)
                    .map_err(|e| AppError::io(&original, e))?
                    .len(),
            );
            item.original_path = Some(original.display().to_string());
            item.status = "available".into();
        } else {
            item.status = "originally-absent".into();
        }
        Ok(())
    })();
    if let Err(error) = checked {
        item.status = "invalid".into();
        item.error = Some(error.to_string());
    }
    Ok(item)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn inventories_original_and_absent_snapshots_without_credentials() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("backups");
        let path = temp.path().join("settings.json");
        let bytes = br#"{"env":{"ANTHROPIC_AUTH_TOKEN":"secret-test-value"}}"#;
        assert_eq!(
            inspect("settings", "Settings", &path, &root)
                .unwrap()
                .status,
            "missing"
        );
        first_write_backup(&root, &path, Some(bytes)).unwrap();
        let item = inspect("settings", "Settings", &path, &root).unwrap();
        assert_eq!(item.status, "available");
        assert_eq!(item.size_bytes, Some(bytes.len() as u64));
        assert!(item.original_path.is_some());
        assert!(!serde_json::to_string(&item)
            .unwrap()
            .contains("secret-test-value"));
        // Inventory does not require the live target to exist or parse successfully.
        fs::write(&path, "invalid live JSON").unwrap();
        assert_eq!(
            inspect("settings", "Settings", &path, &root)
                .unwrap()
                .status,
            "available"
        );
        let absent = temp.path().join("mcp.json");
        first_write_backup(&root, &absent, None).unwrap();
        let item = inspect("mcp", "MCP", &absent, &root).unwrap();
        assert_eq!(item.status, "originally-absent");
        assert!(item.original_path.is_none());
    }

    #[test]
    fn damaged_or_mismatched_snapshots_are_not_reported_as_missing() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("backups");
        let path = temp.path().join("settings.json");
        first_write_backup(&root, &path, Some(b"{}")).unwrap();
        let key = first_write_key(&path);
        let original = root.join(format!("{key}.original"));
        fs::write(&original, "broken").unwrap();
        assert_eq!(
            inspect("settings", "Settings", &path, &root)
                .unwrap()
                .status,
            "invalid"
        );
        fs::write(&original, "[]").unwrap();
        assert_eq!(
            inspect("settings", "Settings", &path, &root)
                .unwrap()
                .status,
            "invalid"
        );
        fs::remove_file(&original).unwrap();
        assert_eq!(
            inspect("settings", "Settings", &path, &root)
                .unwrap()
                .status,
            "invalid"
        );
        fs::write(
            root.join(format!("{key}.source.json")),
            r#"{"path":"/other","existed":false}"#,
        )
        .unwrap();
        assert_eq!(
            inspect("settings", "Settings", &path, &root)
                .unwrap()
                .status,
            "invalid"
        );
    }

    #[cfg(unix)]
    #[test]
    fn refuses_symlink_snapshots() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("backups");
        let path = temp.path().join("settings.json");
        first_write_backup(&root, &path, Some(b"{}")).unwrap();
        let original = root.join(format!("{}.original", first_write_key(&path)));
        fs::remove_file(&original).unwrap();
        std::os::unix::fs::symlink("/etc/passwd", &original).unwrap();
        let item = inspect("settings", "Settings", &path, &root).unwrap();
        assert_eq!(item.status, "invalid");
        assert!(item.original_path.is_none());
    }
}
