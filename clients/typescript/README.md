# patch-client (TypeScript)

PATCH Plant Data API v3용 수작업 TypeScript 클라이언트입니다.

## 설치

```bash
npm install patch-client
```

## 사용

```ts
import { PatchClientV3 } from "patch-client";

const client = new PatchClientV3({
  accessToken: process.env.PATCH_TOKEN,
  accountType: "manager",
});

const plants = await client.getPlantList({ page: 0, size: 20 });
```
