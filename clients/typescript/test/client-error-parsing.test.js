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
