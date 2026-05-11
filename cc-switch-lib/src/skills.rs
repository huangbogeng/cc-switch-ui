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

/// Sync all enabled skills from the database to ~/.claude/skills/.
///
/// Two-phase:
/// 1. Cleanup — remove directories in the target that are not in the
///    enabled set (orphaned skills from previous syncs).
/// 2. Write — copy each enabled skill from SSOT to the target dir.
pub fn sync_enabled_to_claude(db: &Database, app_type: &str) -> Result<(), AppError> {
    let skills = db.get_enabled_skills(app_type)?;
    let ssot_dir = get_ssot_dir();
    let target_dir = get_claude_skills_dir();

    // Index enabled skills by directory name
    let enabled_dirs: std::collections::HashSet<String> = skills
        .iter()
        .map(|s| s.directory.clone())
        .collect();

    // Phase 1: Cleanup orphaned skills in target dir
    if target_dir.exists() {
        for entry in fs::read_dir(&target_dir).map_err(|e| AppError::io(&target_dir, e))? {
            let entry = entry.map_err(|e| AppError::io(&target_dir, e))?;
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }
            let dir_name = entry
                .file_name()
                .to_string_lossy()
                .to_string();
            if dir_name.starts_with('.') {
                continue;
            }
            if !enabled_dirs.contains(&dir_name) {
                log::info!("Removing orphaned skill from target: {}", dir_name);
                if let Err(e) = fs::remove_dir_all(&path) {
                    log::warn!("Failed to remove orphaned skill '{}': {}", dir_name, e);
                }
            }
        }
    }

    // Phase 2: Copy enabled skills
    fs::create_dir_all(&target_dir).map_err(|e| AppError::io(&target_dir, e))?;

    for skill in &skills {
        let source = ssot_dir.join(&skill.directory);
        let dest = target_dir.join(&skill.directory);

        if !source.exists() {
            log::warn!(
                "Skill '{}' not found in SSOT at {}, skipping",
                skill.directory,
                source.display()
            );
            continue;
        }

        // Remove existing target before copy
        if dest.exists() {
            if let Err(e) = fs::remove_dir_all(&dest) {
                log::warn!(
                    "Failed to remove existing skill dir '{}': {}",
                    dest.display(),
                    e
                );
                continue;
            }
        }

        copy_dir_recursive(&source, &dest)?;
        log::debug!("Synced skill '{}' to {}", skill.directory, dest.display());
    }

    log::info!(
        "Synced {} skills to ~/.claude/skills/ for app_type={}",
        skills.len(),
        app_type
    );
    Ok(())
}

/// Recursively copy a directory.
fn copy_dir_recursive(src: &Path, dest: &Path) -> Result<(), AppError> {
    fs::create_dir_all(dest).map_err(|e| AppError::io(dest, e))?;

    for entry in fs::read_dir(src).map_err(|e| AppError::io(src, e))? {
        let entry = entry.map_err(|e| AppError::io(src, e))?;
        let path = entry.path();
        let dest_path = dest.join(entry.file_name());

        if path.is_dir() {
            copy_dir_recursive(&path, &dest_path)?;
        } else {
            fs::copy(&path, &dest_path).map_err(|e| {
                AppError::IoContext {
                    context: format!("copy {:?} -> {:?}", path.display(), dest_path.display()),
                    source: e,
                }
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
    let ssot_dir = get_ssot_dir();
    let skill_dir = ssot_dir.join(&skill.directory);

    if skill_dir.join("SKILL.md").exists() {
        return Ok(());
    }

    fs::create_dir_all(&skill_dir).map_err(|e| AppError::io(&skill_dir, e))?;

    let content = format!(
        "---\nname: {}\n{}\n---\n",
        skill.name,
        skill
            .description
            .as_ref()
            .map(|d| format!("description: \"{}\"", d.replace('"', "\\\"")))
            .unwrap_or_default(),
    );

    fs::write(skill_dir.join("SKILL.md"), &content)
        .map_err(|e| AppError::io(&skill_dir.join("SKILL.md"), e))?;

    log::info!("Created SSOT for skill '{}' at {}", skill.directory, skill_dir.display());
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
    if depth > 8 {
        return Ok(());
    }

    // Skip cache directory — these are cached copies of marketplace plugins
    if current.file_name().map_or(false, |n| n == "cache") {
        return Ok(());
    }

    // Check if this is a skill directory: parent directory is named "skills"
    if current.join("SKILL.md").exists() {
        let parent_name = current.parent().and_then(|p| p.file_name());
        if parent_name.map_or(false, |n| n == "skills") {
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
        if ancestor.file_name().map_or(false, |n| n == "skills") {
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
    let skill_md = source_dir.join("SKILL.md");
    let (name, description) = parse_skill_md(&skill_md);
    let display_name = if name.is_empty() {
        dir_name.to_string()
    } else {
        name
    };

    // Copy whole skill directory to SSOT
    let ssot_dir = get_ssot_dir();
    let ssot_dest = ssot_dir.join(dir_name);
    if !ssot_dest.exists() {
        fs::create_dir_all(&ssot_dir).map_err(|e| AppError::io(&ssot_dir, e))?;
        copy_dir_recursive(source_dir, &ssot_dest)?;
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

    log::info!("Imported skill '{}' from {}", dir_name, source_dir.display());
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
        for entry in fs::read_dir(&claude_skills_dir)
            .map_err(|e| AppError::io(&claude_skills_dir, e))?
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
                        db.get_all_skills(app_type).ok().and_then(|skills| {
                            skills.into_iter().find(|s| s.directory == dir_name)
                        }).map(|mut s| {
                            s.collection = Some(coll);
                            let _ = db.save_skill(&s);
                            imported += 1;
                        });
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
                    db.get_all_skills(app_type).ok().and_then(|skills| {
                        skills.into_iter().find(|s| s.directory == *dir_name)
                    }).map(|mut s| {
                        s.collection = Some(coll);
                        let _ = db.save_skill(&s);
                        imported += 1;
                    });
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
        assert_eq!(
            fs::read_to_string(dest.join("top.txt")).unwrap(),
            "top"
        );
        assert_eq!(
            fs::read_to_string(dest.join("sub").join("nested.txt")).unwrap(),
            "nested"
        );
    }
}
