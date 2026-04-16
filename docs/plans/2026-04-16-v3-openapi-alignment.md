# V3 OpenAPI Alignment Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Make the Rust crate a breaking-change, v3-only client aligned to the latest PATCH Plant Data API OpenAPI.

**Architecture:** Refactor the handwritten Rust client so its public methods map to the current v3 path set, update handwritten `serde` models to match current schemas, and keep robust low-level request/response handling intact. Treat legacy v2 and removed v3 helpers as deleted API surface rather than compatibility shims.

**Tech Stack:** Rust, reqwest, serde, serde_json, tokio, jq, curl

---

### Task 1: Document and lock the target API surface

**Files:**
- Create: `docs/plans/2026-04-16-v3-openapi-alignment-design.md`
- Create: `docs/plans/2026-04-16-v3-openapi-alignment.md`

**Step 1: Save the approved design**

Record the v3-only, breaking-change scope and the endpoints to add/remove.

**Step 2: Save the implementation plan**

Capture the exact files, tests, and verification commands for the remaining work.

### Task 2: Write failing tests for the new v3-only API

**Files:**
- Modify: `clients/rust/src/client.rs`
- Modify: `clients/rust/tests/models.rs`

**Step 1: Write failing tests for new client methods**

Add request-path tests for:
- `auth-methods`
- `login-with-oauth2`
- `model-info/modules`
- `model-info/inverters`
- `model-info/combiners`
- `indicator/device-state`
- `registry/stat`

**Step 2: Write failing tests for updated metrics discriminants**

Add model tests that expect:
- `1d` to deserialize into aggregated variants
- sensor payloads to deserialize into a typed sensor variant

**Step 3: Run targeted tests to verify they fail**

Run: `cargo test -p patch-client new_ -- --nocapture`

Expected: compile or assertion failures showing the missing methods/types/variants.

### Task 3: Implement the new client methods and remove legacy public methods

**Files:**
- Modify: `clients/rust/src/client.rs`
- Modify: `clients/rust/src/lib.rs`

**Step 1: Add the missing v3 methods**

Implement the new public methods using existing request helpers and auth behavior.

**Step 2: Remove deleted public methods**

Delete all v2 methods and removed v3 methods from the public API surface.

**Step 3: Run focused client tests**

Run: `cargo test -p patch-client client::tests:: -- --nocapture`

Expected: the new client tests pass.

### Task 4: Update models to match current v3 schemas

**Files:**
- Modify: `clients/rust/src/model.rs`
- Modify: `clients/rust/tests/models.rs`

**Step 1: Add missing schema-backed types**

Implement typed models for OAuth provider metadata, model catalog items, registry stat payloads, and sensor metrics.

**Step 2: Update metrics discrimination**

Teach `MetricsBody` to recognize current interval and source/unit combinations.

**Step 3: Run focused model tests**

Run: `cargo test -p patch-client --test models -- --nocapture`

Expected: the new schema and metrics tests pass.

### Task 5: Sync the local v3 OpenAPI snapshot

**Files:**
- Modify: `openapi/openapi-v3.json`

**Step 1: Refresh the local v3 spec from remote**

Use the repo sync script or equivalent filtered output from the remote OpenAPI.

**Step 2: Verify path parity**

Run: `comm -3 <(jq -r '.paths|keys[]' openapi/openapi-v3.json | sort) <(curl -fsSL https://patch-api.conalog.com/openapi.json | jq -r '.paths|keys[]|select(startswith("/api/v3/"))' | sort)`

Expected: no output.

### Task 6: Run full verification

**Files:**
- Modify: `clients/rust/src/client.rs`
- Modify: `clients/rust/src/model.rs`
- Modify: `clients/rust/tests/models.rs`
- Modify: `openapi/openapi-v3.json`

**Step 1: Run Rust test suite**

Run: `cargo test -p patch-client`

Expected: all tests pass.

**Step 2: Spot-check the public v3 surface**

Run: `rg -n 'pub async fn|pub fn' clients/rust/src/client.rs`

Expected: only current v3 methods remain public.
