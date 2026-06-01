# Provider Switch And Route Separation Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Separate provider selection, route target selection, and route lifecycle so the UI and backend match upstream `cc-switch` business semantics.

**Architecture:** Keep the existing provider page and proxy APIs, but rewrite their responsibilities. Provider switching updates direct selection only, proxy target mutation becomes explicit, and route start/stop remains the only live-config takeover path.

**Tech Stack:** React 19, TypeScript, Rust, Axum, cargo test, Vite

---

### Task 1: Separate backend responsibilities

**Files:**
- Modify: `cc-switch-server/src/handlers/providers.rs`
- Modify: `cc-switch-server/src/proxy/handlers.rs`
- Modify: `cc-switch-server/src/proxy/server.rs`

- [ ] Step 1: Write or adjust focused Rust tests for switch-vs-route-target semantics where practical.
- [ ] Step 2: Remove route-target mutation from provider switching.
- [ ] Step 3: Make proxy target updates hot-switch when route is running.
- [ ] Step 4: Run focused cargo tests.

### Task 2: Separate frontend actions

**Files:**
- Modify: `cc-switch-ui/src/pages/ProvidersPage.tsx`
- Modify: `cc-switch-ui/src/components/providers/ProviderCard.tsx`
- Modify: `cc-switch-ui/src/lib/provider.ts`

- [ ] Step 1: Rewrite page handlers so select, set-route-target, and route start/stop are independent.
- [ ] Step 2: Update provider card badges and actions to reflect independent states.
- [ ] Step 3: Keep existing edit/delete/save behavior intact.
- [ ] Step 4: Run frontend build.

### Task 3: Verify end-to-end semantics

**Files:**
- Modify: `docs/superpowers/specs/2026-06-01-provider-switch-route-separation-design.md`
- Modify: `docs/superpowers/plans/2026-06-01-provider-switch-route-separation.md`

- [ ] Step 1: Run targeted Rust verification for provider/proxy paths.
- [ ] Step 2: Run `npm run build`.
- [ ] Step 3: Review final diff for any remaining implicit route coupling.
