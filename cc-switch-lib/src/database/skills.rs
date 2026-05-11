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

    pub fn save_skill(&self, skill: &SkillRecord) -> Result<(), AppError> {
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
