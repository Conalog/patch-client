# patch-client

`patch-client` is a collection of handwritten clients that support only `v3` endpoints of the PATCH Plant Data API.

## Directories

- `/Users/cypark/Documents/work/patch-client/clients/typescript`: JavaScript/TypeScript client (`patch-client`)
- `/Users/cypark/Documents/work/patch-client/clients/python`: Python client (`patch-client`, import: `patch_client`)
- `/Users/cypark/Documents/work/patch-client/clients/go`: Go client (`patchclient` package)
- `/Users/cypark/Documents/work/patch-client/openapi/openapi-v3.json`: spec containing only v3 paths

## Sync v3 Spec

```bash
./scripts/update-openapi-v3.sh
```
