const test = require("node:test");
const assert = require("node:assert/strict");
const { PatchClientError, PatchClientV3 } = require("../dist");

function readHeader(headers, key) {
  if (!headers) {
    return undefined;
  }
  if (typeof headers.get === "function") {
    return headers.get(key);
  }
  return headers[key] ?? headers[key.toLowerCase()] ?? headers[key.toUpperCase()];
}

test("parses application/problem+json error response as object payload", async () => {
  const client = new PatchClientV3({
    fetchFn: async () =>
      new Response(JSON.stringify({ title: "Bad Request", detail: "invalid input" }), {
        status: 400,
        headers: { "content-type": "application/problem+json" },
      }),
  });

  await assert.rejects(
    () => client.getPlantList(),
    (err) => {
      assert.ok(err instanceof PatchClientError);
      assert.deepEqual(err.payload, { title: "Bad Request", detail: "invalid input" });
      return true;
    }
  );
});

test("parses case-insensitive JSON content-type for success payload", async () => {
  const client = new PatchClientV3({
    fetchFn: async () =>
      new Response(JSON.stringify({ ok: true }), {
        status: 200,
        headers: { "content-type": "Application/JSON; charset=utf-8" },
      }),
  });

  const payload = await client.getPlantList();
  assert.deepEqual(payload, { ok: true });
});

test("preserves lowercase bearer prefix without duplication", async () => {
  let capturedAuth;
  const client = new PatchClientV3({
    accessToken: "bearer token-value",
    fetchFn: async (_url, init) => {
      capturedAuth = readHeader(init.headers, "Authorization");
      return new Response(JSON.stringify({ ok: true }), {
        status: 200,
        headers: { "content-type": "application/json" },
      });
    },
  });

  await client.getPlantList();
  assert.equal(capturedAuth, "bearer token-value");
});

test("adds bearer prefix when token is raw", async () => {
  let capturedAuth;
  const client = new PatchClientV3({
    accessToken: "token-value",
    fetchFn: async (_url, init) => {
      capturedAuth = readHeader(init.headers, "Authorization");
      return new Response(JSON.stringify({ ok: true }), {
        status: 200,
        headers: { "content-type": "application/json" },
      });
    },
  });

  await client.getPlantList();
  assert.equal(capturedAuth, "Bearer token-value");
});
