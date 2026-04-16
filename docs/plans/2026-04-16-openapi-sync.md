# OpenAPI Sync Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add a GitHub Actions workflow that refreshes the local v3 OpenAPI snapshot weekly and on manual trigger, then commits real changes directly to `main`.

**Architecture:** Reuse the existing shell sync script as the single source of truth for generating `openapi/openapi-v3.json`. Wrap it in a GitHub Actions workflow that checks for diffs, commits only when needed, and documents the automation in the repository README.

**Tech Stack:** GitHub Actions, Bash, `curl`, `jq`, Git

---

### Task 1: Add the workflow

**Files:**
- Create: `.github/workflows/sync-openapi-v3.yml`

**Step 1: Draft workflow behavior**

Define:
- `schedule` with a weekly cron
- `workflow_dispatch`
- `contents: write` permissions
- checkout, `jq` install, sync script run, diff detection, conditional commit and push

**Step 2: Validate assumptions locally**

Run: `scripts/update-openapi-v3.sh`
Expected: local snapshot refresh succeeds without manual edits

**Step 3: Write minimal workflow**

Create the YAML so it only automates the existing script and avoids unrelated build or release steps.

**Step 4: Review for no-op safety**

Confirm the commit step is gated so no commit is created when there is no diff.

### Task 2: Document the automation

**Files:**
- Modify: `README.md`

**Step 1: Add automation section**

Document:
- weekly schedule
- manual trigger path
- direct commit behavior

**Step 2: Keep local workflow intact**

Retain the existing local script instructions so the workflow and local operation stay aligned.

### Task 3: Verify and finalize

**Files:**
- Review: `.github/workflows/sync-openapi-v3.yml`
- Review: `README.md`

**Step 1: Run the sync script**

Run: `./scripts/update-openapi-v3.sh`
Expected: command succeeds and writes `openapi/openapi-v3.json`

**Step 2: Inspect git diff**

Run: `git diff -- .github/workflows/sync-openapi-v3.yml README.md docs/plans`
Expected: only the workflow, docs, and intentional README changes appear

**Step 3: Commit**

Run:
```bash
git add .github/workflows/sync-openapi-v3.yml README.md docs/plans
git commit -m "chore: automate openapi v3 sync"
```

Expected: a single commit capturing the workflow and docs
