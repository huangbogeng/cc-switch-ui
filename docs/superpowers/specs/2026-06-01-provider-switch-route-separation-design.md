# Provider Switch And Route Separation Design

## Goal

Rewrite provider switching and route control so the current `cc-switch-ui` checkout matches the upstream `cc-switch` business semantics:

- selecting a provider does not implicitly change the route target,
- selecting a provider does not implicitly restart the route,
- route start/stop owns proxy takeover of live config,
- route target changes are explicit and independent.

## Current Problem

The current implementation couples three distinct actions:

1. selecting the active provider,
2. choosing the provider used by the local route,
3. starting or restarting the local route service.

Because `handleSwitch()` also updates proxy target and restarts the proxy when running, the UI exposes one action while the system performs several side effects. This makes provider state, route state, and live-config state hard to reason about.

## Target Model

### State

Keep these states distinct:

- `currentProviderId`: the provider selected for direct live-config use,
- `proxyRunning`: whether the local route service is running,
- `proxyTargetProviderId`: the provider that the running route forwards to,
- `takeoverActive`: whether live config currently points at the local route.

### Actions

- `Select Provider`
  - updates current provider only,
  - writes direct live config only when route takeover is not active,
  - never changes route target,
  - never restarts the route.
- `Set Route Target`
  - updates route target only,
  - if route is running, hot-switch the running route target,
  - never changes current provider.
- `Start Route`
  - starts proxy takeover using the explicit route target if present,
  - otherwise falls back to current provider.
- `Stop Route`
  - stops proxy takeover and restores the selected provider's direct live config.

## UI Changes

Each provider card should show three independent concepts:

- `Selected` when it is the current provider,
- `Route Target` when it is the chosen route target,
- `Route Running` when the route is active and currently targeting that provider.

Actions become:

- `Select` / `Selected`
- `Use For Route` / `Route Target`
- `Start Route` / `Stop Route`

## Backend Changes

- `switch_provider` in `handlers/providers.rs`
  - remove route-target mutation,
  - keep current-provider update,
  - if takeover is active, do not rewrite route target or restart route.
- `proxy_set_target` in `proxy/handlers.rs`
  - become the single route-target write path,
  - hot-switch active route target when proxy is running.
- `proxy_start` and `proxy_stop`
  - remain the only path that takes over or restores live config.

## Verification

- switching provider leaves route target unchanged,
- setting route target leaves current provider unchanged,
- setting route target while route is running hot-switches without stop/start,
- starting route uses explicit route target or falls back to current provider,
- stopping route restores selected provider live config,
- frontend build and targeted Rust tests pass.
