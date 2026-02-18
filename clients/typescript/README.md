# patch-client (TypeScript)

Handwritten TypeScript client for PATCH Plant Data API v3.

## Installation

```bash
npm install patch-client
```

## Quick Start (TypeScript)

```ts
import { PatchClientV3 } from "patch-client";

const client = new PatchClientV3({
  accessToken: process.env.PATCH_TOKEN,
  accountType: "manager",
});

const plants = await client.getPlantList({ page: 0, size: 20 });
```

## Also Usable from JavaScript

This package is authored in TypeScript, but distributed as JavaScript (`dist/*.js`),
so you can use it without TypeScript.

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

## Runtime Requirements

- `fetch` is required.
  - Node.js 18+ provides it by default.
  - For older Node.js versions, inject `fetchFn` manually.
- `FormData` is required when using the file upload API (`uploadPlantFiles`).
  - Node.js 18+ and modern browsers provide it by default.
  - In older Node.js environments, install `form-data` and pass the created instance directly to `uploadPlantFiles`.

### `fetchFn` Injection Example (Legacy Node.js)

`node-fetch` v3 is ESM-only. In CommonJS, use `node-fetch@2`.

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

### `FormData` Injection Example (Legacy Node.js)

`node-fetch` v3 is ESM-only. In CommonJS, use `node-fetch@2`.

#### CommonJS (`node-fetch@2`)

Install `form-data` and `node-fetch@2`.

```bash
npm install form-data node-fetch@2
```

```js
const fs = require("fs");
const path = require("path");
const FormData = require("form-data");
const fetch = require("node-fetch");
const { PatchClientV3, PatchClientError } = require("patch-client");

(async () => {
  try {
    const client = new PatchClientV3({
      accessToken: process.env.PATCH_TOKEN,
      accountType: "manager",
      fetchFn: fetch,
    });

    const formData = new FormData();
    const filePath = "/path/to/your/actual/file.csv"; // Replace with a real file path.
    formData.append("file", fs.createReadStream(filePath), path.basename(filePath));

    const result = await client.uploadPlantFiles("your-plant-id", formData); // Replace with a real plant ID.
    console.log("Successfully uploaded files:", result);
  } catch (err) {
    if (err instanceof PatchClientError) {
      console.error("File upload API error:", err.status, err.payload);
    } else {
      console.error("Error while uploading files:", err);
    }
  }
})();
```

#### ESM (`node-fetch@3+`)

Install `form-data` and `node-fetch`.

```bash
npm install form-data node-fetch
```

```js
import fs from "fs";
import path from "path";
import FormData from "form-data";
import fetch from "node-fetch";
import { PatchClientV3, PatchClientError } from "patch-client";

(async () => {
  try {
    const client = new PatchClientV3({
      accessToken: process.env.PATCH_TOKEN,
      accountType: "manager",
      fetchFn: fetch,
    });

    const formData = new FormData();
    const filePath = "/path/to/your/actual/file.csv"; // Replace with a real file path.
    formData.append("file", fs.createReadStream(filePath), path.basename(filePath));

    const result = await client.uploadPlantFiles("your-plant-id", formData); // Replace with a real plant ID.
    console.log("Successfully uploaded files:", result);
  } catch (err) {
    if (err instanceof PatchClientError) {
      console.error("File upload API error:", err.status, err.payload);
    } else {
      console.error("Error while uploading files:", err);
    }
  }
})();
```

## Authentication / Headers

- `accessToken` accepts either `Bearer <token>` or a raw token.
- `accountType` should be one of `"viewer"`, `"manager"`, or `"admin"`.

## Error Handling

When a request fails, `PatchClientError` is thrown. You can inspect `status` and `payload`.

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
      console.error("Plant list API error:", err.status, err.payload);
    } else {
      console.error("Error while fetching plant list:", err);
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
      console.error("Plant list API error:", err.status, err.payload);
    } else {
      console.error("Error while fetching plant list:", err);
    }
  }
})();
```
