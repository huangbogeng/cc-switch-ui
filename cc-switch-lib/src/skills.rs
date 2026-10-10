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
use crate::operation_report::OperationReport;
use std::fs;
use std::path::{Path, PathBuf};

mod recovery;
pub use recovery::{list_backups, restore_backup, SkillBackup};

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
    sync_skills_report(skills, ssot_dir, target_dir)?.ensure_success()
}

pub fn sync_report(db: &Database, app_type: &str) -> Result<OperationReport, AppError> {
    sync_skills_report(
        &db.get_all_skills(app_type)?,
        &get_ssot_dir(),
        &get_claude_skills_dir(),
    )
}

fn sync_skills_report(
    skills: &[SkillRecord],
    ssot_dir: &Path,
    target_dir: &Path,
) -> Result<OperationReport, AppError> {
    // Ambiguous target ownership is rejected before any writes.
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
    let mut report = OperationReport::new();
    for skill in skills {
        match sync_one(skill, ssot_dir, target_dir) {
            Ok(true) => report.record(&skill.directory, "applied", None),
            Ok(false) => report.record(&skill.directory, "skipped", None),
            Err(error) => report.record(&skill.directory, "failed", Some(error.to_string())),
        }
    }
    Ok(report)
}

fn sync_one(skill: &SkillRecord, ssot_dir: &Path, target_dir: &Path) -> Result<bool, AppError> {
    let source = ssot_dir.join(&skill.directory);
    let dest = target_dir.join(&skill.directory);
    reject_symlink(&dest)?;
    if !skill.enabled {
        return Ok(backup_target(&dest, target_dir)?.is_some());
    }
    reject_symlink(&source)?;
    if !source.join("SKILL.md").is_file() {
        return Err(AppError::Config(format!(
            "Skill '{}' has no SKILL.md; target preserved",
            skill.directory
        )));
    }
    let source_version = directory_version(&source)?;
    let target_version = directory_version(&dest)?;
    if source_version == target_version {
        return Ok(false);
    }
    // Finish copying before touching the previous installation.
    let stage = tempfile::tempdir_in(target_dir).map_err(|e| AppError::io(target_dir, e))?;
    let staged = stage.path().join("skill");
    copy_dir_recursive(&source, &staged)?;
    if directory_version(&source)? != source_version
        || directory_version(&staged)? != source_version
    {
        return Err(AppError::Conflict {
            path: source.display().to_string(),
        });
    }
    publish_directory(&staged, &dest, target_dir, target_version)?;
    Ok(true)
}

/// Content digest without following links. Absence and an empty directory differ.
fn directory_version(path: &Path) -> Result<Option<Vec<u8>>, AppError> {
    use sha2::{Digest, Sha256};
    fn hash_tree(path: &Path, hasher: &mut Sha256) -> Result<(), AppError> {
        reject_symlink(path)?;
        let metadata = fs::metadata(path).map_err(|e| AppError::io(path, e))?;
        if metadata.is_file() {
            hasher.update(b"file");
            let bytes = fs::read(path).map_err(|e| AppError::io(path, e))?;
            hasher.update((bytes.len() as u64).to_le_bytes());
            hasher.update(bytes);
        } else if metadata.is_dir() {
            hasher.update(b"dir");
            let mut entries = fs::read_dir(path)
                .map_err(|e| AppError::io(path, e))?
                .collect::<Result<Vec<_>, _>>()
                .map_err(|e| AppError::io(path, e))?;
            entries.sort_by_key(|entry| entry.file_name());
            hasher.update((entries.len() as u64).to_le_bytes());
            for entry in entries {
                let name = entry.file_name();
                let bytes = name.as_encoded_bytes();
                hasher.update((bytes.len() as u64).to_le_bytes());
                hasher.update(bytes);
                hash_tree(&entry.path(), hasher)?;
            }
        } else {
            return Err(AppError::InvalidInput(format!(
                "Unsupported skill file: {}",
                path.display()
            )));
        }
        Ok(())
    }
    reject_symlink(path)?;
    match fs::metadata(path) {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(AppError::io(path, e)),
        Ok(_) => {
            let mut hash = Sha256::new();
            hash_tree(path, &mut hash)?;
            Ok(Some(hash.finalize().to_vec()))
        }
    }
}

fn publish_directory(
    staged: &Path,
    dest: &Path,
    parent: &Path,
    expected: Option<Vec<u8>>,
) -> Result<(), AppError> {
    if directory_version(dest)? != expected {
        return Err(AppError::Conflict {
            path: dest.display().to_string(),
        });
    }
    let backup = backup_target(dest, parent)?;
    if let Err(error) = fs::rename(staged, dest) {
        if let Some(backup) = backup {
            fs::rename(&backup, dest).map_err(|restore| {
                AppError::Config(format!(
                    "Publish failed: {error}; restore failed: {restore}; backup: {}",
                    backup.display()
                ))
            })?;
        }
        return Err(AppError::io(dest, error));
    }
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
    fs::read(&skill_md).map_err(|e| AppError::io(&skill_md, e))?;
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
    let original = directory_version(source_dir)?;
    let expected = directory_version(&ssot_dest)?;
    if expected.is_some() && expected != original {
        return Err(AppError::Config(format!("Source store for '{dir_name}' differs from import source; review existing files before retrying")));
    }
    if expected.is_none() {
        fs::create_dir_all(&ssot_dir).map_err(|e| AppError::io(&ssot_dir, e))?;
        let stage = tempfile::tempdir_in(&ssot_dir).map_err(|e| AppError::io(&ssot_dir, e))?;
        let staged = stage.path().join("skill");
        copy_dir_recursive(source_dir, &staged)?;
        if directory_version(source_dir)? != original || directory_version(&staged)? != original {
            return Err(AppError::Conflict {
                path: source_dir.display().to_string(),
            });
        }
        publish_directory(&staged, &ssot_dest, &ssot_dir, expected)?;
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
/// Every candidate has an outcome. A failed scan is included without hiding completed imports.
pub fn import_from_claude(db: &Database, app_type: &str) -> Result<OperationReport, AppError> {
    let existing = db.get_all_skills(app_type)?;
    reject_symlink(&get_ssot_dir())?;
    let mut report = OperationReport::new();
    let live = get_claude_skills_dir();
    let mut candidates = Vec::new();
    let direct = (|| -> Result<(), AppError> {
        reject_symlink(&live)?;
        if !live.exists() {
            return Ok(());
        }
        for entry in fs::read_dir(&live).map_err(|e| AppError::io(&live, e))? {
            let entry = entry.map_err(|e| AppError::io(&live, e))?;
            let name = entry.file_name().to_string_lossy().to_string();
            if !name.starts_with('.')
                && entry.path().is_dir()
                && entry.path().join("SKILL.md").exists()
            {
                candidates.push((entry.path(), name));
            }
        }
        Ok(())
    })();
    if let Err(error) = direct {
        report.record("Claude skills scan", "failed", Some(error.to_string()));
    }
    candidates.sort_by(|a, b| a.0.cmp(&b.0));
    // Preserve candidates discovered before a later plugin traversal failure.
    let plugins = config::get_claude_config_dir().join("plugins");
    let mut plugin_candidates = Vec::new();
    if plugins.exists() {
        if let Err(error) = scan_for_skills(&plugins, &mut plugin_candidates, 0) {
            report.record("Claude plugins scan", "failed", Some(error.to_string()));
        }
    }
    plugin_candidates.sort_by(|a, b| a.0.cmp(&b.0));
    candidates.extend(plugin_candidates);
    import_candidates(db, app_type, &existing, candidates, &mut report);
    Ok(report)
}
fn import_candidates(
    db: &Database,
    app_type: &str,
    existing: &[SkillRecord],
    candidates: Vec<(PathBuf, String)>,
    report: &mut OperationReport,
) {
    let mut handled = std::collections::HashSet::new();
    for (path, name) in candidates {
        if handled.contains(&name) {
            report.record(name, "skipped", None);
            continue;
        }
        let result = (|| -> Result<&'static str, AppError> {
            validate_directory(&name)?;
            reject_symlink(&path)?;
            reject_symlink(&path.join("SKILL.md"))?;
            if let Some(record) = existing.iter().find(|s| s.directory == name) {
                if record.collection.is_some() {
                    return Ok("skipped");
                }
                let mut record = record.clone();
                record.collection = Some(derive_collection(&path));
                db.save_skill(&record)?;
                return Ok("updated");
            }
            import_skill_from_dir(db, app_type, &path, &name)?;
            Ok("imported")
        })();
        match result {
            Ok(outcome) => {
                handled.insert(name.clone());
                report.record(name, outcome, None);
            }
            Err(error) => report.record(name, "failed", Some(error.to_string())),
        }
    }
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
    fn sync_reports_partial_progress_and_retry_skips_completed_files() {
        let temp = tempfile::tempdir().unwrap();
        let source = temp.path().join("ssot");
        let target = temp.path().join("live");
        install(&source, "good", "new");
        install(&target, "missing", "keep");
        let records = [skill("missing", true), skill("good", true)];
        let report = sync_skills_report(&records, &source, &target).unwrap();
        assert_eq!((report.applied, report.failed), (1, 1));
        assert!(!report.success);
        assert_eq!(
            fs::read_to_string(target.join("missing/SKILL.md")).unwrap(),
            "keep"
        );
        assert_eq!(
            fs::read_to_string(target.join("good/SKILL.md")).unwrap(),
            "new"
        );
        install(&source, "missing", "fixed");
        let report = sync_skills_report(&records, &source, &target).unwrap();
        assert!(report.success);
        assert_eq!((report.applied, report.skipped), (1, 1));
        assert_eq!(
            fs::read_dir(target.join(".cc-switch-backups"))
                .unwrap()
                .count(),
            1
        );
    }

    #[test]
    fn collection_backfill_counts_only_committed_records_and_handles_duplicates_once() {
        let temp = tempfile::tempdir().unwrap();
        let source = temp.path().join("test-plugin/skills");
        install(&source, "blocked", "test");
        install(&source, "good", "test");
        let db = Database::test_memory();
        let blocked = skill("blocked", false);
        let good = skill("good", true);
        db.save_skill(&blocked).unwrap();
        db.save_skill(&good).unwrap();
        db.conn().execute_batch("CREATE TRIGGER reject_backfill BEFORE INSERT ON skills WHEN NEW.directory = 'blocked' BEGIN SELECT RAISE(ABORT, 'Test backfill failure'); END;").unwrap();
        let candidates = vec![
            (source.join("blocked"), "blocked".into()),
            (source.join("good"), "good".into()),
            (source.join("good"), "good".into()),
        ];
        let mut report = OperationReport::new();
        import_candidates(
            &db,
            "claude_code",
            &[blocked, good],
            candidates,
            &mut report,
        );
        assert_eq!(
            (
                report.imported,
                report.updated,
                report.skipped,
                report.failed
            ),
            (0, 1, 1, 1)
        );
        let records = db.get_all_skills("claude_code").unwrap();
        assert!(records
            .iter()
            .find(|s| s.directory == "blocked")
            .unwrap()
            .collection
            .is_none());
        assert!(
            !records
                .iter()
                .find(|s| s.directory == "blocked")
                .unwrap()
                .enabled
        );
        assert_eq!(
            records
                .iter()
                .find(|s| s.directory == "good")
                .unwrap()
                .collection
                .as_deref(),
            Some("Test Plugin")
        );
    }

    #[test]
    fn unchanged_sync_is_a_noop_without_backup_growth() {
        let temp = tempfile::tempdir().unwrap();
        let source = temp.path().join("ssot");
        let target = temp.path().join("target");
        install(&source, "enabled", "identical");
        install(&target, "enabled", "identical");
        sync_skills(&[skill("enabled", true)], &source, &target).unwrap();
        sync_skills(&[skill("enabled", true)], &source, &target).unwrap();
        assert!(!target.join(".cc-switch-backups").exists());
        assert_eq!(fs::read_dir(&target).unwrap().count(), 1);
    }

    #[test]
    fn changed_destination_is_preserved_when_publication_conflicts() {
        let temp = tempfile::tempdir().unwrap();
        let target = temp.path().join("target");
        let staging = temp.path().join("staging");
        install(&target, "enabled", "old content");
        install(&staging, "enabled", "intended content");
        let dest = target.join("enabled");
        let original = directory_version(&dest).unwrap();
        fs::write(dest.join("SKILL.md"), "external edit").unwrap();
        let result = publish_directory(&staging.join("enabled"), &dest, &target, original);
        assert!(matches!(result, Err(AppError::Conflict { .. })));
        assert_eq!(
            fs::read_to_string(dest.join("SKILL.md")).unwrap(),
            "external edit"
        );
        assert!(!target.join(".cc-switch-backups").exists());
        assert!(staging.join("enabled/SKILL.md").is_file());
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
