# patch-client (TypeScript)

Handwritten TypeScript client for PATCH Plant Data API v3.

## Installation

```bash
npm install patch-client
```

## Get a Token First (Login)

`<issued-jwt-token>` in examples means a token returned by `authenticateUser`.
If you store tokens in env vars, load that issued token from `process.env.PATCH_TOKEN`.

```ts
import { PatchClientV3 } from "patch-client";

const authClient = new PatchClientV3();
const auth = await authClient.authenticateUser({
  type: "manager",
  email: process.env.PATCH_EMAIL,
  password: process.env.PATCH_PASSWORD,
});

const token = auth.token;
authClient.setAccessToken(token);
```

## Quick Start (TypeScript)

```ts
import { PatchClientV3 } from "patch-client";

const client = new PatchClientV3({
  accessToken: "<issued-jwt-token>",
  accountType: "manager",
});

const plants = await client.getPlantList({ page: 1, size: 20, full: true });
```

## Current v3 API Examples

```ts
const authMethods = await client.listOAuthMethods({ provider: "google" });
const oauthLocation = await client.startOAuthLogin({
  provider: "google",
  redirect_url: "myscheme://callback",
});

const weather = await client.getPlantWeatherForecast("your-plant-id", { days: 3 });
const registry = await client.getPlantRegistryLogs("your-plant-id", {
  date: "2026-07-07",
  asset_type: "inverter",
});

await client.assignPlantPermission("your-org-id", "your-plant-id", {
  type: "viewer",
  username: "viewer-user",
});

void authMethods;
void oauthLocation;
void weather;
void registry;
```

## Also Usable from JavaScript

This package is authored in TypeScript, but distributed as JavaScript (`dist/*.js`),
so you can use it without TypeScript.

### CommonJS (`require`)

```js
const { PatchClientV3 } = require("patch-client");

(async () => {
  const client = new PatchClientV3({
    accessToken: "<issued-jwt-token>",
    accountType: "manager",
  });

  const plants = await client.getPlantList({ page: 1, size: 20, full: true });
  console.log("Successfully fetched plants:", plants);
})();
```

### ESM (`import`)

```js
import { PatchClientV3 } from "patch-client";

(async () => {
  const client = new PatchClientV3({
    accessToken: "<issued-jwt-token>",
    accountType: "manager",
  });

  const plants = await client.getPlantList({ page: 1, size: 20, full: true });
  console.log("Successfully fetched plants:", plants);
})();
```

## Runtime Requirements

- `fetch` is required.
  - Node.js 18+ provides it by default.
  - For older Node.js versions, inject `fetchFn` manually.
  - When `maxResponseBytes` is set, the injected `fetchFn` should expose a streaming response body
    (`ReadableStream.getReader()` or async-iterable body) so byte limits can be enforced safely.
  - If your runtime cannot expose streaming response bodies, set `maxResponseBytes: Infinity` to
    opt out of response-size enforcement.

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
    accessToken: "<issued-jwt-token>",
    accountType: "manager",
    fetchFn: fetch,
  });

  const plants = await client.getPlantList({ page: 1, size: 20, full: true });
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
    accessToken: "<issued-jwt-token>",
    accountType: "manager",
    fetchFn: fetch,
  });

  const plants = await client.getPlantList({ page: 1, size: 20, full: true });
  console.log("Successfully fetched plants:", plants);
})();
```

## Authentication / Headers

- The package does not auto-issue tokens from environment variables.
- Issue a token with `authenticateUser(...)`, then pass it via `accessToken` (or `setAccessToken(...)`).
- Refresh an existing token with `refreshUserToken(...)` and update the client with the returned token.
- `accessToken` accepts either `Bearer <token>` or a raw token.
- `accountType` should be one of `"viewer"`, `"manager"`, or `"temporary"`.
- Set a request `accountType` to `null` for participant bearer tokens. This omits `Account-Type`.

## Fieldwork

Fieldwork methods use camel-cased OpenAPI operation IDs. Command methods require an idempotency key:

```ts
await client.fieldworkWorkCreate({ plant_id: "plant-1", title: "Inspect" }, {
  idempotencyKey: "work-create-001",
});
```

You can also provide `Idempotency-Key` in `options.headers`. The client rejects absent or malformed keys before it sends the request. Fieldwork bodies and results use `JsonObject` or `unknown` when the API schema is open-ended.

`fieldworkEvents()` returns raw SSE bytes. It does not parse or buffer SSE. It requires `AbortController` for its default setup timeout. For `watch: "unread"`, do not pass `work_id` or `surface`. A 30-second default timeout covers connection setup and a bounded non-2xx error body read. A positive `timeoutMs` overrides that budget. The timer stops after a valid SSE response arrives. The caller owns cancellation of an open stream through `signal` or the returned body. WHATWG bodies support `getReader()`. Legacy Node async-iterable bodies support `for await...of` and close through `body.cancel()` or `iterator.return()`; custom sources must unblock their own pending reads.

The upload methods use native `FormData`: `fieldworkUploadAttachment`, `uploadPlantFiles`, and `uploadPlantImages`.

## Error Handling

When a request fails, `PatchClientError` is thrown. You can inspect `status` and `payload`.

#### CommonJS (`require`)

```js
const { PatchClientV3, PatchClientError } = require("patch-client");

(async () => {
  try {
    const client = new PatchClientV3({
      accessToken: "<issued-jwt-token>",
      accountType: "manager",
    });
    const plants = await client.getPlantList({ page: 1, size: 20, full: true });
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
      accessToken: "<issued-jwt-token>",
      accountType: "manager",
    });
    const plants = await client.getPlantList({ page: 1, size: 20, full: true });
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
