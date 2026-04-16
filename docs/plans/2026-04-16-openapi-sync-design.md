# OpenAPI Sync Workflow Design

**Date:** 2026-04-16

## Goal

Keep `openapi/openapi-v3.json` aligned with `https://patch-api.conalog.com/openapi.json` automatically, with a weekly schedule and a manual trigger, and commit changes directly to `main`.

## Current Context

- The repository already has a sync script at `scripts/update-openapi-v3.sh`.
- The script fetches the upstream OpenAPI document, filters `/api/v3/` paths, rewrites the API title, and writes the result to `openapi/openapi-v3.json`.
- There is no existing `.github/workflows` directory, so the automation can be introduced without conflicting with current CI conventions.

## Recommended Approach

Use a GitHub Actions workflow that runs on:

- `schedule` once per week
- `workflow_dispatch` for manual runs

The workflow should:

1. Check out `main`
2. Install `jq`
3. Run `scripts/update-openapi-v3.sh`
4. Detect whether `openapi/openapi-v3.json` changed
5. Commit and push to `main` only when there is a real diff

## Alternatives Considered

### 1. GitHub Actions direct-to-main sync

This is the recommended approach.

- Matches the requested weekly plus manual trigger behavior
- Reuses the existing repository script
- Keeps operational logic close to the repository
- Avoids external scheduling infrastructure

### 2. GitHub Actions with PR creation

This is safer for review-heavy repositories, but it does not match the request to commit directly to `main` and adds operational overhead.

### 3. External scheduler or local cron

This would work technically, but it increases maintenance cost and moves a repository-owned concern outside GitHub.

## Behavior Details

- The scheduled run should happen weekly at a fixed UTC time.
- Manual runs should use the standard Actions "Run workflow" entry.
- The workflow should request `contents: write` so the default `GITHUB_TOKEN` can push to `main`.
- Concurrency should prevent overlapping sync jobs from racing each other.
- If there is no diff after running the sync script, the workflow exits without creating a commit.

## Error Handling

- `curl -fsSL` in the existing script already causes the job to fail on fetch errors.
- `set -euo pipefail` in the script ensures bad fetches or bad `jq` processing fail the job.
- A failed workflow should leave the repository unchanged.

## Verification

Local verification should cover:

- Running `scripts/update-openapi-v3.sh`
- Confirming the workflow file is syntactically valid YAML
- Reviewing the generated diff to ensure only intended automation files and docs changed

## Files To Change

- Create: `.github/workflows/sync-openapi-v3.yml`
- Modify: `README.md`
- Create: `docs/plans/2026-04-16-openapi-sync.md`

