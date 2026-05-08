use super::circuit_breaker::CircuitBreaker;
use cc_switch_lib::database::Provider;
use std::collections::HashMap;
use std::time::Duration;

pub struct ProviderRouter {
    auto_failover_enabled: bool,
    breakers: HashMap<String, CircuitBreaker>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SelectProvidersError {
    AllCandidatesCircuitOpen,
}

impl ProviderRouter {
    pub fn new(auto_failover_enabled: bool) -> Self {
        Self {
            auto_failover_enabled,
            breakers: HashMap::new(),
        }
    }

    pub fn set_auto_failover_enabled(&mut self, enabled: bool) {
        self.auto_failover_enabled = enabled;
    }

    pub fn select_providers(
        &mut self,
        app_type: &str,
        current_provider: &Provider,
        providers: &HashMap<String, Provider>,
    ) -> Result<Vec<Provider>, SelectProvidersError> {
        let mut candidates = if self.auto_failover_enabled {
            let queue = queue_order(providers);
            if queue.is_empty() {
                vec![current_provider.clone()]
            } else {
                queue
            }
        } else {
            vec![current_provider.clone()]
        };

        candidates.retain(|provider| {
            let key = breaker_key(app_type, &provider.id);
            let breaker = self.breakers.entry(key).or_insert_with(default_breaker);
            breaker.allow_request()
        });

        if candidates.is_empty() {
            // When queue mode is enabled with configured candidates,
            // do not bypass open breakers by force-falling back.
            if self.auto_failover_enabled && !queue_order(providers).is_empty() {
                return Err(SelectProvidersError::AllCandidatesCircuitOpen);
            }
            return Ok(vec![current_provider.clone()]);
        }

        Ok(candidates)
    }

    pub fn record_success(&mut self, app_type: &str, provider_id: &str) {
        let key = breaker_key(app_type, provider_id);
        if let Some(breaker) = self.breakers.get_mut(&key) {
            breaker.record_success();
        }
        // TODO: integrate provider health persistence update when provider health schema is available.
    }

    pub fn record_failure(&mut self, app_type: &str, provider_id: &str) {
        let key = breaker_key(app_type, provider_id);
        let breaker = self.breakers.entry(key).or_insert_with(default_breaker);
        breaker.record_failure();
        // TODO: integrate provider health persistence update when provider health schema is available.
    }
}

fn default_breaker() -> CircuitBreaker {
    CircuitBreaker::new(3, Duration::from_secs(30))
}

fn queue_order(providers: &HashMap<String, Provider>) -> Vec<Provider> {
    let mut queue: Vec<Provider> = providers
        .values()
        .filter(|provider| provider.in_failover_queue)
        .cloned()
        .collect();
    queue.sort_by_key(|provider| (provider.sort_index.unwrap_or(i32::MAX), provider.id.clone()));
    queue
}

fn breaker_key(app_type: &str, provider_id: &str) -> String {
    format!("{app_type}:{provider_id}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn provider(id: &str, in_failover_queue: bool, sort_index: Option<i32>) -> Provider {
        Provider {
            id: id.to_string(),
            name: id.to_string(),
            settings_config: json!({}),
            website_url: None,
            category: None,
            created_at: None,
            sort_index,
            notes: None,
            icon: None,
            icon_color: None,
            meta: json!({}),
            in_failover_queue,
        }
    }

    #[test]
    fn failover_disabled_returns_only_current_provider() {
        let current = provider("current", true, Some(2));
        let mut providers = HashMap::new();
        providers.insert("current".to_string(), current.clone());
        providers.insert("queued-1".to_string(), provider("queued-1", true, Some(1)));

        let mut router = ProviderRouter::new(false);
        let result = router
            .select_providers("claude", &current, &providers)
            .unwrap();

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].id, "current");
    }

    #[test]
    fn failover_enabled_returns_queue_order() {
        let current = provider("current", true, Some(2));
        let mut providers = HashMap::new();
        providers.insert("current".to_string(), current.clone());
        providers.insert("queued-2".to_string(), provider("queued-2", true, Some(3)));
        providers.insert("queued-1".to_string(), provider("queued-1", true, Some(1)));
        providers.insert(
            "not-queued".to_string(),
            provider("not-queued", false, Some(0)),
        );

        let mut router = ProviderRouter::new(true);
        let result = router
            .select_providers("claude", &current, &providers)
            .unwrap();
        let ids: Vec<_> = result.into_iter().map(|provider| provider.id).collect();

        assert_eq!(ids, vec!["queued-1", "current", "queued-2"]);
    }

    #[test]
    fn failover_enabled_falls_back_to_current_when_queue_empty() {
        let current = provider("current", false, Some(5));
        let mut providers = HashMap::new();
        providers.insert("current".to_string(), current.clone());
        providers.insert("other".to_string(), provider("other", false, Some(1)));

        let mut router = ProviderRouter::new(true);
        let result = router
            .select_providers("claude", &current, &providers)
            .unwrap();

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].id, "current");
    }

    #[test]
    fn open_breaker_filters_provider_from_candidates() {
        let current = provider("current", true, Some(1));
        let fallback = provider("fallback", true, Some(2));
        let mut providers = HashMap::new();
        providers.insert("current".to_string(), current.clone());
        providers.insert("fallback".to_string(), fallback.clone());

        let mut router = ProviderRouter::new(true);
        router.record_failure("claude", "current");
        router.record_failure("claude", "current");
        router.record_failure("claude", "current");

        let result = router
            .select_providers("claude", &current, &providers)
            .unwrap();
        let ids: Vec<_> = result.into_iter().map(|provider| provider.id).collect();
        assert_eq!(ids, vec!["fallback"]);
    }

    #[test]
    fn breaker_key_is_scoped_by_app_type() {
        let current = provider("shared", true, Some(1));
        let mut providers = HashMap::new();
        providers.insert("shared".to_string(), current.clone());

        let mut router = ProviderRouter::new(true);
        router.record_failure("claude", "shared");
        router.record_failure("claude", "shared");
        router.record_failure("claude", "shared");

        let claude_result = router.select_providers("claude", &current, &providers);
        assert!(matches!(
            claude_result,
            Err(SelectProvidersError::AllCandidatesCircuitOpen)
        ));

        let codex_result = router
            .select_providers("codex", &current, &providers)
            .unwrap();
        assert_eq!(codex_result.len(), 1);
        assert_eq!(codex_result[0].id, "shared");
    }

    #[test]
    fn failover_queue_does_not_bypass_open_breakers() {
        let current = provider("current", true, Some(1));
        let fallback = provider("fallback", true, Some(2));
        let mut providers = HashMap::new();
        providers.insert("current".to_string(), current.clone());
        providers.insert("fallback".to_string(), fallback.clone());

        let mut router = ProviderRouter::new(true);
        for id in ["current", "fallback"] {
            router.record_failure("claude", id);
            router.record_failure("claude", id);
            router.record_failure("claude", id);
        }

        let result = router.select_providers("claude", &current, &providers);
        assert!(matches!(
            result,
            Err(SelectProvidersError::AllCandidatesCircuitOpen)
        ));
    }
}
