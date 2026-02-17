# patch-client (TypeScript)

PATCH Plant Data API v3용 수작업 TypeScript 클라이언트입니다.

## 설치

```bash
npm install patch-client
```

## 빠른 시작 (TypeScript)

```ts
import { PatchClientV3 } from "patch-client";

const client = new PatchClientV3({
  accessToken: process.env.PATCH_TOKEN,
  accountType: "manager",
});

const plants = await client.getPlantList({ page: 0, size: 20 });
```

## JavaScript에서도 사용 가능

이 패키지는 TypeScript로 작성되었지만, 배포물은 JavaScript(`dist/*.js`)이므로
TypeScript 없이도 사용할 수 있습니다.

### CommonJS (`require`)

```js
const { PatchClientV3 } = require("patch-client");

(async () => {
  const client = new PatchClientV3({
    accessToken: process.env.PATCH_TOKEN,
    accountType: "manager",
  });

  const plants = await client.getPlantList({ page: 0, size: 20 });
  console.log("Successfully fetched plants:", plants);
})();
```

### ESM (`import`)

```js
import { PatchClientV3 } from "patch-client";

(async () => {
  const client = new PatchClientV3({
    accessToken: process.env.PATCH_TOKEN,
    accountType: "manager",
  });

  const plants = await client.getPlantList({ page: 0, size: 20 });
  console.log("Successfully fetched plants:", plants);
})();
```

## 런타임 요구사항

- `fetch`가 필요합니다.
  - Node.js 18+에서는 기본 제공됩니다.
  - 구버전 Node.js에서는 `fetchFn`을 직접 주입하세요.
- 파일 업로드 API(`uploadPlantFiles`)를 사용할 때는 `FormData`가 필요합니다.
  - Node.js 18+ 및 최신 브라우저 환경에서는 기본 제공됩니다.
  - 구버전 Node.js 환경에서는 `form-data` 패키지를 설치한 후, 생성한 인스턴스를 `uploadPlantFiles` 메서드에 직접 전달해야 합니다.

### `fetchFn` 주입 예시 (구버전 Node.js)

`node-fetch` v3는 ESM만 지원합니다. CommonJS 환경에서는 `node-fetch@2`를 사용해야 합니다.

#### CommonJS (`node-fetch@2`)

```bash
npm install node-fetch@2
```

```js
const fetch = require("node-fetch");
const { PatchClientV3 } = require("patch-client");

(async () => {
  const client = new PatchClientV3({
    accessToken: process.env.PATCH_TOKEN,
    accountType: "manager",
    fetchFn: fetch,
  });

  const plants = await client.getPlantList({ page: 0, size: 20 });
  console.log("Successfully fetched plants:", plants);
})();
```

#### ESM (`node-fetch@3+`)

```bash
npm install node-fetch
```

```js
import fetch from "node-fetch";
import { PatchClientV3 } from "patch-client";

(async () => {
  const client = new PatchClientV3({
    accessToken: process.env.PATCH_TOKEN,
    accountType: "manager",
    fetchFn: fetch,
  });

  const plants = await client.getPlantList({ page: 0, size: 20 });
  console.log("Successfully fetched plants:", plants);
})();
```

### `FormData` 주입 예시 (구버전 Node.js)

`node-fetch` v3는 ESM만 지원합니다. CommonJS 환경에서는 `node-fetch@2`를 사용해야 합니다.

#### CommonJS (`node-fetch@2`)

`form-data`와 `node-fetch@2` 패키지를 설치하세요.

```bash
npm install form-data node-fetch@2
```

```js
const fs = require("fs");
const path = require("path");
const FormData = require("form-data");
const fetch = require("node-fetch");
const { PatchClientV3 } = require("patch-client");

(async () => {
  try {
    const client = new PatchClientV3({
      accessToken: process.env.PATCH_TOKEN,
      accountType: "manager",
      fetchFn: fetch,
    });

    const formData = new FormData();
    const filePath = "/path/to/your/actual/file.csv"; // 실제 파일 경로로 변경하세요.
    formData.append("file", fs.createReadStream(filePath), path.basename(filePath));

    const result = await client.uploadPlantFiles("your-plant-id", formData); // 실제 플랜트 ID로 변경하세요.
    console.log("Successfully uploaded files:", result);
  } catch (err) {
    console.error("An error occurred:", err);
  }
})();
```

#### ESM (`node-fetch@3+`)

`form-data`와 `node-fetch` 패키지를 설치하세요.

```bash
npm install form-data node-fetch
```

```js
import fs from "fs";
import path from "path";
import FormData from "form-data";
import fetch from "node-fetch";
import { PatchClientV3 } from "patch-client";

(async () => {
  try {
    const client = new PatchClientV3({
      accessToken: process.env.PATCH_TOKEN,
      accountType: "manager",
      fetchFn: fetch,
    });

    const formData = new FormData();
    const filePath = "/path/to/your/actual/file.csv"; // 실제 파일 경로로 변경하세요.
    formData.append("file", fs.createReadStream(filePath), path.basename(filePath));

    const result = await client.uploadPlantFiles("your-plant-id", formData); // 실제 플랜트 ID로 변경하세요.
    console.log("Successfully uploaded files:", result);
  } catch (err) {
    console.error("An error occurred:", err);
  }
})();
```

## 인증/헤더

- `accessToken`은 `Bearer <token>` 또는 raw token 모두 허용합니다.
- `accountType`은 `"viewer"`, `"manager"`, 또는 `"admin"` 중 하나를 사용하세요.

## 에러 처리

요청 실패 시 `PatchClientError`가 발생하며, `status`와 `payload`를 확인할 수 있습니다.

#### CommonJS (`require`)

```js
const { PatchClientV3, PatchClientError } = require("patch-client");

(async () => {
  try {
    const client = new PatchClientV3({
      accessToken: process.env.PATCH_TOKEN,
      accountType: "manager",
    });
    const plants = await client.getPlantList({ page: 0, size: 20 });
    console.log("Successfully fetched plants:", plants);
  } catch (err) {
    if (err instanceof PatchClientError) {
      console.error(err.status, err.payload);
    } else {
      console.error(err);
    }
  }
})();
```

#### ESM (`import`)

```js
import { PatchClientV3, PatchClientError } from "patch-client";

(async () => {
  try {
    const client = new PatchClientV3({
      accessToken: process.env.PATCH_TOKEN,
      accountType: "manager",
    });
    const plants = await client.getPlantList({ page: 0, size: 20 });
    console.log("Successfully fetched plants:", plants);
  } catch (err) {
    if (err instanceof PatchClientError) {
      console.error(err.status, err.payload);
    } else {
      console.error(err);
    }
  }
})();
```
