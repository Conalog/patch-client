# patch-client

This repository contains handwritten multi-language clients for the `/api/v3` endpoints and a shared OpenAPI snapshot covering `/api/v3` and `/api/v4` in the PATCH Plant Data API.

## What is in this repo

- TypeScript client (`clients/typescript`)
- Python client (`clients/python`)
- Go client (`clients/go`)
- Rust client (`clients/rust`)
- OpenAPI snapshot for `/api/v3` and `/api/v4` (`openapi/openapi-v3.json`)

## Repository structure

```text
.
├── clients/
│   ├── typescript/
│   ├── python/
│   ├── go/
│   └── rust/
├── openapi/
│   └── openapi-v3.json
└── scripts/
    └── update-openapi-v3.sh
```

## Language clients

For language-specific usage, see each client README:

- TypeScript: `clients/typescript/README.md`
- Python: `clients/python/README.md`
- Go: `clients/go/README.md`
- Rust: `clients/rust/README.md` (crate: `patch-client`)

## API spec sync (v3 + v4)

Extracts `/api/v3/` and `/api/v4/` paths from the source OpenAPI and updates `openapi/openapi-v3.json`.

Requirements:
- `curl`
- `jq`

Run:

```bash
./scripts/update-openapi-v3.sh
```

## Automated spec sync

GitHub Actions also keeps `openapi/openapi-v3.json` up to date automatically.

- Weekly schedule: every Monday at 00:00 UTC, which is Monday 09:00 in Korea Standard Time
- Manual trigger: Actions -> `Sync OpenAPI v3+v4` -> `Run workflow`
- Commit behavior: if the extracted v3+v4 snapshot changed, the workflow commits the updated `openapi/openapi-v3.json` directly to `main`

The workflow is defined in `.github/workflows/sync-openapi-v3.yml` and reuses `scripts/update-openapi-v3.sh` so local and automated sync stay consistent.

## Local development

### TypeScript

```bash
cd clients/typescript
npm install
npm run build
```

### Python

```bash
cd clients/python
python -m pip install -e .
```

### Go

```bash
cd clients/go
go test ./...
```

### Rust

```bash
cd clients/rust
cargo test
```

## Notes

- The default API base URL is `https://patch-api.conalog.com` across clients.
- The current OpenAPI snapshot uses OpenAPI `3.1.0`, reports product version `3.0.0`, and contains 131 operations: 130 under `/api/v3` and 1 under `/api/v4`.
- The Rust client covers `/api/v3` and `GET /api/v4/plants/{plant_id}/blueprint`; the Go, Python, and TypeScript clients currently target `/api/v3`.
- Manager authentication uses an access token and `Account-Type: manager`. Fieldwork participant tokens omit `Account-Type`; see each client README.
