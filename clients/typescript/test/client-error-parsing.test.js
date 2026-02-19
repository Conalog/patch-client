const test = require("node:test");
const assert = require("node:assert/strict");
const { PatchClientError, PatchClientV3 } = require("../dist");

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

  const result = await client.getPlantList();
  assert.deepEqual(result, { ok: true });
});

test("preserves existing lowercase bearer prefix", async () => {
  let authorization;
  const client = new PatchClientV3({
    accessToken: "bearer token-value",
    fetchFn: async (_url, init) => {
      authorization = init.headers.Authorization;
      return new Response(JSON.stringify({ ok: true }), {
        status: 200,
        headers: { "content-type": "application/json" },
      });
    },
  });

  await client.getPlantList();
  assert.equal(authorization, "bearer token-value");
});
