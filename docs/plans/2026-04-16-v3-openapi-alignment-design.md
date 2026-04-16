# V3 OpenAPI Alignment Design

**Date:** 2026-04-16

**Goal:** Align the Rust client's public API and models with the latest `https://patch-api.conalog.com/openapi.json` `/api/v3` surface, allowing breaking changes and removing all legacy v2 helpers.

## Scope

- Keep only `/api/v3` API support in the Rust crate.
- Remove public methods that target `/api/v2`.
- Remove public methods that target v3 endpoints no longer present in the latest OpenAPI.
- Add public methods and models for v3 endpoints that exist in the latest OpenAPI but are missing in Rust.
- Update metrics deserialization so valid v3 metrics payloads do not fall into legacy-only discriminants.
- Sync `openapi/openapi-v3.json` to the latest remote v3-only subset.

## Public API Shape

- Keep account login, account fetch, token refresh, plant list/get/create, blueprint, registry records, metrics, logs, health-level, and organization member/permission APIs where they still exist in v3.
- Add OAuth helper endpoints:
  - `GET /api/v3/account/auth-methods`
  - `GET /api/v3/account/login-with-oauth2`
- Add model catalog endpoints:
  - `GET /api/v3/model-info/modules`
  - `GET /api/v3/model-info/inverters`
  - `GET /api/v3/model-info/combiners`
- Add plant indicator/stat endpoints:
  - `GET /api/v3/plants/{plant_id}/indicator/device-state`
  - `GET /api/v3/plants/{plant_id}/registry/stat`
- Remove:
  - all `/api/v2` methods
  - `upload_plant_file_v3`
  - `get_panel_seqnum_v3`

## Model Strategy

- Preserve handwritten `serde` models.
- Add typed models for newly documented v3 schemas.
- Keep tolerant deserialization where the OpenAPI is broad or partially undocumented.
- Prefer `Option<T>` for fields that are not required or appear variably in API responses.

## Metrics Strategy

- Expand `MetricsBody` to recognize current v3 combinations, especially:
  - sensor source payloads
  - interval values `15m`, `1h`, `1d`, `1M`, `1y`
- Normalize daily/aggregated intervals around current spec values instead of legacy `day`.
- Keep an `Unknown(Value)` fallback for future-safe parsing.

## Testing Strategy

- Add tests first for:
  - newly supported endpoints and query serialization
  - new model deserialization
  - updated metrics discriminants
  - local v3 spec sync expectations
- Remove tests that assert deleted legacy APIs.
- Verify with `cargo test` from `clients/rust`.
