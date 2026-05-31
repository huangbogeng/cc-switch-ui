//! Provider CRUD operations.

use crate::database::types::Provider;
use crate::error::AppError;
use rusqlite::params;
use serde_json::json;
use std::collections::HashMap;

/// Seed data for a single provider preset.
struct SeedProvider {
    id: &'static str,
    name: &'static str,
    website_url: &'static str,
    icon: &'static str,
    icon_color: &'static str,
    description: &'static str,
    /// settings_config as JSON string
    settings_json: &'static str,
    /// meta as JSON string
    meta_json: &'static str,
}

/// Built-in provider presets seeded on first run.
/// Mirrors the frontend `providerPresets.ts` definitions.
const SEED_PROVIDERS: &[SeedProvider] = &[
    SeedProvider {
        id: "deepseek",
        name: "DeepSeek",
        website_url: "https://platform.deepseek.com",
        icon: "deepseek",
        icon_color: "#1E88E5",
        description: "DeepSeek V4 模型",
        settings_json: r#"{"env":{"ANTHROPIC_BASE_URL":"https://api.deepseek.com/anthropic","ANTHROPIC_AUTH_TOKEN":"","ANTHROPIC_MODEL":"deepseek-v4-pro","ANTHROPIC_DEFAULT_HAIKU_MODEL":"deepseek-v4-flash","ANTHROPIC_DEFAULT_SONNET_MODEL":"deepseek-v4-pro","ANTHROPIC_DEFAULT_OPUS_MODEL":"deepseek-v4-pro","CLAUDE_CODE_SUBAGENT_MODEL":"deepseek-v4-flash","CLAUDE_CODE_EFFORT_LEVEL":"max"}}"#,
        meta_json: r##"{"icon":"deepseek","iconColor":"#1E88E5","apiFormat":"anthropic","official":true}"##,
    },
    SeedProvider {
        id: "minimax",
        name: "MiniMax",
        website_url: "https://platform.minimaxi.com",
        icon: "minimax",
        icon_color: "#FF6B6B",
        description: "MiniMax M2.7 模型",
        settings_json: r#"{"env":{"ANTHROPIC_BASE_URL":"https://api.minimaxi.com/v1","ANTHROPIC_AUTH_TOKEN":"","API_TIMEOUT_MS":"3000000","CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC":"1","ANTHROPIC_MODEL":"MiniMax-M2.7","ANTHROPIC_DEFAULT_SONNET_MODEL":"MiniMax-M2.7","ANTHROPIC_DEFAULT_OPUS_MODEL":"MiniMax-M2.7","ANTHROPIC_DEFAULT_HAIKU_MODEL":"MiniMax-M2.7"}}"#,
        meta_json: r##"{"icon":"minimax","iconColor":"#FF6B6B","apiFormat":"openai_chat","official":true}"##,
    },
    SeedProvider {
        id: "siliconflow",
        name: "SiliconFlow",
        website_url: "https://siliconflow.cn",
        icon: "siliconflow",
        icon_color: "#6E29F6",
        description: "SiliconFlow 聚合平台",
        settings_json: r#"{"env":{"ANTHROPIC_BASE_URL":"https://api.siliconflow.cn","ANTHROPIC_AUTH_TOKEN":"","ANTHROPIC_MODEL":"Pro/MiniMaxAI/MiniMax-M2.7","ANTHROPIC_DEFAULT_HAIKU_MODEL":"Pro/MiniMaxAI/MiniMax-M2.7","ANTHROPIC_DEFAULT_SONNET_MODEL":"Pro/MiniMaxAI/MiniMax-M2.7","ANTHROPIC_DEFAULT_OPUS_MODEL":"Pro/MiniMaxAI/MiniMax-M2.7"}}"#,
        meta_json: r##"{"icon":"siliconflow","iconColor":"#6E29F6","apiFormat":"openai_chat","official":true}"##,
    },
    SeedProvider {
        id: "openrouter",
        name: "OpenRouter",
        website_url: "https://openrouter.ai",
        icon: "openrouter",
        icon_color: "#7C3AED",
        description: "Access 100+ AI models via unified API",
        settings_json: r#"{"env":{"ANTHROPIC_BASE_URL":"https://openrouter.ai/api","ANTHROPIC_AUTH_TOKEN":"","ANTHROPIC_MODEL":"anthropic/claude-sonnet-4.6","ANTHROPIC_DEFAULT_HAIKU_MODEL":"anthropic/claude-haiku-4.5","ANTHROPIC_DEFAULT_SONNET_MODEL":"anthropic/claude-sonnet-4.6","ANTHROPIC_DEFAULT_OPUS_MODEL":"anthropic/claude-opus-4.7"}}"#,
        meta_json: r##"{"icon":"openrouter","iconColor":"#7C3AED","apiFormat":"openai_chat","official":true}"##,
    },
    SeedProvider {
        id: "gemini-native",
        name: "Gemini Native",
        website_url: "https://ai.google.dev",
        icon: "google",
        icon_color: "#4285F4",
        description: "Google Gemini Native API (gemini_native format)",
        settings_json: r#"{"env":{"ANTHROPIC_BASE_URL":"https://generativelanguage.googleapis.com","ANTHROPIC_API_KEY":"","ANTHROPIC_MODEL":"gemini-3.1-pro","ANTHROPIC_DEFAULT_HAIKU_MODEL":"gemini-3-flash","ANTHROPIC_DEFAULT_SONNET_MODEL":"gemini-3.1-pro","ANTHROPIC_DEFAULT_OPUS_MODEL":"gemini-3.1-pro"}}"#,
        meta_json: r##"{"icon":"google","iconColor":"#4285F4","apiFormat":"gemini_native","apiKeyField":"ANTHROPIC_API_KEY","official":true}"##,
    },
];

impl super::Database {
    pub fn list_providers(&self, app_type: &str) -> Result<HashMap<String, Provider>, AppError> {
        let conn = self.conn();
        let mut stmt = conn.prepare(
            "SELECT id, name, settings_config, website_url, category, created_at,
                    sort_index, notes, icon, icon_color, meta, in_failover_queue
             FROM providers WHERE app_type = ?1 ORDER BY sort_index ASC NULLS LAST",
        )?;

        let rows = stmt.query_map(params![app_type], |row| {
            let settings_config_str: String = row.get(2)?;
            let meta_str: String = row.get(10)?;
            Ok(Provider {
                id: row.get(0)?,
                name: row.get(1)?,
                settings_config: serde_json::from_str(&settings_config_str)
                    .unwrap_or(serde_json::Value::Null),
                website_url: row.get(3)?,
                category: row.get(4)?,
                created_at: row.get(5)?,
                sort_index: row.get(6)?,
                notes: row.get(7)?,
                icon: row.get(8)?,
                icon_color: row.get(9)?,
                meta: serde_json::from_str(&meta_str)
                    .unwrap_or(serde_json::Value::Object(Default::default())),
                in_failover_queue: row.get::<_, i32>(11)? != 0,
            })
        })?;

        let mut map = HashMap::new();
        for provider in rows {
            let p = provider.map_err(|e| AppError::Database(e.to_string()))?;
            map.insert(p.id.clone(), p);
        }
        Ok(map)
    }

    pub fn get_provider(&self, id: &str, app_type: &str) -> Result<Option<Provider>, AppError> {
        let conn = self.conn();
        let mut stmt = conn.prepare(
            "SELECT id, name, settings_config, website_url, category, created_at,
                    sort_index, notes, icon, icon_color, meta, in_failover_queue
             FROM providers WHERE id = ?1 AND app_type = ?2",
        )?;

        let mut rows = stmt.query_map(params![id, app_type], |row| {
            let settings_config_str: String = row.get(2)?;
            let meta_str: String = row.get(10)?;
            Ok(Provider {
                id: row.get(0)?,
                name: row.get(1)?,
                settings_config: serde_json::from_str(&settings_config_str)
                    .unwrap_or(serde_json::Value::Null),
                website_url: row.get(3)?,
                category: row.get(4)?,
                created_at: row.get(5)?,
                sort_index: row.get(6)?,
                notes: row.get(7)?,
                icon: row.get(8)?,
                icon_color: row.get(9)?,
                meta: serde_json::from_str(&meta_str)
                    .unwrap_or(serde_json::Value::Object(Default::default())),
                in_failover_queue: row.get::<_, i32>(11)? != 0,
            })
        })?;

        match rows.next() {
            Some(r) => Ok(Some(r.map_err(|e| AppError::Database(e.to_string()))?)),
            None => Ok(None),
        }
    }

    pub fn save_provider(&self, app_type: &str, provider: &Provider) -> Result<(), AppError> {
        let conn = self.conn();
        let settings_config_str = serde_json::to_string(&provider.settings_config)
            .map_err(|e| AppError::JsonSerialize { source: e })?;
        let meta_str = serde_json::to_string(&provider.meta).unwrap_or_else(|_| "{}".to_string());

        conn.execute(
            "INSERT INTO providers (id, app_type, name, settings_config, website_url, category,
                created_at, sort_index, notes, icon, icon_color, meta, in_failover_queue)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)
             ON CONFLICT(id, app_type) DO UPDATE SET
                name = excluded.name,
                settings_config = excluded.settings_config,
                website_url = excluded.website_url,
                category = excluded.category,
                sort_index = excluded.sort_index,
                notes = excluded.notes,
                icon = excluded.icon,
                icon_color = excluded.icon_color,
                meta = excluded.meta,
                in_failover_queue = excluded.in_failover_queue",
            params![
                provider.id,
                app_type,
                provider.name,
                settings_config_str,
                provider.website_url,
                provider.category,
                provider.created_at,
                provider.sort_index,
                provider.notes,
                provider.icon,
                provider.icon_color,
                meta_str,
                provider.in_failover_queue as i32,
            ],
        )
        .map_err(|e| AppError::Database(e.to_string()))?;
        Ok(())
    }

    pub fn delete_provider(&self, id: &str, app_type: &str) -> Result<(), AppError> {
        let conn = self.conn();
        conn.execute(
            "DELETE FROM providers WHERE id = ?1 AND app_type = ?2",
            params![id, app_type],
        )
        .map_err(|e| AppError::Database(e.to_string()))?;
        Ok(())
    }

    pub fn set_current_provider(&self, id: &str, app_type: &str) -> Result<(), AppError> {
        let conn = self.conn();
        conn.execute(
            "UPDATE providers SET is_current = 0 WHERE app_type = ?1",
            params![app_type],
        )
        .map_err(|e| AppError::Database(e.to_string()))?;
        conn.execute(
            "UPDATE providers SET is_current = 1 WHERE id = ?1 AND app_type = ?2",
            params![id, app_type],
        )
        .map_err(|e| AppError::Database(e.to_string()))?;
        Ok(())
    }

    pub fn get_current_provider_id(&self, app_type: &str) -> Result<Option<String>, AppError> {
        let conn = self.conn();
        let mut stmt =
            conn.prepare("SELECT id FROM providers WHERE app_type = ?1 AND is_current = 1")?;
        let mut rows = stmt.query(params![app_type])?;
        match rows.next()? {
            Some(row) => Ok(Some(row.get(0)?)),
            None => Ok(None),
        }
    }

    /// Check whether the providers table is empty for a given `app_type`.
    pub fn is_providers_empty(&self, app_type: &str) -> Result<bool, AppError> {
        let conn = self.conn();
        let count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM providers WHERE app_type = ?1",
            params![app_type],
            |row| row.get(0),
        )?;
        Ok(count == 0)
    }

    /// Seed built-in provider presets on first run.
    ///
    /// Safe to call on every startup — it checks `is_providers_empty` internally
    /// and skips seeding if the table already has entries.
    ///
    /// Returns the number of providers seeded (0 if already populated).
    pub fn seed_default_providers(&self, app_type: &str) -> Result<usize, AppError> {
        if !self.is_providers_empty(app_type)? {
            log::info!(
                "[Seed] providers table already populated for app_type={}",
                app_type
            );
            return Ok(0);
        }

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);
        let mut count = 0_usize;

        for seed in SEED_PROVIDERS {
            let settings_config: serde_json::Value =
                serde_json::from_str(seed.settings_json).unwrap_or(json!({}));
            let meta: serde_json::Value =
                serde_json::from_str(seed.meta_json).unwrap_or(json!({}));

            let provider = Provider {
                id: seed.id.to_string(),
                name: seed.name.to_string(),
                settings_config,
                website_url: Some(seed.website_url.to_string()),
                category: Some("official".to_string()),
                created_at: Some(now),
                sort_index: Some(count as i32),
                notes: Some(seed.description.to_string()),
                icon: Some(seed.icon.to_string()),
                icon_color: Some(seed.icon_color.to_string()),
                meta,
                in_failover_queue: false,
            };

            self.save_provider(app_type, &provider)?;
            count += 1;
        }

        log::info!(
            "[Seed] seeded {} default providers for app_type={}",
            count,
            app_type
        );
        Ok(count)
    }

    /// Import a provider from the current live Claude settings file.
    ///
    /// Reads `~/.claude/settings.json` and creates a "default" provider entry
    /// when the user already has API keys configured. This gives existing users
    /// a provider card on first launch.
    ///
    /// Returns `Ok(true)` when a provider was imported, `Ok(false)` when
    /// there are no useful credentials in the live file.
    pub fn import_provider_from_live(&self, app_type: &str) -> Result<bool, AppError> {
        let path = crate::config::get_claude_settings_path();
        if !path.exists() {
            log::info!("[Seed] no live settings file at {}", path.display());
            return Ok(false);
        }

        let raw = match std::fs::read_to_string(&path) {
            Ok(v) => v,
            Err(e) => {
                log::warn!("[Seed] failed to read live settings: {}", e);
                return Ok(false);
            }
        };
        let live: serde_json::Value =
            serde_json::from_str(&raw).unwrap_or(serde_json::Value::Object(Default::default()));
        let env = live
            .get("env")
            .and_then(|v| v.as_object())
            .cloned()
            .unwrap_or_default();

        // Detect known API key vars
        let has_anthropic =
            env.contains_key("ANTHROPIC_AUTH_TOKEN") || env.contains_key("ANTHROPIC_API_KEY");
        let _has_base_url = env.contains_key("ANTHROPIC_BASE_URL");

        if !has_anthropic {
            log::info!(
                "[Seed] live settings has no Anthropic auth token — skipping live import"
            );
            return Ok(false);
        }

        // Derive a sensible name from the base URL
        let name = if let Some(base_url) = env
            .get("ANTHROPIC_BASE_URL")
            .and_then(|v| v.as_str())
        {
            if base_url.contains("deepseek") {
                "DeepSeek (imported)"
            } else if base_url.contains("minimaxi") {
                "MiniMax (imported)"
            } else if base_url.contains("openrouter") {
                "OpenRouter (imported)"
            } else if base_url.contains("siliconflow") {
                "SiliconFlow (imported)"
            } else if base_url.contains("generativelanguage") {
                "Gemini (imported)"
            } else if base_url.contains("openai") || base_url.contains("chatgpt") {
                "OpenAI (imported)"
            } else {
                "Default (imported)"
            }
        } else {
            "Default (imported)"
        };

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);

        let settings = crate::live::sanitize_claude_settings_for_live(&live);
        let provider = Provider {
            id: "default".to_string(),
            name: name.to_string(),
            settings_config: settings,
            website_url: None,
            category: Some("imported".to_string()),
            created_at: Some(now),
            sort_index: Some(-1), // imported provider first
            notes: Some("Auto-imported from existing Claude Code settings".to_string()),
            icon: None,
            icon_color: None,
            meta: json!({}),
            in_failover_queue: false,
        };

        // Check if provider already exists before inserting
        if self.get_provider("default", app_type)?.is_some() {
            log::info!(
                "[Seed] 'default' provider already exists — skipping live import"
            );
            return Ok(false);
        }

        self.save_provider(app_type, &provider)?;
        // Set as current so the user sees a selected provider on first launch
        let _ = self.set_current_provider("default", app_type);
        log::info!(
            "[Seed] imported provider '{}' from live settings",
            name
        );
        Ok(true)
    }

    /// One-shot startup initialization: seed built-in provider presets.
    ///
    /// Call this once during server startup. Idempotent — repeated calls
    /// are no-ops when providers already exist.
    pub fn initialize_providers_on_startup(&self, app_type: &str) {
        match self.seed_default_providers(app_type) {
            Ok(0) => {}
            Ok(n) => log::info!("[Init] seeded {} default providers", n),
            Err(e) => log::warn!("[Init] seed providers failed: {}", e),
        }
    }
}

#[cfg(test)]
mod tests {
    use rusqlite::Connection;

    fn in_memory_db() -> super::super::Database {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS providers (
                id TEXT NOT NULL,
                app_type TEXT NOT NULL,
                name TEXT NOT NULL,
                settings_config TEXT NOT NULL,
                website_url TEXT,
                category TEXT,
                created_at INTEGER,
                sort_index INTEGER,
                notes TEXT,
                icon TEXT,
                icon_color TEXT,
                meta TEXT NOT NULL DEFAULT '{}',
                is_current INTEGER NOT NULL DEFAULT 0,
                in_failover_queue INTEGER NOT NULL DEFAULT 0,
                PRIMARY KEY (id, app_type)
            );",
        )
        .unwrap();
        super::super::Database {
            conn: std::sync::Mutex::new(conn),
        }
    }

    #[test]
    fn seeds_providers_on_empty_table() {
        let db = in_memory_db();
        let app_type = "claude_code";

        assert!(db.is_providers_empty(app_type).unwrap());
        let count = db.seed_default_providers(app_type).unwrap();
        assert_eq!(count, 5, "should seed 5 providers");

        let providers = db.list_providers(app_type).unwrap();
        assert_eq!(providers.len(), 5);
        assert!(providers.contains_key("deepseek"));
        assert!(providers.contains_key("minimax"));
        assert!(providers.contains_key("siliconflow"));
        assert!(providers.contains_key("openrouter"));
        assert!(providers.contains_key("gemini-native"));

        // Second call is a no-op
        assert!(!db.is_providers_empty(app_type).unwrap());
        let count2 = db.seed_default_providers(app_type).unwrap();
        assert_eq!(count2, 0);
    }

    #[test]
    fn seeds_providers_are_idempotent() {
        let db = in_memory_db();
        let app_type = "claude_code";

        let first = db.seed_default_providers(app_type).unwrap();
        assert_eq!(first, 5, "first call seeds 5 providers");

        let second = db.seed_default_providers(app_type).unwrap();
        assert_eq!(second, 0, "second call is no-op");

        let providers = db.list_providers(app_type).unwrap();
        assert_eq!(providers.len(), 5, "still exactly 5 providers after two calls");
    }
}
