use cc_switch_lib::database::Database;
use std::collections::HashSet;
use std::sync::Arc;
use tokio::sync::{broadcast, RwLock};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProviderSwitchedEvent {
    pub app_type: String,
    pub provider_id: String,
}

/// Deduplicates and applies provider switch updates after successful failover.
#[derive(Clone)]
pub struct FailoverSwitchManager {
    db: Arc<Database>,
    pending: Arc<RwLock<HashSet<String>>>,
    events: broadcast::Sender<ProviderSwitchedEvent>,
}

impl FailoverSwitchManager {
    pub fn new(db: Arc<Database>) -> Self {
        let (events, _) = broadcast::channel(32);
        Self {
            db,
            pending: Arc::new(RwLock::new(HashSet::new())),
            events,
        }
    }

    #[allow(dead_code)]
    pub fn subscribe(&self) -> broadcast::Receiver<ProviderSwitchedEvent> {
        self.events.subscribe()
    }

    /// Try switching active provider for app_type.
    /// Returns true if a switch is applied, false if skipped (already in progress).
    pub async fn try_switch(&self, app_type: &str, provider_id: &str) -> Result<bool, String> {
        let key = format!("{app_type}:{provider_id}");
        {
            let mut pending = self.pending.write().await;
            if pending.contains(&key) {
                return Ok(false);
            }
            pending.insert(key.clone());
        }

        let result = self.apply_switch(app_type, provider_id);

        {
            let mut pending = self.pending.write().await;
            pending.remove(&key);
        }

        result
    }

    fn apply_switch(&self, app_type: &str, provider_id: &str) -> Result<bool, String> {
        self.db
            .set_current_provider(provider_id, app_type)
            .map_err(|e| e.to_string())?;
        self.db
            .set_proxy_target_provider_id(provider_id)
            .map_err(|e| e.to_string())?;
        let _ = self.events.send(ProviderSwitchedEvent {
            app_type: app_type.to_string(),
            provider_id: provider_id.to_string(),
        });
        Ok(true)
    }
}
