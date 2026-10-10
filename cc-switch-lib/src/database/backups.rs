//! Private SQLite snapshots and staged restores applied before the server starts.
use super::{migrations, Database};
use crate::{config, error::AppError, providers::AppType, settings};
use rusqlite::{
    backup::{Backup, StepResult},
    Connection, OpenFlags,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeSet,
    fs,
    io::Read,
    path::{Path, PathBuf},
    sync::Mutex,
    time::{Duration, Instant},
};

static OPERATIONS: Mutex<()> = Mutex::new(());
const MAX_MANUAL: usize = 20;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Entry {
    pub id: String,
    pub kind: String,
    pub size_bytes: u64,
    pub created_at: String,
    pub protected: bool,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Pending {
    pub source_id: String,
    staging_id: String,
    digest: String,
    database_path: PathBuf,
    safety_id: Option<String>,
    applying: bool,
    completed: bool,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PendingStatus {
    pub source_id: String,
    pub applying: bool,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Status {
    pub entries: Vec<Entry>,
    pub pending: Option<PendingStatus>,
    pub directory: String,
    pub manual_limit: usize,
}
fn root() -> PathBuf {
    config::get_app_config_dir().join("backups/database")
}
fn manifest() -> PathBuf {
    root().join("pending-restore.json")
}
fn refuse_link(path: &Path) -> Result<(), AppError> {
    match fs::symlink_metadata(path) {
        Ok(meta) if meta.file_type().is_symlink() => Err(AppError::InvalidInput(
            "Linked backup paths are not supported".into(),
        )),
        Ok(_) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(AppError::io(path, e)),
    }
}
fn ensure_root() -> Result<(), AppError> {
    refuse_link(&root())?;
    fs::create_dir_all(root()).map_err(|e| AppError::io(root(), e))
}
fn path_for(id: &str, allow_staging: bool) -> Result<PathBuf, AppError> {
    let valid = ["manual-", "safety-", "staged-"].iter().any(|prefix| {
        if *prefix == "staged-" && !allow_staging {
            return false;
        }
        id.strip_prefix(prefix)
            .and_then(|s| s.strip_suffix(".db"))
            .is_some_and(|s| uuid::Uuid::parse_str(s).is_ok() && s.len() == 36)
    });
    if !valid {
        return Err(AppError::InvalidInput("Invalid database backup ID".into()));
    }
    ensure_root()?;
    let path = root().join(id);
    refuse_link(&path)?;
    Ok(path)
}
fn pending() -> Result<Option<Pending>, AppError> {
    refuse_link(&manifest())?;
    match fs::read(manifest()) {
        Ok(bytes) => {
            let p: Pending =
                serde_json::from_slice(&bytes).map_err(|e| AppError::json(manifest(), e))?;
            if !p.staging_id.starts_with("staged-")
                || p.safety_id
                    .as_ref()
                    .is_some_and(|s| !s.starts_with("safety-"))
                || (p.applying && p.safety_id.is_none())
                || (p.completed && !p.applying)
                || p.digest.len() != 64
                || !p.digest.bytes().all(|b| b.is_ascii_hexdigit())
            {
                return Err(AppError::InvalidInput("Invalid restore manifest".into()));
            }
            path_for(&p.staging_id, true)?;
            path_for(&p.source_id, false)?;
            if let Some(id) = &p.safety_id {
                path_for(id, false)?;
            }
            Ok(Some(p))
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(AppError::io(manifest(), e)),
    }
}
fn hash(path: &Path) -> Result<String, AppError> {
    let mut file = fs::File::open(path).map_err(|e| AppError::io(path, e))?;
    let mut hash = Sha256::new();
    let mut buffer = [0; 65536];
    loop {
        let n = file.read(&mut buffer).map_err(|e| AppError::io(path, e))?;
        if n == 0 {
            break;
        }
        hash.update(&buffer[..n]);
    }
    Ok(hex::encode(hash.finalize()))
}
fn copy_sqlite(source: &Connection, destination: &mut Connection) -> Result<(), AppError> {
    let backup = Backup::new(source, destination)?;
    let start = Instant::now();
    loop {
        if start.elapsed() > Duration::from_secs(30) {
            return Err(AppError::Database(
                "Database snapshot timed out; retry when other writers finish".into(),
            ));
        }
        match backup.step(256)? {
            StepResult::Done => return Ok(()),
            StepResult::Busy | StepResult::Locked => std::thread::sleep(Duration::from_millis(10)),
            StepResult::More => {}
            _ => return Err(AppError::Database("Unexpected SQLite backup status".into())),
        }
    }
}
/// Snapshot existing databases before any upgrade DDL, including unversioned releases.
pub(super) fn before_upgrade(
    source: &Connection,
    database_path: &Path,
    version: u32,
) -> Result<(), AppError> {
    let directory = database_path
        .parent()
        .unwrap_or(Path::new("."))
        .join("backups/upgrades");
    refuse_link(
        &database_path
            .parent()
            .unwrap_or(Path::new("."))
            .join("backups"),
    )?;
    refuse_link(&directory)?;
    fs::create_dir_all(&directory).map_err(|e| AppError::io(&directory, e))?;
    let temporary =
        tempfile::NamedTempFile::new_in(&directory).map_err(|e| AppError::io(&directory, e))?;
    let mut destination = Connection::open(temporary.path())?;
    copy_sqlite(source, &mut destination)?;
    integrity(&destination)?;
    drop(destination);
    temporary
        .as_file()
        .sync_all()
        .map_err(|e| AppError::io(temporary.path(), e))?;
    let path = directory.join(format!(
        "schema-{version}-to-{}-{}.db",
        migrations::SCHEMA_VERSION,
        uuid::Uuid::new_v4()
    ));
    temporary
        .persist_noclobber(&path)
        .map_err(|e| AppError::io(&path, e.error))?;
    log::info!("Pre-upgrade database snapshot: {}", path.display());
    Ok(())
}

fn integrity(conn: &Connection) -> Result<(), AppError> {
    let result: String = conn.query_row("PRAGMA integrity_check", [], |row| row.get(0))?;
    if result != "ok" {
        return Err(AppError::InvalidInput(
            "Database integrity check failed".into(),
        ));
    }
    if conn
        .prepare("PRAGMA foreign_key_check")?
        .query([])?
        .next()?
        .is_some()
    {
        return Err(AppError::InvalidInput(
            "Database contains invalid references".into(),
        ));
    }
    Ok(())
}
fn columns(conn: &Connection, table: &str) -> Result<BTreeSet<String>, AppError> {
    Ok(conn
        .prepare(&format!("PRAGMA table_info({table})"))?
        .query_map([], |r| r.get(1))?
        .collect::<Result<_, _>>()?)
}
fn column_definitions(
    conn: &Connection,
    table: &str,
) -> Result<BTreeSet<(String, String, bool, u32)>, AppError> {
    Ok(conn
        .prepare(&format!("PRAGMA table_info({table})"))?
        .query_map([], |r| {
            Ok((
                r.get(1)?,
                r.get::<_, String>(2)?.to_uppercase(),
                r.get(3)?,
                r.get(5)?,
            ))
        })?
        .collect::<Result<_, _>>()?)
}
fn tables(conn: &Connection) -> Result<BTreeSet<String>, AppError> {
    Ok(conn
        .prepare(
            "SELECT name FROM sqlite_master WHERE type = 'table' AND name NOT LIKE 'sqlite_%'",
        )?
        .query_map([], |r| r.get(0))?
        .collect::<Result<_, _>>()?)
}
fn validate_and_migrate(conn: Connection) -> Result<Database, AppError> {
    integrity(&conn)?;
    let version: u32 = conn.pragma_query_value(None, "user_version", |r| r.get(0))?;
    if version > migrations::SCHEMA_VERSION {
        return Err(AppError::InvalidInput(
            "Backup was created by a newer schema version".into(),
        ));
    }
    let unsafe_objects: u32 = conn.query_row(
        "SELECT COUNT(*) FROM sqlite_master WHERE type IN ('trigger', 'view')",
        [],
        |r| r.get(0),
    )?;
    if unsafe_objects != 0 {
        return Err(AppError::InvalidInput(
            "Backup contains unsupported triggers or views".into(),
        ));
    }
    let provider_columns = columns(&conn, "providers")?;
    if ![
        "id",
        "app_type",
        "name",
        "settings_config",
        "meta",
        "is_current",
    ]
    .iter()
    .all(|c| provider_columns.contains(*c))
    {
        return Err(AppError::InvalidInput(
            "Not a supported CC Switch UI database".into(),
        ));
    }
    let reference = Database {
        conn: Mutex::new(Connection::open_in_memory()?),
    };
    reference.create_tables()?;
    let expected = tables(&reference.conn())?;
    if !tables(&conn)?.is_subset(&expected) {
        return Err(AppError::InvalidInput(
            "Backup contains unsupported database tables".into(),
        ));
    }
    let staged = Database {
        conn: Mutex::new(conn),
    };
    staged.create_tables()?;
    {
        let conn = staged.conn();
        for table in expected {
            if column_definitions(&conn, &table)? != column_definitions(&reference.conn(), &table)?
            {
                return Err(AppError::InvalidInput(format!(
                    "Unsupported schema for {table}"
                )));
            }
        }
        integrity(&conn)?;
        if crate::managed_deletion::pending_on(&conn)? {
            return Err(AppError::InvalidInput(
                "Backup contains unfinished managed deletion".into(),
            ));
        }
        let routes: u32 =
            conn.query_row("SELECT COUNT(*) FROM proxy_live_backup", [], |r| r.get(0))?;
        if routes != 0 {
            return Err(AppError::InvalidInput(
                "Backup contains an unfinished local route; use a snapshot made after stopping it"
                    .into(),
            ));
        }
        let invalid_target: u32 = conn.query_row("SELECT COUNT(*) FROM proxy_target_config t WHERE t.active_target_provider_id IS NOT NULL AND NOT EXISTS(SELECT 1 FROM providers p WHERE p.id=t.active_target_provider_id AND p.app_type='claude_code')", [], |r| r.get(0))?;
        if invalid_target != 0 {
            return Err(AppError::InvalidInput(
                "Backup contains a missing route target".into(),
            ));
        }
        let mut stmt = conn.prepare("SELECT settings_config, meta FROM providers")?;
        for row in stmt.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))? {
            let (settings, meta) = row?;
            let value: serde_json::Value = serde_json::from_str(&settings)
                .map_err(|_| AppError::InvalidInput("Invalid provider data in backup".into()))?;
            if !value.is_object()
                || value.get("env").is_some_and(|e| !e.is_object())
                || serde_json::from_str::<serde_json::Value>(&meta).is_err()
            {
                return Err(AppError::InvalidInput(
                    "Invalid provider configuration in backup".into(),
                ));
            }
        }
    }
    Ok(staged)
}
fn snapshot(source: &Connection, kind: &str) -> Result<String, AppError> {
    ensure_root()?;
    let file = tempfile::NamedTempFile::new_in(root()).map_err(|e| AppError::io(root(), e))?;
    let mut target = Connection::open(file.path())?;
    copy_sqlite(source, &mut target)?;
    integrity(&target)?;
    drop(target);
    file.as_file()
        .sync_all()
        .map_err(|e| AppError::io(file.path(), e))?;
    let id = format!("{kind}-{}.db", uuid::Uuid::new_v4());
    file.persist_noclobber(path_for(&id, true)?)
        .map_err(|e| AppError::io(root(), e.error))?;
    Ok(id)
}
fn status_unlocked() -> Result<Status, AppError> {
    ensure_root()?;
    let pending = pending()?;
    let mut entries = Vec::new();
    for entry in fs::read_dir(root()).map_err(|e| AppError::io(root(), e))? {
        let entry = entry.map_err(|e| AppError::io(root(), e))?;
        let id = entry.file_name().to_string_lossy().to_string();
        if !(id.starts_with("manual-") || id.starts_with("safety-"))
            || path_for(&id, false).is_err()
        {
            continue;
        }
        let meta = fs::symlink_metadata(entry.path()).map_err(|e| AppError::io(entry.path(), e))?;
        if !meta.is_file() {
            continue;
        }
        let created: chrono::DateTime<chrono::Utc> = meta
            .modified()
            .map_err(|e| AppError::io(entry.path(), e))?
            .into();
        let protected = pending
            .as_ref()
            .is_some_and(|p| p.source_id == id || p.safety_id.as_deref() == Some(&id));
        entries.push(Entry {
            kind: if id.starts_with("safety-") {
                "safety"
            } else {
                "manual"
            }
            .into(),
            id,
            size_bytes: meta.len(),
            created_at: created.to_rfc3339(),
            protected,
        });
    }
    entries.sort_by(|a, b| b.created_at.cmp(&a.created_at));
    if let Some(index) = entries.iter().position(|e| e.kind == "safety") {
        entries[index].protected = true;
    }
    Ok(Status {
        entries,
        pending: pending.map(|p| PendingStatus {
            source_id: p.source_id,
            applying: p.applying,
        }),
        directory: root().display().to_string(),
        manual_limit: MAX_MANUAL,
    })
}
pub fn restore_pending() -> Result<bool, AppError> {
    let _guard = OPERATIONS.lock()?;
    Ok(pending()?.is_some())
}
pub fn status() -> Result<Status, AppError> {
    let _guard = OPERATIONS.lock()?;
    status_unlocked()
}
pub fn create(db: &Database) -> Result<String, AppError> {
    let _guard = OPERATIONS.lock()?;
    if status_unlocked()?
        .entries
        .iter()
        .filter(|e| e.kind == "manual")
        .count()
        >= MAX_MANUAL
    {
        return Err(AppError::InvalidInput(
            "Manual backup limit reached; delete an old manual backup first".into(),
        ));
    }
    let conn = db.conn();
    if crate::managed_deletion::pending_on(&conn)? {
        return Err(AppError::ProviderInUse(
            "Finish managed deletion before creating a database backup".into(),
        ));
    }
    snapshot(&conn, "manual")
}
pub fn delete(id: &str) -> Result<(), AppError> {
    let _guard = OPERATIONS.lock()?;
    let path = path_for(id, false)?;
    if status_unlocked()?
        .entries
        .iter()
        .any(|entry| entry.id == id && entry.protected)
    {
        return Err(AppError::ProviderInUse(
            "The latest safety backup and backups used by a pending restore are protected".into(),
        ));
    }
    fs::remove_file(&path).map_err(|e| AppError::io(path, e))
}
pub fn schedule(id: &str) -> Result<(), AppError> {
    let _guard = OPERATIONS.lock()?;
    if pending()?.is_some() {
        return Err(AppError::ProviderInUse(
            "Cancel the pending restore before selecting another backup".into(),
        ));
    }
    let source =
        Connection::open_with_flags(path_for(id, false)?, OpenFlags::SQLITE_OPEN_READ_ONLY)?;
    let file = tempfile::NamedTempFile::new_in(root()).map_err(|e| AppError::io(root(), e))?;
    let mut target = Connection::open(file.path())?;
    copy_sqlite(&source, &mut target)?;
    drop(source);
    let staged = validate_and_migrate(target)?;
    // Live and device files are independent: never associate their current
    // credentials with an older restored provider record for later backfill.
    staged.conn().execute(
        "UPDATE providers SET is_current=0 WHERE app_type=?1",
        [crate::DEFAULT_APP_TYPE],
    )?;
    drop(staged);
    file.as_file()
        .sync_all()
        .map_err(|e| AppError::io(file.path(), e))?;
    let staging_id = format!("staged-{}.db", uuid::Uuid::new_v4());
    let staging = path_for(&staging_id, true)?;
    let pending = Pending {
        source_id: id.into(),
        staging_id,
        digest: hash(file.path())?,
        database_path: config::get_app_config_dir().join("cc-switch.db"),
        safety_id: None,
        applying: false,
        completed: false,
    };
    file.persist_noclobber(&staging)
        .map_err(|e| AppError::io(&staging, e.error))?;
    if let Err(e) = config::write_json_file(&manifest(), &pending) {
        let _ = fs::remove_file(staging);
        return Err(e);
    }
    Ok(())
}
fn cleanup(pending: &Pending) -> Result<(), AppError> {
    let path = path_for(&pending.staging_id, true)?;
    match fs::remove_file(&path) {
        Ok(()) => {}
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(e) => return Err(AppError::io(path, e)),
    }
    fs::remove_file(manifest()).map_err(|e| AppError::io(manifest(), e))
}
pub fn cancel() -> Result<(), AppError> {
    let _guard = OPERATIONS.lock()?;
    if let Some(p) = pending()? {
        if p.applying {
            return Err(AppError::ProviderInUse(
                "Restore already started; restart to complete it".into(),
            ));
        }
        cleanup(&p)?;
    }
    Ok(())
}
/// Only for an explicit destructive clean; prevent credentials replay later.
pub fn discard_for_clean() -> Result<(), AppError> {
    let _guard = OPERATIONS.lock()?;
    if let Some(p) = pending()? {
        cleanup(&p)?;
    }
    Ok(())
}
/// Apply before any server workers start. Retry keeps the original safety snapshot.
pub fn apply_pending() -> Result<(), AppError> {
    let _guard = OPERATIONS.lock()?;
    let Some(mut p) = pending()? else {
        return Ok(());
    };
    if p.database_path != config::get_app_config_dir().join("cc-switch.db") {
        return Err(AppError::InvalidInput(
            "Restore configuration directory changed".into(),
        ));
    }
    if p.completed {
        return cleanup(&p);
    }
    let staging = path_for(&p.staging_id, true)?;
    if hash(&staging)? != p.digest {
        return Err(AppError::InvalidInput(
            "Staged restore changed; cancel it before retrying".into(),
        ));
    }
    let source = Connection::open_with_flags(&staging, OpenFlags::SQLITE_OPEN_READ_ONLY)?;
    integrity(&source)?;
    if crate::provider_operations::recovery_pending() {
        return Err(AppError::ProviderInUse(
            "Recover the pending provider operation before restoring".into(),
        ));
    }
    refuse_link(&p.database_path)?;
    let mut current =
        Connection::open_with_flags(&p.database_path, OpenFlags::SQLITE_OPEN_READ_WRITE)?;
    let routes: u32 =
        current.query_row("SELECT COUNT(*) FROM proxy_live_backup", [], |r| r.get(0))?;
    if routes != 0 {
        return Err(AppError::ProviderInUse(
            "Restore direct config after the interrupted route before restoring the database"
                .into(),
        ));
    }
    if crate::managed_deletion::pending_on(&current)? {
        return Err(AppError::ProviderInUse(
            "Finish managed deletion before restoring the database".into(),
        ));
    }
    if p.safety_id.is_none() {
        p.safety_id = Some(snapshot(&current, "safety")?);
        config::write_json_file(&manifest(), &p)?;
    }
    if let Some(id) = &p.safety_id {
        let safety =
            Connection::open_with_flags(path_for(id, false)?, OpenFlags::SQLITE_OPEN_READ_ONLY)?;
        integrity(&safety)?;
    }
    p.applying = true;
    config::write_json_file(&manifest(), &p)?;
    copy_sqlite(&source, &mut current)?;
    drop(current);
    drop(source);
    settings::reload_settings()?;
    settings::set_current_provider(&AppType::ClaudeCode, None)?;
    p.completed = true;
    config::write_json_file(&manifest(), &p)?;
    cleanup(&p)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        database::Provider,
        live::tests::{acquire_env_lock, cleanup as cleanup_home, setup_test_home},
    };
    use serde_json::json;
    fn fixture() -> (Database, Provider) {
        let db = Database::init().unwrap();
        settings::reload_settings().unwrap();
        let provider:Provider=serde_json::from_value(json!({"id":"backup-test", "name":"Original", "settingsConfig":{"env":{"ANTHROPIC_AUTH_TOKEN":"old-key"}}})).unwrap();
        db.save_provider(crate::DEFAULT_APP_TYPE, &provider)
            .unwrap();
        db.set_current_provider(&provider.id, crate::DEFAULT_APP_TYPE)
            .unwrap();
        settings::set_current_provider(&AppType::ClaudeCode, Some(&provider.id)).unwrap();
        (db, provider)
    }
    #[test]
    fn managed_deletion_intents_cannot_be_backed_up_or_replayed_by_restore() {
        let _env = acquire_env_lock();
        let home = setup_test_home();
        let (db, _) = fixture();
        let valid = create(&db).unwrap();
        db.conn()
            .execute(
                "INSERT INTO managed_deletion(id,intent_json) VALUES (1,'{}')",
                [],
            )
            .unwrap();
        assert!(create(&db).is_err());
        // Simulate a separately supplied SQLite snapshot containing an unfinished cleanup.
        let unfinished = snapshot(&db.conn(), "manual").unwrap();
        assert!(schedule(&unfinished).is_err());
        schedule(&valid).unwrap();
        assert!(apply_pending().is_err());
        assert!(crate::managed_deletion::pending_on(&db.conn()).unwrap());
        db.conn()
            .execute("DELETE FROM managed_deletion", [])
            .unwrap();
        assert!(crate::managed_deletion::begin(&db, "mcp", "missing").is_err());
        cancel().unwrap();
        drop(db);
        cleanup_home(home);
        settings::reload_settings().unwrap();
    }

    #[test]
    fn wal_snapshot_restores_on_restart_and_retains_exact_safety_state() {
        let _env = acquire_env_lock();
        let home = setup_test_home();
        let (db, mut provider) = fixture();
        db.conn()
            .pragma_update(None, "journal_mode", "WAL")
            .unwrap();
        let id = create(&db).unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(
                fs::metadata(path_for(&id, false).unwrap())
                    .unwrap()
                    .permissions()
                    .mode()
                    & 0o777,
                0o600
            );
        }
        provider.name = "Latest".into();
        provider.settings_config["env"]["ANTHROPIC_AUTH_TOKEN"] = json!("new-key");
        db.save_provider(crate::DEFAULT_APP_TYPE, &provider)
            .unwrap();
        let live = config::get_claude_settings_path();
        config::write_json_file(
            &live,
            &json!({"env":{"ANTHROPIC_AUTH_TOKEN":"new-key","API_TIMEOUT_MS":"333"}}),
        )
        .unwrap();
        let live_before = fs::read(&live).unwrap();
        schedule(&id).unwrap();
        assert!(restore_pending().unwrap());
        assert_eq!(
            db.get_provider(&provider.id, crate::DEFAULT_APP_TYPE)
                .unwrap()
                .unwrap()
                .name,
            "Latest"
        );
        assert!(delete(&id).is_err());
        assert!(schedule(&id).is_err());
        // Changes after scheduling must survive in the automatic safety snapshot.
        provider.name = "Before restart".into();
        db.save_provider(crate::DEFAULT_APP_TYPE, &provider)
            .unwrap();
        drop(db);
        apply_pending().unwrap();
        assert!(!restore_pending().unwrap());
        let restored = Database::init().unwrap();
        assert_eq!(
            restored
                .get_provider(&provider.id, crate::DEFAULT_APP_TYPE)
                .unwrap()
                .unwrap()
                .name,
            "Original"
        );
        assert!(restored
            .get_current_provider_id(crate::DEFAULT_APP_TYPE)
            .unwrap()
            .is_none());
        assert!(settings::get_current_provider(&AppType::ClaudeCode).is_none());
        assert_eq!(fs::read(&live).unwrap(), live_before);
        let entries = status().unwrap().entries;
        let safety = entries.iter().find(|e| e.kind == "safety").unwrap();
        assert!(safety.protected);
        assert!(delete(&safety.id).is_err());
        let conn = Connection::open_with_flags(
            path_for(&safety.id, false).unwrap(),
            OpenFlags::SQLITE_OPEN_READ_ONLY,
        )
        .unwrap();
        let name: String = conn
            .query_row(
                "SELECT name FROM providers WHERE id='backup-test'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(name, "Before restart");
        drop(conn);
        apply_pending().unwrap();
        assert_eq!(
            status()
                .unwrap()
                .entries
                .iter()
                .filter(|e| e.kind == "safety")
                .count(),
            1
        );
        drop(restored);
        cleanup_home(home);
        settings::reload_settings().unwrap();
    }
    #[test]
    fn corrupt_future_foreign_and_linked_sources_do_not_stage_or_replace() {
        let _env = acquire_env_lock();
        let home = setup_test_home();
        let (db, provider) = fixture();
        let id = create(&db).unwrap();
        let path = path_for(&id, false).unwrap();
        let bytes = fs::read(&path).unwrap();
        fs::write(&path, "not sqlite").unwrap();
        assert!(schedule(&id).is_err());
        assert!(!restore_pending().unwrap());
        fs::write(&path, &bytes).unwrap();
        let conn = Connection::open(&path).unwrap();
        conn.pragma_update(None, "user_version", 999).unwrap();
        drop(conn);
        assert!(schedule(&id).is_err());
        fs::write(&path, &bytes).unwrap();
        let conn = Connection::open(&path).unwrap();
        conn.execute_batch(
            "CREATE TRIGGER unsupported AFTER INSERT ON providers BEGIN SELECT 1; END;",
        )
        .unwrap();
        drop(conn);
        assert!(schedule(&id).is_err());
        let missing = format!("manual-{}.db", uuid::Uuid::new_v4());
        assert!(schedule(&missing).is_err());
        assert!(!path_for(&missing, false).unwrap().exists());
        for id in [
            "../../cc-switch.db",
            "/tmp/evil.db",
            "manual-invalid.db",
            "staged-00000000-0000-0000-0000-000000000000.db",
        ] {
            assert!(schedule(id).is_err());
            assert!(delete(id).is_err());
        }
        #[cfg(unix)]
        {
            fs::remove_file(&path).unwrap();
            std::os::unix::fs::symlink(config::get_app_config_dir().join("cc-switch.db"), &path)
                .unwrap();
            assert!(schedule(&id).is_err());
            assert!(delete(&id).is_err());
        }
        assert_eq!(
            db.get_provider(&provider.id, crate::DEFAULT_APP_TYPE)
                .unwrap()
                .unwrap()
                .name,
            "Original"
        );
        assert!(!restore_pending().unwrap());
        drop(db);
        cleanup_home(home);
        settings::reload_settings().unwrap();
    }
    #[test]
    fn changed_staging_is_blocked_and_can_be_cancelled_without_server() {
        let _env = acquire_env_lock();
        let home = setup_test_home();
        let (db, provider) = fixture();
        let id = create(&db).unwrap();
        schedule(&id).unwrap();
        let p = pending().unwrap().unwrap();
        fs::write(path_for(&p.staging_id, true).unwrap(), "changed").unwrap();
        drop(db);
        assert!(apply_pending().is_err());
        assert!(status().unwrap().entries.iter().all(|e| e.kind != "safety"));
        cancel().unwrap();
        assert!(!restore_pending().unwrap());
        let db = Database::init().unwrap();
        assert_eq!(
            db.get_provider(&provider.id, crate::DEFAULT_APP_TYPE)
                .unwrap()
                .unwrap()
                .name,
            "Original"
        );
        delete(&id).unwrap();
        assert!(status().unwrap().entries.is_empty());
        drop(db);
        cleanup_home(home);
        settings::reload_settings().unwrap();
    }
    #[test]
    fn failed_device_reset_retries_without_overwriting_original_safety_snapshot() {
        let _env = acquire_env_lock();
        let home = setup_test_home();
        let (db, mut provider) = fixture();
        let id = create(&db).unwrap();
        provider.name = "Safety state".into();
        db.save_provider(crate::DEFAULT_APP_TYPE, &provider)
            .unwrap();
        schedule(&id).unwrap();
        drop(db);
        let device = config::get_app_config_dir().join("settings.json");
        let retained = device.with_extension("retained");
        fs::rename(&device, &retained).unwrap();
        fs::create_dir(&device).unwrap();
        assert!(apply_pending().is_err());
        let p = pending().unwrap().unwrap();
        assert!(p.applying);
        let safety = p.safety_id.unwrap();
        assert!(cancel().is_err());
        let safety_digest = hash(&path_for(&safety, false).unwrap()).unwrap();
        fs::remove_dir(&device).unwrap();
        fs::rename(&retained, &device).unwrap();
        apply_pending().unwrap();
        assert!(!restore_pending().unwrap());
        assert_eq!(
            hash(&path_for(&safety, false).unwrap()).unwrap(),
            safety_digest
        );
        assert!(settings::get_current_provider(&AppType::ClaudeCode).is_none());
        let db = Database::init().unwrap();
        assert_eq!(
            db.get_provider(&provider.id, crate::DEFAULT_APP_TYPE)
                .unwrap()
                .unwrap()
                .name,
            "Original"
        );
        drop(db);
        cleanup_home(home);
        settings::reload_settings().unwrap();
    }
    #[test]
    fn clean_discards_staged_restore_to_prevent_credentials_replay() {
        let _env = acquire_env_lock();
        let home = setup_test_home();
        let (db, _) = fixture();
        let id = create(&db).unwrap();
        schedule(&id).unwrap();
        let staged = path_for(&pending().unwrap().unwrap().staging_id, true).unwrap();
        discard_for_clean().unwrap();
        assert!(!staged.exists());
        assert!(!restore_pending().unwrap());
        drop(db);
        apply_pending().unwrap();
        cleanup_home(home);
        settings::reload_settings().unwrap();
    }
    #[test]
    fn manual_limit_and_latest_safety_protection_allow_deliberate_cleanup() {
        let _env = acquire_env_lock();
        let home = setup_test_home();
        let (db, _) = fixture();
        let mut ids = Vec::new();
        for _ in 0..MAX_MANUAL {
            ids.push(create(&db).unwrap());
        }
        assert!(create(&db).is_err());
        delete(&ids[0]).unwrap();
        assert!(create(&db).is_ok());
        let older = snapshot(&db.conn(), "safety").unwrap();
        // Filesystem timestamp resolution can tie snapshots created in one tick.
        fs::File::open(path_for(&older, false).unwrap())
            .unwrap()
            .set_times(fs::FileTimes::new().set_modified(std::time::SystemTime::UNIX_EPOCH))
            .unwrap();
        let newer = snapshot(&db.conn(), "safety").unwrap();
        assert!(delete(&newer).is_err());
        delete(&older).unwrap();
        assert!(path_for(&newer, false).unwrap().exists());
        drop(db);
        cleanup_home(home);
        settings::reload_settings().unwrap();
    }
    #[test]
    fn legacy_snapshot_is_migrated_in_staging_before_restart() {
        let _env = acquire_env_lock();
        let home = setup_test_home();
        let (db, _) = fixture();
        let id = create(&db).unwrap();
        let conn = Connection::open(path_for(&id, false).unwrap()).unwrap();
        conn.execute_batch("ALTER TABLE proxy_live_backup DROP COLUMN takeover_env; ALTER TABLE skills DROP COLUMN collection; PRAGMA user_version=1;").unwrap();
        drop(conn);
        schedule(&id).unwrap();
        let p = pending().unwrap().unwrap();
        let staged = Connection::open_with_flags(
            path_for(&p.staging_id, true).unwrap(),
            OpenFlags::SQLITE_OPEN_READ_ONLY,
        )
        .unwrap();
        assert!(columns(&staged, "proxy_live_backup")
            .unwrap()
            .contains("takeover_env"));
        assert!(columns(&staged, "skills").unwrap().contains("collection"));
        let version: u32 = staged
            .pragma_query_value(None, "user_version", |r| r.get(0))
            .unwrap();
        assert_eq!(version, migrations::SCHEMA_VERSION);
        drop(staged);
        cancel().unwrap();
        drop(db);
        cleanup_home(home);
        settings::reload_settings().unwrap();
    }
}
