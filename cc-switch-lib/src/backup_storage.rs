//! Bounded, metadata-only inventory of known backup locations.
use crate::config;
use serde::Serialize;
use std::{
    fs,
    path::{Path, PathBuf},
    time::UNIX_EPOCH,
};
const LIMIT: usize = 20_000;
const DEPTH: usize = 16;
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupLocation {
    pub id: &'static str,
    pub label: &'static str,
    pub path: Option<String>,
    pub scope: &'static str,
    pub recovery: &'static str,
    pub entries: usize,
    pub size_bytes: u64,
    pub modified_at: Option<u64>,
    pub status: &'static str,
    pub warnings: Vec<String>,
}
fn location(
    id: &'static str,
    label: &'static str,
    path: PathBuf,
    scope: &'static str,
    recovery: &'static str,
) -> BackupLocation {
    let mut item = BackupLocation {
        id,
        label,
        path: Some(path.display().to_string()),
        scope,
        recovery,
        entries: 0,
        size_bytes: 0,
        modified_at: None,
        status: "available",
        warnings: vec![],
    };
    for ancestor in path.ancestors() {
        if fs::symlink_metadata(ancestor).is_ok_and(|meta| meta.file_type().is_symlink()) {
            item.status = "unavailable";
            item.warnings
                .push("Linked backup location or parent; skipped".into());
            return item;
        }
    }
    match fs::symlink_metadata(&path) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            item.status = "missing";
            return item;
        }
        Err(_) => {
            item.status = "unavailable";
            item.warnings
                .push("Could not inspect backup location".into());
            return item;
        }
        Ok(meta) if !meta.is_dir() => {
            item.status = "unavailable";
            item.warnings
                .push("Backup location is not a directory".into());
            return item;
        }
        _ => {}
    }
    let mut remaining = LIMIT;
    walk(&path, 0, &mut remaining, &mut item);
    if !item.warnings.is_empty() {
        item.status = "partial";
    }
    item
}
fn warn(item: &mut BackupLocation, message: &str) {
    if item.warnings.len() < 10 && !item.warnings.iter().any(|warning| warning == message) {
        item.warnings.push(message.into());
    }
}
fn walk(path: &Path, depth: usize, remaining: &mut usize, item: &mut BackupLocation) {
    if depth > DEPTH {
        warn(item, "Depth limit reached; totals are incomplete");
        return;
    }
    let entries = match fs::read_dir(path) {
        Ok(entries) => entries,
        Err(_) => {
            warn(
                item,
                "Some directories could not be read; totals are incomplete",
            );
            return;
        }
    };
    for entry in entries {
        if *remaining == 0 {
            warn(item, "Entry limit reached; totals are incomplete");
            return;
        }
        *remaining -= 1;
        let entry = match entry {
            Ok(entry) => entry,
            Err(_) => {
                warn(
                    item,
                    "Some entries could not be read; totals are incomplete",
                );
                continue;
            }
        };
        let meta = match fs::symlink_metadata(entry.path()) {
            Ok(meta) => meta,
            Err(_) => {
                warn(
                    item,
                    "Some entries could not be inspected; totals are incomplete",
                );
                continue;
            }
        };
        if meta.file_type().is_symlink() {
            warn(item, "Linked entries were skipped; totals are incomplete");
            continue;
        }
        if depth == 0 {
            item.entries += 1;
        }
        if let Some(time) = meta
            .modified()
            .ok()
            .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
            .map(|time| time.as_secs())
        {
            item.modified_at = Some(item.modified_at.map_or(time, |old| old.max(time)));
        }
        if meta.is_dir() {
            walk(&entry.path(), depth + 1, remaining, item);
        } else if meta.is_file() {
            item.size_bytes = item.size_bytes.saturating_add(meta.len());
        }
    }
}
/// The client cannot supply paths. Source builds have no release-backup location.
pub fn list() -> Vec<BackupLocation> {
    let app = config::get_app_config_dir();
    let live = config::get_claude_config_dir();
    let mut items = vec![
        location("database", "Database snapshots", app.join("backups/database"), "Manual and safety database snapshots, plus staged restore metadata.", "Use Database Backups below to review and stage a restore; restart is required."),
        location("migrations", "Database migration protection", app.join("backups/upgrades"), "Database copies kept before a schema upgrade.", "Keep these for upgrade recovery. They are separate from the manual database restore list."),
        location("files", "Original configuration files", app.join("backups/live-first-write"), "Original live files and source markers saved before the first managed write.", "Use Configuration file backups below to inspect the original snapshot locations."),
        location("skills-live", "Skills live backups", live.join("skills/.cc-switch-backups"), "Managed live Skills directories retained before replacement, disabling, or removal.", "Use Skills recovery below to select and restore files."),
        location("skills-source", "Skills source backups", app.join("skills/.cc-switch-backups"), "Previous versions of managed Skill source directories.", "Use Skills recovery below; enabled Skills are synced after restoration."),
    ];
    let release_dir = std::env::current_exe()
        .ok()
        .and_then(|path| path.parent().map(Path::to_path_buf))
        .filter(|path| path.join("dist/index.html").is_file());
    if let Some(parent) = release_dir.and_then(|path| path.parent().map(Path::to_path_buf)) {
        items.push(location("installation", "Installation upgrade copies", parent.join("cc-switch-upgrade-backups"), "Upgrade folders containing the previous program and a complete profile archive; interrupted upgrades may leave incomplete folders.", "Use the documented installation rollback procedure. These copies are not restored through the database-only action."));
    } else {
        items.push(BackupLocation { id:"installation", label:"Installation upgrade copies", path:None, scope:"Previous program and complete profile archives from release upgrades.", recovery:"Available when running a release installation; source builds do not identify an installation backup directory.", entries:0, size_bytes:0, modified_at:None, status:"unavailable", warnings:vec![] });
    }
    items
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn inventory_is_metadata_only_and_distinguishes_missing_from_invalid() {
        let root = tempfile::tempdir().unwrap();
        let file = root.path().join("snapshot.db");
        fs::write(&file, b"private-credential-contents").unwrap();
        let item = location("test", "Test", root.path().into(), "scope", "recovery");
        assert_eq!(item.entries, 1);
        assert_eq!(item.size_bytes, 27);
        assert_eq!(item.status, "available");
        assert!(!serde_json::to_string(&item)
            .unwrap()
            .contains("private-credential"));
        assert_eq!(fs::read(file).unwrap(), b"private-credential-contents");
        assert_eq!(
            location(
                "test",
                "Test",
                root.path().join("missing"),
                "scope",
                "recovery"
            )
            .status,
            "missing"
        );
        fs::write(root.path().join("invalid"), b"x").unwrap();
        assert_eq!(
            location(
                "test",
                "Test",
                root.path().join("invalid"),
                "scope",
                "recovery"
            )
            .status,
            "unavailable"
        );
    }
    #[cfg(unix)]
    #[test]
    fn symlinks_are_skipped_and_bounded_scans_report_incomplete_totals() {
        let root = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        fs::write(outside.path().join("secret"), b"not counted").unwrap();
        std::os::unix::fs::symlink(outside.path(), root.path().join("linked")).unwrap();
        let mut item = location("test", "Test", root.path().into(), "scope", "recovery");
        assert_eq!(item.status, "partial");
        assert_eq!(item.size_bytes, 0);
        fs::write(root.path().join("one"), b"x").unwrap();
        let mut remaining = 0;
        walk(root.path(), 0, &mut remaining, &mut item);
        assert!(item
            .warnings
            .iter()
            .any(|warning| warning.contains("limit")));
        let linked = location(
            "test",
            "Test",
            root.path().join("linked/child"),
            "scope",
            "recovery",
        );
        assert_eq!(linked.status, "unavailable");
    }
}
