use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use crate::error::AppError;

/// 获取用户主目录，带回退和日志
///
/// ## Windows 注意事项
///
/// - `dirs::home_dir()` 在 Windows 上使用 `SHGetKnownFolderPath(FOLDERID_Profile)`，
///   返回的是真实用户目录（类似 `C:\\Users\\Alice`），与 v3.10.2 行为一致。
/// - 不要直接使用 `HOME` 环境变量：它可能由 Git/Cygwin/MSYS 等第三方工具注入，
///   且不一定等于用户目录，可能导致 `.cc-switch/cc-switch.db` 路径变化，从而“看起来像数据丢失”。
///
/// ## 测试隔离
///
/// 为了让 Windows CI/本地测试能稳定隔离真实用户数据，可通过 `CC_SWITCH_TEST_HOME`
/// 显式覆盖 home dir（仅用于测试/调试场景）。
pub fn get_home_dir() -> PathBuf {
    if let Ok(home) = std::env::var("CC_SWITCH_TEST_HOME") {
        let trimmed = home.trim();
        if !trimmed.is_empty() {
            return PathBuf::from(trimmed);
        }
    }

    dirs::home_dir().unwrap_or_else(|| {
        log::warn!("无法获取用户主目录，回退到当前目录");
        PathBuf::from(".")
    })
}

/// 获取 Claude Code 配置目录路径
pub fn get_claude_config_dir() -> PathBuf {
    if let Some(custom) = crate::settings::get_claude_override_dir() {
        return custom;
    }

    get_home_dir().join(".claude")
}

/// 默认 Claude MCP 配置文件路径 (~/.claude.json)
pub fn get_default_claude_mcp_path() -> PathBuf {
    get_home_dir().join(".claude.json")
}

fn derive_mcp_path_from_override(dir: &Path) -> Option<PathBuf> {
    let file_name = dir
        .file_name()
        .map(|name| name.to_string_lossy().to_string())?
        .trim()
        .to_string();
    if file_name.is_empty() {
        return None;
    }
    let parent = dir.parent().unwrap_or_else(|| Path::new(""));
    Some(parent.join(format!("{file_name}.json")))
}

/// 获取 Claude MCP 配置文件路径，若设置了目录覆盖则与覆盖目录同级
pub fn get_claude_mcp_path() -> PathBuf {
    if let Some(custom_dir) = crate::settings::get_claude_override_dir() {
        if let Some(path) = derive_mcp_path_from_override(&custom_dir) {
            return path;
        }
    }
    get_default_claude_mcp_path()
}

/// 获取 Claude Code 主配置文件路径
pub fn get_claude_settings_path() -> PathBuf {
    let dir = get_claude_config_dir();
    let settings = dir.join("settings.json");
    if settings.exists() {
        return settings;
    }
    // 兼容旧版命名：若存在旧文件则继续使用
    let legacy = dir.join("claude.json");
    if legacy.exists() {
        return legacy;
    }
    // 默认新建：回落到标准文件名 settings.json（不再生成 claude.json）
    settings
}

/// 获取应用配置目录路径 (~/.cc-switch)
pub fn get_app_config_dir() -> PathBuf {
    if let Some(custom) = crate::app_store::get_app_config_dir_override() {
        return custom;
    }

    let default_dir = get_home_dir().join(".cc-switch");

    // 兼容 v3.10.3：当用户环境存在 `HOME` 且与真实用户目录不同，
    // v3.10.3 可能在 `HOME/.cc-switch/` 下创建/使用了数据库。
    // 这里仅在“默认位置没有数据库”时回退到旧位置，避免再次出现“供应商消失”问题，
    // 同时也避免新安装因为 `HOME` 被设置而写入非预期路径。
    #[cfg(windows)]
    {
        let default_db = default_dir.join("cc-switch.db");
        if !default_db.exists() {
            if let Ok(home_env) = std::env::var("HOME") {
                let trimmed = home_env.trim();
                if !trimmed.is_empty() {
                    let legacy_dir = PathBuf::from(trimmed).join(".cc-switch");
                    if legacy_dir.join("cc-switch.db").exists() {
                        log::info!(
                            "Detected v3.10.3 legacy database at {}, using it instead of {}",
                            legacy_dir.display(),
                            default_dir.display()
                        );
                        return legacy_dir;
                    }
                }
            }
        }
    }

    default_dir
}

/// 获取应用配置文件路径
pub fn get_app_config_path() -> PathBuf {
    get_app_config_dir().join("config.json")
}

/// 清理供应商名称，确保文件名安全
#[allow(dead_code)]
pub fn sanitize_provider_name(name: &str) -> String {
    name.chars()
        .map(|c| match c {
            '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*' => '-',
            _ => c,
        })
        .collect::<String>()
        .to_lowercase()
}

/// 获取供应商配置文件路径
#[allow(dead_code)]
pub fn get_provider_config_path(provider_id: &str, provider_name: Option<&str>) -> PathBuf {
    let base_name = provider_name
        .map(sanitize_provider_name)
        .unwrap_or_else(|| sanitize_provider_name(provider_id));

    get_claude_config_dir().join(format!("settings-{base_name}.json"))
}

/// 读取 JSON 配置文件
pub fn read_json_file<T: for<'a> Deserialize<'a>>(path: &Path) -> Result<T, AppError> {
    if !path.exists() {
        return Err(AppError::Config(format!("文件不存在: {}", path.display())));
    }

    let content = fs::read_to_string(path).map_err(|e| AppError::io(path, e))?;

    serde_json::from_str(&content).map_err(|e| AppError::json(path, e))
}

/// 写入 JSON 配置文件
pub fn write_json_file<T: Serialize>(path: &Path, data: &T) -> Result<(), AppError> {
    // 确保目录存在
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| AppError::io(parent, e))?;
    }

    let json =
        serde_json::to_string_pretty(data).map_err(|e| AppError::JsonSerialize { source: e })?;

    atomic_write(path, json.as_bytes())
}

/// 原子写入文本文件（用于 TOML/纯文本）
pub fn write_text_file(path: &Path, data: &str) -> Result<(), AppError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| AppError::io(parent, e))?;
    }
    atomic_write(path, data.as_bytes())
}

/// Write and sync a private temporary file, then atomically replace the target.
/// Failed replacement leaves the existing destination intact and cleans up staging.
pub fn atomic_write(path: &Path, data: &[u8]) -> Result<(), AppError> {
    let staged = stage_write(path, data)?;
    staged
        .persist(path)
        .map_err(|e| AppError::io(path, e.error))?;
    Ok(())
}

fn stage_write(path: &Path, data: &[u8]) -> Result<tempfile::NamedTempFile, AppError> {
    let parent = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    fs::create_dir_all(parent).map_err(|e| AppError::io(parent, e))?;
    let mut staged =
        tempfile::NamedTempFile::new_in(parent).map_err(|e| AppError::io(parent, e))?;
    staged
        .write_all(data)
        .map_err(|e| AppError::io(staged.path(), e))?;
    staged.flush().map_err(|e| AppError::io(staged.path(), e))?;

    // New files retain tempfile's private permissions. Existing Unix permissions
    // remain compatible with user-configured access to the destination.
    #[cfg(unix)]
    match fs::metadata(path) {
        Ok(metadata) => staged
            .as_file()
            .set_permissions(metadata.permissions())
            .map_err(|e| AppError::io(staged.path(), e))?,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(e) => return Err(AppError::io(path, e)),
    }

    staged
        .as_file()
        .sync_all()
        .map_err(|e| AppError::io(staged.path(), e))?;
    Ok(staged)
}

/// Reapply a pure JSON patch against the latest file after an external edit.
/// All in-process users of this helper share a lock. External processes do not;
/// a small race remains between the final byte comparison and publication.
pub fn update_json_file(
    path: &Path,
    patch: impl FnMut(&mut serde_json::Value) -> Result<(), AppError>,
) -> Result<(), AppError> {
    update_json_with_backup(
        path,
        &get_app_config_dir().join("backups/live-first-write"),
        patch,
    )
}

fn read_current(path: &Path) -> Result<Option<Vec<u8>>, AppError> {
    match fs::read(path) {
        Ok(bytes) => Ok(Some(bytes)),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(AppError::io(path, e)),
    }
}

fn refuse_symlink(path: &Path) -> Result<(), AppError> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_symlink() => Err(AppError::InvalidInput(format!(
            "Refusing symlink: {}",
            path.display()
        ))),
        Ok(_) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(AppError::io(path, e)),
    }
}

fn write_private_once(path: &Path, bytes: &[u8]) -> Result<(), AppError> {
    refuse_symlink(path)?;
    if path.exists() {
        return Ok(());
    }
    let mut file = tempfile::NamedTempFile::new_in(path.parent().unwrap())
        .map_err(|e| AppError::io(path, e))?;
    file.write_all(bytes)
        .map_err(|e| AppError::io(file.path(), e))?;
    file.as_file()
        .sync_all()
        .map_err(|e| AppError::io(file.path(), e))?;
    match file.persist_noclobber(path) {
        Ok(_) => Ok(()),
        Err(e) if e.error.kind() == std::io::ErrorKind::AlreadyExists => Ok(()),
        Err(e) => Err(AppError::io(path, e.error)),
    }
}

fn absolute_path(path: &Path) -> Result<PathBuf, AppError> {
    if path.is_absolute() {
        Ok(path.to_path_buf())
    } else {
        Ok(std::env::current_dir()
            .map_err(|e| AppError::io(path, e))?
            .join(path))
    }
}

fn first_write_key(path: &Path) -> String {
    use sha2::{Digest, Sha256};
    hex::encode(Sha256::digest(path.to_string_lossy().as_bytes()))
}

/// Read the immutable first-write snapshot for this fixed configuration path.
/// A target which originally did not exist is represented by an empty object.
pub fn read_first_write_backup(path: &Path) -> Result<Option<serde_json::Value>, AppError> {
    let root = get_app_config_dir().join("backups/live-first-write");
    refuse_symlink(&root)?;
    refuse_symlink(root.parent().unwrap())?;
    let absolute = absolute_path(path)?;
    let key = first_write_key(&absolute);
    let marker = root.join(format!("{key}.source.json"));
    refuse_symlink(&marker)?;
    let Some(bytes) = read_current(&marker)? else {
        return Ok(None);
    };
    let metadata: serde_json::Value =
        serde_json::from_slice(&bytes).map_err(|e| AppError::json(&marker, e))?;
    if metadata.get("path") != Some(&serde_json::json!(absolute)) {
        return Err(AppError::Config(
            "Backup source path does not match configuration".into(),
        ));
    }
    match metadata.get("existed").and_then(serde_json::Value::as_bool) {
        Some(false) => Ok(Some(serde_json::json!({}))),
        Some(true) => {
            let original = root.join(format!("{key}.original"));
            refuse_symlink(&original)?;
            read_json_file(&original).map(Some)
        }
        None => Err(AppError::Config("Invalid backup metadata".into())),
    }
}

fn first_write_backup(root: &Path, path: &Path, current: Option<&[u8]>) -> Result<(), AppError> {
    refuse_symlink(root)?;
    if let Some(parent) = root.parent() {
        refuse_symlink(parent)?;
    }
    fs::create_dir_all(root).map_err(|e| AppError::io(root, e))?;
    let absolute = absolute_path(path)?;
    let key = first_write_key(&absolute);
    let marker = root.join(format!("{key}.source.json"));
    refuse_symlink(&marker)?;
    if marker.exists() {
        return Ok(());
    }
    if let Some(bytes) = current {
        write_private_once(&root.join(format!("{key}.original")), bytes)?;
    }
    let metadata =
        serde_json::to_vec(&serde_json::json!({"path": absolute, "existed": current.is_some()}))
            .map_err(|source| AppError::JsonSerialize { source })?;
    write_private_once(&marker, &metadata)
}

fn update_json_with_backup(
    path: &Path,
    backup_root: &Path,
    mut patch: impl FnMut(&mut serde_json::Value) -> Result<(), AppError>,
) -> Result<(), AppError> {
    static WRITES: std::sync::Mutex<()> = std::sync::Mutex::new(());
    let _guard = WRITES.lock().map_err(|e| AppError::Lock(e.to_string()))?;
    for _ in 0..3 {
        refuse_symlink(path)?;
        let before = read_current(path)?;
        let mut value: serde_json::Value = match before.as_deref() {
            Some(bytes) => serde_json::from_slice(bytes).map_err(|e| AppError::json(path, e))?,
            None => serde_json::json!({}),
        };
        if !value.is_object() {
            return Err(AppError::Config(
                "Configuration root must be an object".into(),
            ));
        }
        let original = value.clone();
        patch(&mut value)?;
        if value == original {
            return Ok(());
        }
        let bytes = serde_json::to_vec_pretty(&value)
            .map_err(|source| AppError::JsonSerialize { source })?;
        let staged = stage_write(path, &bytes)?;
        first_write_backup(backup_root, path, before.as_deref())?;
        refuse_symlink(path)?;
        if read_current(path)? != before {
            continue;
        }
        staged
            .persist(path)
            .map_err(|e| AppError::io(path, e.error))?;
        return Ok(());
    }
    Err(AppError::Conflict {
        path: path.display().to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn guarded_update_rebases_external_edits_and_keeps_first_backup() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("config.json");
        let backups = temp.path().join("backups");
        fs::write(&path, b"{\"original\":true}").unwrap();
        let mut calls = 0;
        update_json_with_backup(&path, &backups, |value| {
            calls += 1;
            value["managed"] = serde_json::json!(true);
            if calls == 1 {
                fs::write(&path, b"{\"external\":42}").unwrap();
            }
            Ok(())
        })
        .unwrap();
        assert_eq!(calls, 2);
        let result: serde_json::Value = read_json_file(&path).unwrap();
        assert_eq!(result, serde_json::json!({"external":42, "managed":true}));
        update_json_with_backup(&path, &backups, |value| {
            value["managed"] = serde_json::json!(false);
            Ok(())
        })
        .unwrap();
        let entries: Vec<_> = fs::read_dir(&backups)
            .unwrap()
            .map(|e| e.unwrap().path())
            .collect();
        assert_eq!(entries.len(), 2);
        let original = entries
            .iter()
            .find(|p| p.extension().is_some_and(|e| e == "original"))
            .unwrap();
        assert_eq!(fs::read(original).unwrap(), b"{\"original\":true}");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(
                fs::metadata(original).unwrap().permissions().mode() & 0o777,
                0o600
            );
        }
    }

    #[test]
    fn sustained_external_writes_return_conflict_and_clean_staging() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("config.json");
        let mut calls = 0;
        let result = update_json_with_backup(&path, &temp.path().join("backups"), |value| {
            calls += 1;
            value["managed"] = serde_json::json!(true);
            fs::write(&path, format!("{{\"external\":{calls}}}")).unwrap();
            Ok(())
        });
        assert!(matches!(result, Err(AppError::Conflict { .. })));
        assert_eq!(calls, 3);
        assert_eq!(fs::read_to_string(&path).unwrap(), "{\"external\":3}");
        assert_eq!(fs::read_dir(temp.path()).unwrap().count(), 2);
    }

    #[test]
    fn noop_and_invalid_json_create_neither_backup_nor_replacement() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("config.json");
        let backups = temp.path().join("backups");
        fs::write(&path, "{ \"managed\": true }").unwrap();
        update_json_with_backup(&path, &backups, |value| {
            value["managed"] = serde_json::json!(true);
            Ok(())
        })
        .unwrap();
        assert_eq!(fs::read_to_string(&path).unwrap(), "{ \"managed\": true }");
        assert!(!backups.exists());
        fs::write(&path, "broken JSON").unwrap();
        assert!(update_json_with_backup(&path, &backups, |_| Ok(())).is_err());
        assert_eq!(fs::read_to_string(&path).unwrap(), "broken JSON");
        assert!(!backups.exists());
    }

    #[test]
    fn atomic_write_replaces_complete_contents_and_cleans_staging() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("config.json");
        fs::write(&path, "old longer contents").unwrap();
        atomic_write(&path, b"new").unwrap();
        assert_eq!(fs::read(&path).unwrap(), b"new");
        assert_eq!(fs::read_dir(temp.path()).unwrap().count(), 1);
    }

    #[test]
    fn failed_replacement_preserves_destination_and_cleans_staging() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("destination");
        fs::create_dir(&path).unwrap();
        fs::write(path.join("keep"), "original").unwrap();
        assert!(atomic_write(&path, b"replacement").is_err());
        assert_eq!(fs::read_to_string(path.join("keep")).unwrap(), "original");
        assert_eq!(fs::read_dir(temp.path()).unwrap().count(), 1);
    }

    #[cfg(unix)]
    #[test]
    fn new_files_are_private_and_replacements_preserve_permissions() {
        use std::os::unix::fs::PermissionsExt;
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("secret.json");
        atomic_write(&path, b"secret").unwrap();
        assert_eq!(
            fs::metadata(&path).unwrap().permissions().mode() & 0o777,
            0o600
        );
        fs::set_permissions(&path, fs::Permissions::from_mode(0o640)).unwrap();
        atomic_write(&path, b"updated").unwrap();
        assert_eq!(
            fs::metadata(&path).unwrap().permissions().mode() & 0o777,
            0o640
        );
    }

    #[test]
    fn derive_mcp_path_from_override_preserves_folder_name() {
        let override_dir = PathBuf::from("/tmp/profile/.claude");
        let derived = derive_mcp_path_from_override(&override_dir)
            .expect("should derive path for nested dir");
        assert_eq!(derived, PathBuf::from("/tmp/profile/.claude.json"));
    }

    #[test]
    fn derive_mcp_path_from_override_handles_non_hidden_folder() {
        let override_dir = PathBuf::from("/data/claude-config");
        let derived = derive_mcp_path_from_override(&override_dir)
            .expect("should derive path for standard dir");
        assert_eq!(derived, PathBuf::from("/data/claude-config.json"));
    }

    #[test]
    fn derive_mcp_path_from_override_supports_relative_rootless_dir() {
        let override_dir = PathBuf::from("claude");
        let derived = derive_mcp_path_from_override(&override_dir)
            .expect("should derive path for single segment");
        assert_eq!(derived, PathBuf::from("claude.json"));
    }

    #[test]
    fn derive_mcp_path_from_root_like_dir_returns_none() {
        let override_dir = PathBuf::from("/");
        assert!(derive_mcp_path_from_override(&override_dir).is_none());
    }
}

/// 复制文件
pub fn copy_file(from: &Path, to: &Path) -> Result<(), AppError> {
    fs::copy(from, to).map_err(|e| AppError::IoContext {
        context: format!("复制文件失败 ({} -> {})", from.display(), to.display()),
        source: e,
    })?;
    Ok(())
}

/// 删除文件
pub fn delete_file(path: &Path) -> Result<(), AppError> {
    if path.exists() {
        fs::remove_file(path).map_err(|e| AppError::io(path, e))?;
    }
    Ok(())
}

/// 检查 Claude Code 配置状态
#[derive(Serialize, Deserialize)]
pub struct ConfigStatus {
    pub exists: bool,
    pub path: String,
}

/// 获取 Claude Code 配置状态
pub fn get_claude_config_status() -> ConfigStatus {
    let path = get_claude_settings_path();
    ConfigStatus {
        exists: path.exists(),
        path: path.to_string_lossy().to_string(),
    }
}
