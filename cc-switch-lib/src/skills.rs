//! Skills sync module
//!
//! Handles syncing installed skills from the CC Switch database
//! to Claude Code's skills directory (~/.claude/skills/).
//!
//! SSOT (Single Source of Truth): ~/.cc-switch/skills/
//! Target for Claude Code: ~/.claude/skills/
//!
//! Sync method: copy (recursive directory copy). Symlink support may be
//! added later following the original cc-switch pattern.

use crate::config;
use crate::database::{Database, SkillRecord};
use crate::error::AppError;
use std::fs;
use std::path::{Path, PathBuf};

/// Get the SSOT skills directory (~/.cc-switch/skills/).
fn get_ssot_dir() -> PathBuf {
    config::get_app_config_dir().join("skills")
}

/// Get the target skills directory for Claude Code (~/.claude/skills/).
fn get_claude_skills_dir() -> PathBuf {
    config::get_claude_config_dir().join("skills")
}

/// Sync managed skills only. External directories are preserved; replaced or
/// disabled installations are moved to `.cc-switch-backups` for recovery.
pub fn sync_enabled_to_claude(db: &Database, app_type: &str) -> Result<(), AppError> {
    let skills = db.get_all_skills(app_type)?;
    sync_skills(&skills, &get_ssot_dir(), &get_claude_skills_dir())
}

fn sync_skills(skills: &[SkillRecord], ssot_dir: &Path, target_dir: &Path) -> Result<(), AppError> {
    let mut directories = std::collections::HashSet::new();
    for skill in skills {
        validate_directory(&skill.directory)?;
        if !directories.insert(&skill.directory) {
            return Err(AppError::Config(format!(
                "Duplicate managed skill directory: {}",
                skill.directory
            )));
        }
    }
    reject_symlink(ssot_dir)?;
    reject_symlink(target_dir)?;
    fs::create_dir_all(target_dir).map_err(|e| AppError::io(target_dir, e))?;
    for skill in skills {
        let source = ssot_dir.join(&skill.directory);
        let dest = target_dir.join(&skill.directory);
        reject_symlink(&dest)?;
        if !skill.enabled {
            backup_target(&dest, target_dir)?;
            continue;
        }
        reject_symlink(&source)?;
        if !source.join("SKILL.md").is_file() {
            return Err(AppError::Config(format!(
                "Skill '{}' has no SKILL.md; target preserved",
                skill.directory
            )));
        }
        // Finish copying before touching the previous installation.
        let stage = tempfile::tempdir_in(target_dir).map_err(|e| AppError::io(target_dir, e))?;
        let staged = stage.path().join("skill");
        copy_dir_recursive(&source, &staged)?;
        let backup = backup_target(&dest, target_dir)?;
        if let Err(error) = fs::rename(&staged, &dest) {
            if let Some(backup) = backup {
                fs::rename(&backup, &dest).map_err(|restore| {
                    AppError::Config(format!(
                        "Publish failed: {error}; restore failed: {restore}; backup: {}",
                        backup.display()
                    ))
                })?;
            }
            return Err(AppError::io(&dest, error));
        }
    }

    log::info!(
        "Synced {} enabled skills to {}",
        skills.iter().filter(|skill| skill.enabled).count(),
        target_dir.display()
    );
    Ok(())
}

/// Skill directories are a single portable path component, never a filesystem path.
pub fn validate_directory(directory: &str) -> Result<(), AppError> {
    if directory.is_empty()
        || directory.trim() != directory
        || directory.starts_with('.')
        || directory.ends_with('.')
        || directory
            .chars()
            .any(|c| c == '/' || c == '\\' || c == ':' || c.is_control())
    {
        return Err(AppError::InvalidInput(
            "Skill directory must be a single non-hidden directory name".into(),
        ));
    }
    Ok(())
}

fn reject_symlink(path: &Path) -> Result<(), AppError> {
    match fs::symlink_metadata(path) {
        Ok(meta) if meta.file_type().is_symlink() => Err(AppError::InvalidInput(format!(
            "Refusing symlink: {}",
            path.display()
        ))),
        Ok(_) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(AppError::io(path, e)),
    }
}

fn backup_target(dest: &Path, target_dir: &Path) -> Result<Option<PathBuf>, AppError> {
    if !dest.exists() {
        return Ok(None);
    }
    let backup_root = target_dir.join(".cc-switch-backups");
    reject_symlink(&backup_root)?;
    fs::create_dir_all(&backup_root).map_err(|e| AppError::io(&backup_root, e))?;
    let backup = backup_root.join(format!(
        "{}-{}",
        dest.file_name().unwrap().to_string_lossy(),
        uuid::Uuid::new_v4()
    ));
    fs::rename(dest, &backup).map_err(|e| AppError::io(dest, e))?;
    log::info!(
        "Previous skill installation backed up to {}",
        backup.display()
    );
    Ok(Some(backup))
}

pub fn remove_from_claude(directory: &str) -> Result<(), AppError> {
    validate_directory(directory)?;
    let target = get_claude_skills_dir();
    reject_symlink(&target)?;
    let dest = target.join(directory);
    reject_symlink(&dest)?;
    backup_target(&dest, &target)?;
    Ok(())
}

/// Recursively copy a directory.
fn copy_dir_recursive(src: &Path, dest: &Path) -> Result<(), AppError> {
    fs::create_dir_all(dest).map_err(|e| AppError::io(dest, e))?;

    for entry in fs::read_dir(src).map_err(|e| AppError::io(src, e))? {
        let entry = entry.map_err(|e| AppError::io(src, e))?;
        let path = entry.path();
        reject_symlink(&path)?;
        let dest_path = dest.join(entry.file_name());

        if path.is_dir() {
            copy_dir_recursive(&path, &dest_path)?;
        } else {
            fs::copy(&path, &dest_path).map_err(|e| AppError::IoContext {
                context: format!("copy {:?} -> {:?}", path.display(), dest_path.display()),
                source: e,
            })?;
        }
    }
    Ok(())
}

/// Parse SKILL.md frontmatter for name and description.
fn parse_skill_md(path: &Path) -> (String, Option<String>) {
    let content = match fs::read_to_string(path) {
        Ok(c) => c,
        Err(_) => return (String::new(), None),
    };
    let content = content.trim_start_matches('\u{feff}');

    let parts: Vec<&str> = content.splitn(3, "---").collect();
    if parts.len() < 3 {
        return (String::new(), None);
    }

    let front_matter = parts[1].trim();
    #[derive(serde::Deserialize)]
    struct SkillMeta {
        name: Option<String>,
        description: Option<String>,
    }
    match serde_yaml::from_str::<SkillMeta>(front_matter) {
        Ok(meta) => (meta.name.unwrap_or_default(), meta.description),
        Err(_) => (String::new(), None),
    }
}

/// Write a SKILL.md file to the SSOT for a given skill record.
/// Creates the directory and file if they don't exist.
pub fn ensure_skill_ssot(skill: &SkillRecord) -> Result<(), AppError> {
    validate_directory(&skill.directory)?;
    let ssot_dir = get_ssot_dir();
    reject_symlink(&ssot_dir)?;
    let skill_dir = ssot_dir.join(&skill.directory);

    reject_symlink(&skill_dir)?;
    reject_symlink(&skill_dir.join("SKILL.md"))?;
    if skill_dir.join("SKILL.md").exists() {
        return Ok(());
    }

    fs::create_dir_all(&skill_dir).map_err(|e| AppError::io(&skill_dir, e))?;

    let mut metadata = serde_json::json!({"name": skill.name});
    if let Some(description) = &skill.description {
        metadata["description"] = serde_json::json!(description);
    }
    let yaml = serde_yaml::to_string(&metadata)
        .map_err(|e| AppError::Config(format!("Invalid skill metadata: {e}")))?;
    let content = format!("---\n{yaml}---\n");

    config::atomic_write(&skill_dir.join("SKILL.md"), content.as_bytes())?;

    log::info!(
        "Created SSOT for skill '{}' at {}",
        skill.directory,
        skill_dir.display()
    );
    Ok(())
}

/// Recursively find SKILL.md files under `~/.claude/plugins/` (excluding cache/)
/// that match the pattern `<any>/skills/<skill-name>/SKILL.md`.
fn find_plugin_skills() -> Result<Vec<(PathBuf, String)>, AppError> {
    let plugins_dir = config::get_claude_config_dir().join("plugins");
    if !plugins_dir.exists() {
        return Ok(Vec::new());
    }

    let mut result = Vec::new();
    scan_for_skills(&plugins_dir, &mut result, 0)?;
    Ok(result)
}

fn scan_for_skills(
    current: &Path,
    result: &mut Vec<(PathBuf, String)>,
    depth: usize,
) -> Result<(), AppError> {
    if fs::symlink_metadata(current)
        .map_err(|e| AppError::io(current, e))?
        .file_type()
        .is_symlink()
    {
        return Ok(());
    }
    if depth > 8 {
        return Ok(());
    }

    // Skip cache directory — these are cached copies of marketplace plugins
    if current.file_name().is_some_and(|n| n == "cache") {
        return Ok(());
    }

    // Check if this is a skill directory: parent directory is named "skills"
    if current.join("SKILL.md").exists() {
        let parent_name = current.parent().and_then(|p| p.file_name());
        if parent_name.is_some_and(|n| n == "skills") {
            if let Some(dir_name) = current.file_name().map(|n| n.to_string_lossy().to_string()) {
                if !dir_name.starts_with('.') {
                    result.push((current.to_path_buf(), dir_name));
                    return Ok(()); // Don't recurse into found skill
                }
            }
        }
    }

    // Recurse into subdirectories
    for entry in fs::read_dir(current).map_err(|e| AppError::io(current, e))? {
        let entry = entry.map_err(|e| AppError::io(current, e))?;
        let path = entry.path();
        if path.is_dir() {
            scan_for_skills(&path, result, depth + 1)?;
        }
    }

    Ok(())
}

/// Derive a human-readable collection name from a skill's source directory path.
///
/// Convention: skill dirs live under `<plugin>/skills/<name>/` or
/// `<plugin>/<version>/skills/<name>/` (cached plugins).  We extract the
/// plugin segment and convert it to display form (e.g. "superpowers" →
/// "Superpowers", "rust-skills" → "Rust Skills").
///
/// Skills that don't live under a named plugin (e.g. standalone skills in
/// `~/.claude/skills/`) get the collection "Other".
fn derive_collection(source_dir: &Path) -> String {
    // Walk up looking for a parent named "skills"
    for ancestor in source_dir.ancestors() {
        if ancestor.file_name().is_some_and(|n| n == "skills") {
            // The parent of "skills" is the plugin dir (or a version dir for cached plugins)
            if let Some(parent) = ancestor.parent() {
                let plugin_name = parent.file_name().map(|n| n.to_string_lossy().to_string());
                if let Some(ref name) = plugin_name {
                    // Skip version-like directory (e.g. "5.1.0"), go one more level up
                    if name.chars().all(|c| c.is_ascii_digit() || c == '.') {
                        if let Some(grandparent) = parent.parent() {
                            if let Some(gp_name) = grandparent.file_name() {
                                return fmt_collection_name(&gp_name.to_string_lossy());
                            }
                        }
                    } else {
                        return fmt_collection_name(name);
                    }
                }
            }
            break;
        }
    }
    "Other".to_string()
}

fn fmt_collection_name(raw: &str) -> String {
    raw.split('-')
        .map(|word| {
            let mut c = word.chars();
            match c.next() {
                None => String::new(),
                Some(f) => f.to_uppercase().to_string() + c.as_str(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// Import a skill from a source directory, copying it to SSOT and creating a DB record.
fn import_skill_from_dir(
    db: &Database,
    app_type: &str,
    source_dir: &Path,
    dir_name: &str,
) -> Result<usize, AppError> {
    validate_directory(dir_name)?;
    reject_symlink(source_dir)?;
    let skill_md = source_dir.join("SKILL.md");
    reject_symlink(&skill_md)?;
    let (name, description) = parse_skill_md(&skill_md);
    let display_name = if name.is_empty() {
        dir_name.to_string()
    } else {
        name
    };

    // Copy whole skill directory to SSOT
    let ssot_dir = get_ssot_dir();
    let ssot_dest = ssot_dir.join(dir_name);
    reject_symlink(&ssot_dir)?;
    reject_symlink(&ssot_dest)?;
    if !ssot_dest.exists() {
        fs::create_dir_all(&ssot_dir).map_err(|e| AppError::io(&ssot_dir, e))?;
        let stage = tempfile::tempdir_in(&ssot_dir).map_err(|e| AppError::io(&ssot_dir, e))?;
        let staged = stage.path().join("skill");
        copy_dir_recursive(source_dir, &staged)?;
        fs::rename(&staged, &ssot_dest).map_err(|e| AppError::io(&ssot_dest, e))?;
    }

    let collection = derive_collection(source_dir);

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64;

    let skill = SkillRecord {
        id: format!("local:{dir_name}"),
        name: display_name,
        description,
        directory: dir_name.to_string(),
        app_type: app_type.to_string(),
        enabled: true,
        collection: Some(collection),
        installed_at: now,
        repo_owner: None,
        repo_name: None,
        repo_branch: None,
        readme_url: None,
    };
    db.save_skill(&skill)?;

    log::info!(
        "Imported skill '{}' from {}",
        dir_name,
        source_dir.display()
    );
    Ok(1)
}

/// Import skills from Claude Code's skill sources into the database.
///
/// Scans two sources:
/// 1. `~/.claude/skills/<dir>/SKILL.md` — direct skill directory
/// 2. `~/.claude/plugins/<...>/skills/<dir>/SKILL.md` — plugin-bundled skills
///
/// Each found skill is copied to the SSOT (`~/.cc-switch/skills/<dir>/`)
/// and a database record is created. Skills already in the DB are skipped.
/// Existing skills with a missing collection field get backfilled.
/// Returns the count of newly imported + backfilled skills.
pub fn import_from_claude(db: &Database, app_type: &str) -> Result<usize, AppError> {
    reject_symlink(&get_ssot_dir())?;
    reject_symlink(&get_claude_skills_dir())?;
    let all_existing = db.get_all_skills(app_type)?;
    let mut existing_dirs: std::collections::HashSet<String> =
        all_existing.iter().map(|s| s.directory.clone()).collect();

    // Track skills that already exist but need collection backfill
    let needs_backfill: std::collections::HashSet<String> = all_existing
        .iter()
        .filter(|s| s.collection.is_none())
        .map(|s| s.directory.clone())
        .collect();

    let mut imported = 0;

    // Phase 1: Scan ~/.claude/skills/
    let claude_skills_dir = get_claude_skills_dir();
    if claude_skills_dir.exists() {
        for entry in
            fs::read_dir(&claude_skills_dir).map_err(|e| AppError::io(&claude_skills_dir, e))?
        {
            let entry = entry.map_err(|e| AppError::io(&claude_skills_dir, e))?;
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }
            let dir_name = entry.file_name().to_string_lossy().to_string();
            if dir_name.starts_with('.') || existing_dirs.contains(&dir_name) {
                // Still backfill collection if missing
                if needs_backfill.contains(&dir_name) {
                    let coll = derive_collection(&path);
                    if !coll.is_empty() {
                        if let Some(mut skill) = db
                            .get_all_skills(app_type)
                            .ok()
                            .and_then(|skills| skills.into_iter().find(|s| s.directory == dir_name))
                        {
                            skill.collection = Some(coll);
                            let _ = db.save_skill(&skill);
                            imported += 1;
                        }
                    }
                }
                continue;
            }
            if !path.join("SKILL.md").exists() {
                continue;
            }

            imported += import_skill_from_dir(db, app_type, &path, &dir_name)?;
            existing_dirs.insert(dir_name);
        }
    }

    // Phase 2: Scan ~/.claude/plugins/ for plugin-bundled skills
    let plugin_skills = find_plugin_skills()?;
    for (skill_dir, dir_name) in &plugin_skills {
        if existing_dirs.contains(dir_name) {
            // Still backfill collection if missing
            if needs_backfill.contains(dir_name) {
                let coll = derive_collection(skill_dir);
                if !coll.is_empty() {
                    if let Some(mut skill) = db
                        .get_all_skills(app_type)
                        .ok()
                        .and_then(|skills| skills.into_iter().find(|s| s.directory == *dir_name))
                    {
                        skill.collection = Some(coll);
                        let _ = db.save_skill(&skill);
                        imported += 1;
                    }
                }
            }
            continue;
        }
        imported += import_skill_from_dir(db, app_type, skill_dir, dir_name)?;
        existing_dirs.insert(dir_name.clone());
    }

    log::info!(
        "Skills import complete: {} new or backfilled skills for app_type={}",
        imported,
        app_type
    );
    Ok(imported)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn skill(directory: &str, enabled: bool) -> SkillRecord {
        serde_json::from_value(serde_json::json!({"id": directory, "name": directory,
            "directory": directory, "appType": "claude_code", "enabled": enabled}))
        .unwrap()
    }

    fn install(root: &Path, name: &str, content: &str) {
        fs::create_dir_all(root.join(name)).unwrap();
        fs::write(root.join(name).join("SKILL.md"), content).unwrap();
    }

    #[test]
    fn sync_preserves_external_skills_and_backs_up_managed_installations() {
        let temp = tempfile::tempdir().unwrap();
        let source = temp.path().join("ssot");
        let target = temp.path().join("target");
        install(&source, "enabled", "new content");
        install(&target, "enabled", "old content");
        install(&target, "disabled", "disabled content");
        install(&target, "external", "user content");
        sync_skills(
            &[skill("enabled", true), skill("disabled", false)],
            &source,
            &target,
        )
        .unwrap();
        assert_eq!(
            fs::read_to_string(target.join("enabled/SKILL.md")).unwrap(),
            "new content"
        );
        assert_eq!(
            fs::read_to_string(target.join("external/SKILL.md")).unwrap(),
            "user content"
        );
        assert!(!target.join("disabled").exists());
        let mut backups: Vec<String> = fs::read_dir(target.join(".cc-switch-backups"))
            .unwrap()
            .map(|entry| fs::read_to_string(entry.unwrap().path().join("SKILL.md")).unwrap())
            .collect();
        backups.sort();
        assert_eq!(backups, vec!["disabled content", "old content"]);
        sync_skills(&[], &source, &target).unwrap();
        assert!(target.join("external/SKILL.md").exists());
    }

    #[test]
    fn missing_source_preserves_previous_installation() {
        let temp = tempfile::tempdir().unwrap();
        let source = temp.path().join("ssot");
        let target = temp.path().join("target");
        install(&target, "enabled", "old content");
        assert!(sync_skills(&[skill("enabled", true)], &source, &target).is_err());
        assert_eq!(
            fs::read_to_string(target.join("enabled/SKILL.md")).unwrap(),
            "old content"
        );
        assert!(!target.join(".cc-switch-backups").exists());
    }

    #[test]
    fn invalid_directories_are_rejected_before_filesystem_mutation() {
        let temp = tempfile::tempdir().unwrap();
        let target = temp.path().join("target");
        for directory in [
            "",
            "..",
            "../escape",
            "a/b",
            "a\\b",
            "C:escape",
            ".hidden",
            "trailing.",
            " leading",
        ] {
            assert!(
                sync_skills(&[skill(directory, true)], temp.path(), &target).is_err(),
                "{directory}"
            );
            assert!(!target.exists());
        }
    }

    #[test]
    fn legacy_duplicate_directories_fail_before_changes() {
        let temp = tempfile::tempdir().unwrap();
        let target = temp.path().join("target");
        assert!(sync_skills(
            &[skill("same", true), skill("same", false)],
            temp.path(),
            &target
        )
        .is_err());
        assert!(!target.exists());
    }

    #[cfg(unix)]
    #[test]
    fn failed_staging_copy_preserves_old_target_and_external_symlink_contents() {
        let temp = tempfile::tempdir().unwrap();
        let source = temp.path().join("ssot");
        let target = temp.path().join("target");
        let external = temp.path().join("private");
        fs::write(&external, "private").unwrap();
        install(&source, "enabled", "new content");
        install(&target, "enabled", "old content");
        std::os::unix::fs::symlink(&external, source.join("enabled/linked")).unwrap();
        assert!(sync_skills(&[skill("enabled", true)], &source, &target).is_err());
        assert_eq!(
            fs::read_to_string(target.join("enabled/SKILL.md")).unwrap(),
            "old content"
        );
        assert_eq!(fs::read_to_string(external).unwrap(), "private");
        assert!(!target.join(".cc-switch-backups").exists());
        assert_eq!(fs::read_dir(&target).unwrap().count(), 1);
    }

    #[cfg(unix)]
    #[test]
    fn disable_refuses_symlink_target() {
        let temp = tempfile::tempdir().unwrap();
        let target = temp.path().join("target");
        let external = temp.path().join("external");
        install(&external, "enabled", "user content");
        fs::create_dir(&target).unwrap();
        std::os::unix::fs::symlink(external.join("enabled"), target.join("enabled")).unwrap();
        assert!(sync_skills(&[skill("enabled", false)], temp.path(), &target).is_err());
        assert_eq!(
            fs::read_to_string(external.join("enabled/SKILL.md")).unwrap(),
            "user content"
        );
    }

    /// `copy_dir_recursive` is tested indirectly through the live config tests.
    /// This test validates basic recursive copy behavior.
    #[test]
    fn copy_dir_recursive_copies_files_and_subdirs() {
        let temp = tempfile::tempdir().expect("tempdir");
        let src = temp.path().join("src");
        let dest = temp.path().join("dest");

        fs::create_dir_all(src.join("sub")).unwrap();
        fs::write(src.join("top.txt"), "top").unwrap();
        fs::write(src.join("sub").join("nested.txt"), "nested").unwrap();

        copy_dir_recursive(&src, &dest).expect("copy");

        assert!(dest.join("top.txt").exists());
        assert!(dest.join("sub").join("nested.txt").exists());
        assert_eq!(fs::read_to_string(dest.join("top.txt")).unwrap(), "top");
        assert_eq!(
            fs::read_to_string(dest.join("sub").join("nested.txt")).unwrap(),
            "nested"
        );
    }
}
