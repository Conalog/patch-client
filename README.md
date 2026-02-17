# patch-client

`patch-client`는 PATCH Plant Data API의 `v3` 엔드포인트만 지원하는 수작업 클라이언트 모음입니다.

## 디렉터리

- `/Users/cypark/Documents/work/patch-client/clients/typescript`: JavaScript/TypeScript 클라이언트 (`patch-client`)
- `/Users/cypark/Documents/work/patch-client/clients/python`: Python 클라이언트 (`patch-client`, import: `patch_client`)
- `/Users/cypark/Documents/work/patch-client/clients/go`: Go 클라이언트 (`patchclient` package)
- `/Users/cypark/Documents/work/patch-client/openapi/openapi-v3.json`: v3 경로만 남긴 스펙

## v3 스펙 동기화

```bash
./scripts/update-openapi-v3.sh
```
