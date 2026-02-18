# patch-client

This repository contains handwritten multi-language clients for the `/api/v3` endpoints of the PATCH Plant Data API.

## What is in this repo

- TypeScript client (`clients/typescript`)
- Python client (`clients/python`)
- Go client (`clients/go`)
- Rust client (`clients/rust`)
- v3-only OpenAPI spec (`openapi/openapi-v3.json`)

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
- Rust: `clients/rust` (crate: `patch-client`)

## API spec sync (v3 only)

Extracts only `/api/v3/` paths from the source OpenAPI and updates `openapi/openapi-v3.json`.

Requirements:
- `curl`
- `jq`

Run:

```bash
./scripts/update-openapi-v3.sh
```

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
- Authentication generally uses an `access token` and the `Account-Type` header (`viewer`, `manager`, `admin`).
