//! Recover skill contents without consuming history or overwriting an external edit.
use super::*;
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SkillBackup {
    pub id: String,
    pub directory: String,
    pub origin: String,
    pub modified_at: i64,
}

fn backup_directory(name: &str) -> Result<String, AppError> {
    let suffix = name
        .len()
        .checked_sub(36)
        .and_then(|offset| name.get(offset..))
        .ok_or_else(|| AppError::InvalidInput("Invalid backup ID".into()))?;
    let uuid = uuid::Uuid::parse_str(suffix)
        .map_err(|_| AppError::InvalidInput("Invalid backup ID".into()))?;
    let directory = name
        .strip_suffix(&format!("-{uuid}"))
        .ok_or_else(|| AppError::InvalidInput("Invalid backup ID".into()))?;
    validate_directory(directory)?;
    Ok(directory.to_string())
}

pub fn list_backups() -> Result<Vec<SkillBackup>, AppError> {
    list_at(&get_ssot_dir(), &get_claude_skills_dir())
}

fn list_at(ssot: &Path, live: &Path) -> Result<Vec<SkillBackup>, AppError> {
    let mut backups = Vec::new();
    for (origin, parent) in [("source", ssot), ("live", live)] {
        reject_symlink(parent)?;
        let root = parent.join(".cc-switch-backups");
        reject_symlink(&root)?;
        if !root.exists() {
            continue;
        }
        for entry in fs::read_dir(&root).map_err(|e| AppError::io(&root, e))? {
            let entry = entry.map_err(|e| AppError::io(&root, e))?;
            let name = entry.file_name().to_string_lossy().to_string();
            let Ok(directory) = backup_directory(&name) else {
                continue;
            };
            reject_symlink(&entry.path())?;
            reject_symlink(&entry.path().join("SKILL.md"))?;
            if !entry.path().join("SKILL.md").is_file() {
                continue;
            }
            let modified_at = entry
                .metadata()
                .map_err(|e| AppError::io(entry.path(), e))?
                .modified()
                .ok()
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|d| d.as_secs() as i64)
                .unwrap_or(0);
            backups.push(SkillBackup {
                id: format!("{origin}:{name}"),
                directory,
                origin: origin.into(),
                modified_at,
            });
        }
    }
    backups.sort_by(|a, b| {
        b.modified_at
            .cmp(&a.modified_at)
            .then_with(|| a.id.cmp(&b.id))
    });
    Ok(backups)
}

/// Restore to the source store. The caller subsequently syncs enabled records.
/// The selected backup is retained and the previous source is backed up as well.
pub fn restore_backup(id: &str) -> Result<String, AppError> {
    restore_at(id, &get_ssot_dir(), &get_claude_skills_dir())
}

fn restore_at(id: &str, ssot: &Path, live: &Path) -> Result<String, AppError> {
    let (origin, name) = id
        .split_once(':')
        .ok_or_else(|| AppError::InvalidInput("Invalid backup ID".into()))?;
    let directory = backup_directory(name)?;
    let parent = match origin {
        "source" => ssot,
        "live" => live,
        _ => return Err(AppError::InvalidInput("Invalid backup origin".into())),
    };
    reject_symlink(ssot)?;
    reject_symlink(parent)?;
    let root = parent.join(".cc-switch-backups");
    reject_symlink(&root)?;
    let source = root.join(name);
    reject_symlink(&source)?;
    reject_symlink(&source.join("SKILL.md"))?;
    if !source.join("SKILL.md").is_file() {
        return Err(AppError::InvalidInput("Backup has no SKILL.md".into()));
    }
    let dest = ssot.join(&directory);
    let expected = directory_version(&dest)?;
    let original = directory_version(&source)?;
    if original == expected {
        return Ok(directory);
    }
    fs::create_dir_all(ssot).map_err(|e| AppError::io(ssot, e))?;
    let stage = tempfile::tempdir_in(ssot).map_err(|e| AppError::io(ssot, e))?;
    let staged = stage.path().join("skill");
    copy_dir_recursive(&source, &staged)?;
    if directory_version(&source)? != original || directory_version(&staged)? != original {
        return Err(AppError::Conflict {
            path: source.display().to_string(),
        });
    }
    publish_directory(&staged, &dest, ssot, expected)?;
    Ok(directory)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn restores_deleted_and_replaced_sources_without_consuming_history() {
        let temp = tempfile::tempdir().unwrap();
        let ssot = temp.path().join("source");
        let live = temp.path().join("live");
        let root = live.join(".cc-switch-backups");
        let name = format!("my-skill-{}", uuid::Uuid::new_v4());
        fs::create_dir_all(root.join(&name)).unwrap();
        fs::write(root.join(&name).join("SKILL.md"), "old content").unwrap();
        let id = format!("live:{name}");
        assert_eq!(list_at(&ssot, &live).unwrap().len(), 1);
        assert_eq!(restore_at(&id, &ssot, &live).unwrap(), "my-skill");
        assert_eq!(
            fs::read_to_string(ssot.join("my-skill/SKILL.md")).unwrap(),
            "old content"
        );
        fs::write(ssot.join("my-skill/SKILL.md"), "new content").unwrap();
        restore_at(&id, &ssot, &live).unwrap();
        let backups = list_at(&ssot, &live).unwrap();
        assert_eq!(backups.len(), 2);
        let undo = backups.iter().find(|b| b.origin == "source").unwrap();
        restore_at(&undo.id, &ssot, &live).unwrap();
        assert_eq!(
            fs::read_to_string(ssot.join("my-skill/SKILL.md")).unwrap(),
            "new content"
        );
        assert!(root.join(name).join("SKILL.md").is_file());
    }

    #[test]
    fn rejects_traversal_unknown_origins_and_invalid_ids_before_writes() {
        let temp = tempfile::tempdir().unwrap();
        let ssot = temp.path().join("source");
        for id in [
            "../escape",
            "live:../escape",
            "unknown:foo",
            "live:😀",
            "source:.hidden-00000000-0000-0000-0000-000000000000",
        ] {
            assert!(restore_at(id, &ssot, temp.path()).is_err());
            assert!(!ssot.exists());
        }
    }

    #[cfg(unix)]
    #[test]
    fn refuses_symlink_backup_contents_without_touching_source() {
        let temp = tempfile::tempdir().unwrap();
        let ssot = temp.path().join("source");
        let live = temp.path().join("live");
        let name = format!("test-{}", uuid::Uuid::new_v4());
        let backup = live.join(".cc-switch-backups").join(&name);
        fs::create_dir_all(&backup).unwrap();
        fs::write(backup.join("SKILL.md"), "instructions").unwrap();
        std::os::unix::fs::symlink(temp.path(), backup.join("escape")).unwrap();
        assert!(restore_at(&format!("live:{name}"), &ssot, &live).is_err());
        assert!(!ssot.exists());
    }
}
