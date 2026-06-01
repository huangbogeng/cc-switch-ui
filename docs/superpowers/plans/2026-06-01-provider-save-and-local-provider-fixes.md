# Provider Save And Local Provider Fixes Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Fix custom/local provider save failures and local-provider adapter resolution regressions without broad provider refactoring.

**Architecture:** Keep the current provider form, provider CRUD handlers, and registry structure. Add focused regression tests first, then make the smallest frontend and backend fixes needed to preserve provider schema round-trips and reliable adapter selection for custom providers.

**Tech Stack:** React 19, TypeScript, Vite, Rust, Axum, serde_json, cargo test

---

### Task 1: Lock provider form round-trip behavior

**Files:**
- Modify: `cc-switch-ui/src/components/providers/providerForm.ts`
- Create: `cc-switch-ui/src/components/providers/providerForm.test.ts`

- [ ] Step 1: Write failing frontend tests for legacy key-field round-trip and custom provider metadata.
- [ ] Step 2: Run the focused frontend test command and confirm the failures point at current provider form behavior.
- [ ] Step 3: Make the minimal `providerForm.ts` changes needed for those tests.
- [ ] Step 4: Re-run the focused frontend test command and confirm pass.

### Task 2: Lock backend schema normalization and adapter fallback

**Files:**
- Modify: `cc-switch-lib/src/providers/schema.rs`
- Modify: `cc-switch-lib/src/providers/registry.rs`

- [ ] Step 1: Write failing Rust tests for custom-provider adapter selection and legacy API-key compatibility.
- [ ] Step 2: Run focused cargo tests and confirm the failures match the provider regressions.
- [ ] Step 3: Make the minimal Rust changes needed for schema normalization and registry fallback.
- [ ] Step 4: Re-run the focused cargo tests and confirm pass.

### Task 3: Verify the save path entry point

**Files:**
- Modify: `cc-switch-ui/src/pages/ProvidersPage.tsx`
- Modify: `cc-switch-server/src/handlers/providers.rs`

- [ ] Step 1: Add or adjust only the guard logic required for custom/local provider save requests.
- [ ] Step 2: Re-run targeted frontend and Rust verification.
- [ ] Step 3: Run `npm run build`.
- [ ] Step 4: Run the focused cargo provider test subset.
