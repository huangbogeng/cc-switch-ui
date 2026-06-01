# Provider Save And Local Provider Fixes Design

## Scope

This design covers two concrete regressions in the current `cc-switch-ui` checkout:

1. Custom or local providers can fail to save correctly from the Providers page.
2. Local models/providers can be configured in the UI but fail to route because the saved provider metadata does not map cleanly to a backend adapter.

The goal is to repair the existing provider save path with minimal code movement and explicit regression coverage.

## Root-Cause Hypotheses

### Save path

The Providers page currently validates and serializes provider form data through `ProviderFormDialog`, `ProvidersPage`, and `buildProvider()`. Recent changes around custom-provider handling and API-key field normalization suggest that save failures are likely caused by one of these boundary issues:

- custom providers being validated differently from preset providers,
- edit flows reconstructing the wrong API-key field and then re-saving a broken payload,
- schema normalization changing persisted env keys in a way the frontend no longer expects.

### Local provider routing

Custom/local providers do not always carry a stable `meta.providerType`, so backend routing falls back to partial inference. If the inferred adapter does not match the custom provider's `apiFormat`, the provider can be saved but still fail at runtime when proxy routing tries to resolve an adapter.

## Design

### Frontend boundary

Keep the existing dialog and page structure. Restrict frontend changes to:

- preserving API-key value/field round-trip for edited providers,
- making custom providers use the same required-key validation path as preset API-key providers,
- ensuring `buildProvider()` emits the metadata needed for backend adapter selection.

### Backend boundary

Keep provider CRUD and routing structure intact. Restrict backend changes to:

- normalizing provider schema without losing the effective key field,
- preserving compatibility with legacy records that stored the key in the alternate env field,
- resolving custom providers to adapters through `meta.providerType` first and `meta.apiFormat` fallback second.

## Tests

Add regression coverage before implementation:

- frontend unit tests for `formFromProvider()` and `buildProvider()` around legacy/custom key-field behavior,
- backend unit tests for schema normalization and adapter resolution for custom providers,
- targeted verification commands for the frontend build and Rust provider tests.

## Non-Goals

- no provider UI redesign,
- no full provider-schema rewrite,
- no broader proxy or usage-accounting changes,
- no cleanup of unrelated in-progress provider work.
