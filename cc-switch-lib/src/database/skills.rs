//! Skill CRUD operations.

use crate::database::types::SkillRecord;
use crate::error::AppError;
use rusqlite::params;

impl super::Database {
    pub fn get_all_skills(&self, app_type: &str) -> Result<Vec<SkillRecord>, AppError> {
        let conn = self.conn();
        let mut stmt = conn.prepare(
            "SELECT id, name, description, directory, app_type, enabled,
                    collection, installed_at, repo_owner, repo_name, repo_branch, readme_url
             FROM skills WHERE app_type = ?1",
        )?;
        let rows = stmt.query_map(params![app_type], |row| {
            Ok(SkillRecord {
                id: row.get(0)?,
                name: row.get(1)?,
                description: row.get(2)?,
                directory: row.get(3)?,
                app_type: row.get(4)?,
                enabled: row.get::<_, i32>(5)? != 0,
                collection: row.get(6)?,
                installed_at: row.get(7)?,
                repo_owner: row.get(8)?,
                repo_name: row.get(9)?,
                repo_branch: row.get(10)?,
                readme_url: row.get(11)?,
            })
        })?;
        let mut result = Vec::new();
        for r in rows {
            result.push(r.map_err(|e| AppError::Database(e.to_string()))?);
        }
        Ok(result)
    }

    pub fn get_enabled_skills(&self, app_type: &str) -> Result<Vec<SkillRecord>, AppError> {
        let conn = self.conn();
        let mut stmt = conn.prepare(
            "SELECT id, name, description, directory, app_type, enabled,
                    collection, installed_at, repo_owner, repo_name, repo_branch, readme_url
             FROM skills WHERE app_type = ?1 AND enabled = 1",
        )?;
        let rows = stmt.query_map(params![app_type], |row| {
            Ok(SkillRecord {
                id: row.get(0)?,
                name: row.get(1)?,
                description: row.get(2)?,
                directory: row.get(3)?,
                app_type: row.get(4)?,
                enabled: row.get::<_, i32>(5)? != 0,
                collection: row.get(6)?,
                installed_at: row.get(7)?,
                repo_owner: row.get(8)?,
                repo_name: row.get(9)?,
                repo_branch: row.get(10)?,
                readme_url: row.get(11)?,
            })
        })?;
        let mut result = Vec::new();
        for r in rows {
            result.push(r.map_err(|e| AppError::Database(e.to_string()))?);
        }
        Ok(result)
    }

    pub fn validate_skill(&self, skill: &SkillRecord) -> Result<(), AppError> {
        crate::skills::validate_directory(&skill.directory)?;
        let conn = self.conn();
        let conflict: bool = conn.query_row(
            "SELECT EXISTS(SELECT 1 FROM skills WHERE directory = ?1 AND app_type = ?2 AND id != ?3)",
            params![skill.directory, skill.app_type, skill.id], |row| row.get(0),
        )?;
        let renamed: bool = conn.query_row(
            "SELECT EXISTS(SELECT 1 FROM skills WHERE id = ?1 AND (directory != ?2 OR app_type != ?3))",
            params![skill.id, skill.directory, skill.app_type], |row| row.get(0),
        )?;
        if conflict || renamed {
            return Err(AppError::InvalidInput(
                "Skill directory is already managed or cannot be changed".into(),
            ));
        }
        Ok(())
    }

    pub fn save_skill(&self, skill: &SkillRecord) -> Result<(), AppError> {
        self.validate_skill(skill)?;
        let conn = self.conn();
        conn.execute(
            "INSERT INTO skills (id, name, description, directory, app_type, enabled, collection,
                installed_at, repo_owner, repo_name, repo_branch, readme_url)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)
             ON CONFLICT(id) DO UPDATE SET
                name = excluded.name,
                description = excluded.description,
                directory = excluded.directory,
                app_type = excluded.app_type,
                enabled = excluded.enabled,
                collection = excluded.collection,
                installed_at = excluded.installed_at,
                repo_owner = excluded.repo_owner,
                repo_name = excluded.repo_name,
                repo_branch = excluded.repo_branch,
                readme_url = excluded.readme_url",
            params![
                skill.id,
                skill.name,
                skill.description,
                skill.directory,
                skill.app_type,
                skill.enabled as i32,
                skill.collection,
                skill.installed_at,
                skill.repo_owner,
                skill.repo_name,
                skill.repo_branch,
                skill.readme_url,
            ],
        )
        .map_err(|e| AppError::Database(e.to_string()))?;
        Ok(())
    }

    pub fn delete_skill(&self, id: &str) -> Result<bool, AppError> {
        let conn = self.conn();
        let affected = conn
            .execute("DELETE FROM skills WHERE id = ?1", params![id])
            .map_err(|e| AppError::Database(e.to_string()))?;
        Ok(affected > 0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::Database;

    #[test]
    fn save_rejects_directory_aliases_and_identity_changes() {
        let db = Database {
            conn: std::sync::Mutex::new(rusqlite::Connection::open_in_memory().unwrap()),
        };
        db.create_tables().unwrap();
        let skill: SkillRecord = serde_json::from_value(serde_json::json!({
            "id": "first", "name": "First", "directory": "same", "appType": "claude_code"
        }))
        .unwrap();
        db.save_skill(&skill).unwrap();
        let mut alias = skill.clone();
        alias.id = "second".into();
        assert!(db.save_skill(&alias).is_err());
        let mut renamed = skill.clone();
        renamed.directory = "renamed".into();
        assert!(db.save_skill(&renamed).is_err());
        let mut traversal = skill.clone();
        traversal.directory = "../escape".into();
        assert!(db.save_skill(&traversal).is_err());
        let records = db.get_all_skills("claude_code").unwrap();
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].directory, "same");
    }
}
