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

test("parses mixed-case +json error content-type as object payload", async () => {
  const client = new PatchClientV3({
    fetchFn: async () =>
      new Response(JSON.stringify({ title: "Bad Request", detail: "mixed-case content-type" }), {
        status: 400,
        headers: { "content-type": "Application/Problem+JSON" },
      }),
  });

  await assert.rejects(
    () => client.getPlantList(),
    (err) => {
      assert.ok(err instanceof PatchClientError);
      assert.deepEqual(err.payload, {
        title: "Bad Request",
        detail: "mixed-case content-type",
      });
      return true;
    }
  );
});

test("removes content-type header case-insensitively for multipart uploads", async () => {
  const client = new PatchClientV3({
    defaultHeaders: { "content-type": "application/json" },
    fetchFn: async (_url, init) => {
      const headers = new Headers(init?.headers);
      assert.equal(headers.has("content-type"), false);
      return new Response(JSON.stringify({ ok: true }), {
        status: 200,
        headers: { "content-type": "application/json" },
      });
    },
  });

  const form = new FormData();
  form.append("filename", new Blob(["hello"], { type: "text/plain" }), "hello.txt");
  await client.uploadPlantFiles("plant-1", form);
});

test("aborts request when timeoutMs is set", async () => {
  const client = new PatchClientV3({
    fetchFn: (_url, init) =>
      new Promise((_resolve, reject) => {
        init?.signal?.addEventListener("abort", () => {
          reject(new DOMException("Aborted", "AbortError"));
        });
      }),
  });

  await assert.rejects(
    () => client.getPlantList(undefined, { timeoutMs: 10 }),
    (err) => {
      assert.ok(err instanceof PatchClientError);
      assert.equal(err.status, 0);
      assert.equal(err.method, "GET");
      assert.match(err.url, /\/api\/v3\/plants$/);
      assert.equal(err.cause?.name, "AbortError");
      return true;
    }
  );
});

test("treats header overrides case-insensitively", async () => {
  let observedAuthHeader = "";

  const client = new PatchClientV3({
    accessToken: "primary-token",
    fetchFn: async (_url, init) => {
      const headers = new Headers(init?.headers);
      observedAuthHeader = headers.get("authorization") ?? "";
      return new Response(JSON.stringify({ ok: true }), {
        status: 200,
        headers: { "content-type": "application/json" },
      });
    },
  });

  await client.getPlantList(undefined, {
    headers: { authorization: "Bearer secondary-token" },
  });

  assert.equal(observedAuthHeader, "Bearer secondary-token");
});

test("keeps lowercase bearer scheme without double-prefixing", async () => {
  let observedAuthHeader = "";
  const client = new PatchClientV3({
    accessToken: "bearer token-value",
    fetchFn: async (_url, init) => {
      const headers = new Headers(init?.headers);
      observedAuthHeader = headers.get("authorization") ?? "";
      return new Response(JSON.stringify({ ok: true }), {
        status: 200,
        headers: { "content-type": "application/json" },
      });
    },
  });

  await client.getPlantList();
  assert.equal(observedAuthHeader, "bearer token-value");
});

test("rejects insecure non-loopback http baseUrl without opt-in", () => {
  assert.throws(
    () => new PatchClientV3({ baseUrl: "http://example.com", fetchFn: async () => new Response() }),
    /allowInsecureHttp=true/
  );
});

test("allows insecure non-loopback http baseUrl with opt-in", async () => {
  const client = new PatchClientV3({
    baseUrl: "http://example.com",
    allowInsecureHttp: true,
    fetchFn: async () =>
      new Response(JSON.stringify({ ok: true }), {
        status: 200,
        headers: { "content-type": "application/json" },
      }),
  });
  const out = await client.getPlantList();
  assert.deepEqual(out, { ok: true });
});

test("rejects baseUrl with query or fragment components", () => {
  assert.throws(
    () =>
      new PatchClientV3({
        baseUrl: "https://example.com?x=1",
        fetchFn: async () => new Response(),
      }),
    /must not include query or fragment/
  );
  assert.throws(
    () =>
      new PatchClientV3({
        baseUrl: "https://example.com#frag",
        fetchFn: async () => new Response(),
      }),
    /must not include query or fragment/
  );
});

test("rejects baseUrl with embedded credentials", () => {
  assert.throws(
    () =>
      new PatchClientV3({
        baseUrl: "https://user:pass@example.com",
        fetchFn: async () => new Response(),
      }),
    /must not include credentials/
  );
});

test("preserves baseUrl path prefix when building request URLs", async () => {
  let observedUrl = "";
  const client = new PatchClientV3({
    baseUrl: "https://example.com/custom-prefix",
    fetchFn: async (url) => {
      observedUrl = url;
      return new Response(JSON.stringify({ ok: true }), {
        status: 200,
        headers: { "content-type": "application/json" },
      });
    },
  });
  await client.getPlantList();
  assert.match(observedUrl, /https:\/\/example\.com\/custom-prefix\/api\/v3\/plants$/);
});

test("allows insecure IPv6 loopback baseUrl without opt-in", async () => {
  const client = new PatchClientV3({
    baseUrl: "http://[::1]:8080",
    fetchFn: async () =>
      new Response(JSON.stringify({ ok: true }), {
        status: 200,
        headers: { "content-type": "application/json" },
      }),
  });
  const out = await client.getPlantList();
  assert.deepEqual(out, { ok: true });
});

test("sets redirect=manual when Authorization header is present", async () => {
  let observedRedirectPolicy = "";
  const client = new PatchClientV3({
    accessToken: "token-value",
    fetchFn: async (_url, init) => {
      observedRedirectPolicy = init?.redirect ?? "";
      return new Response(JSON.stringify({ ok: true }), {
        status: 200,
        headers: { "content-type": "application/json" },
      });
    },
  });
  await client.getAccountInfo();
  assert.equal(observedRedirectPolicy, "manual");
});

test("sets redirect=manual when request has credential-bearing body", async () => {
  let observedRedirectPolicy = "";
  const client = new PatchClientV3({
    fetchFn: async (_url, init) => {
      observedRedirectPolicy = init?.redirect ?? "";
      return new Response(JSON.stringify({ token: "ok" }), {
        status: 200,
        headers: { "content-type": "application/json" },
      });
    },
  });
  await client.authenticateUser({ email: "u@example.com", password: "pw" });
  assert.equal(observedRedirectPolicy, "manual");
});

test("returns Uint8Array for binary responses", async () => {
  const bytes = new Uint8Array([0xff, 0x00, 0x01]);
  const client = new PatchClientV3({
    fetchFn: async () =>
      new Response(bytes, {
        status: 200,
        headers: { "content-type": "application/octet-stream" },
      }),
  });
  const out = await client.getPlantList();
  assert.ok(out instanceof Uint8Array);
  assert.deepEqual(Array.from(out), [255, 0, 1]);
});

test("fails when response exceeds maxResponseBytes", async () => {
  const client = new PatchClientV3({
    maxResponseBytes: 4,
    fetchFn: async () =>
      new Response("12345", {
        status: 200,
        headers: { "content-type": "text/plain", "content-length": "5" },
      }),
  });
  await assert.rejects(
    () => client.getPlantList(),
    (err) => {
      assert.ok(err instanceof PatchClientError);
      assert.equal(err.status, 200);
      assert.match(String(err.payload?.error ?? err.payload), /response exceeded 4 bytes/i);
      return true;
    }
  );
});

test("fails when runtime cannot enforce maxResponseBytes without streaming body", async () => {
  const client = new PatchClientV3({
    maxResponseBytes: 4,
    fetchFn: async () => ({
      ok: true,
      status: 200,
      headers: { get: () => null },
      body: null,
      arrayBuffer: async () => new Uint8Array([1, 2, 3]).buffer,
      text: async () => "",
    }),
  });

  await assert.rejects(
    () => client.getPlantList(),
    (err) => {
      assert.ok(err instanceof PatchClientError);
      assert.equal(err.status, 200);
      assert.match(
        String(err.payload?.error ?? err.payload),
        /cannot be bounded.*missing streaming body/i
      );
      return true;
    }
  );
});

test("accepts valid no-body success responses without parse errors", async () => {
  const client = new PatchClientV3({
    maxResponseBytes: 4,
    fetchFn: async () => ({
      ok: true,
      status: 205,
      headers: { get: () => null },
      body: null,
      arrayBuffer: async () => new Uint8Array().buffer,
      text: async () => "",
    }),
  });

  const out = await client.getPlantList();
  assert.equal(out, null);
});

test("allows opting out of maxResponseBytes enforcement with Infinity", async () => {
  const client = new PatchClientV3({
    maxResponseBytes: Number.POSITIVE_INFINITY,
    fetchFn: async () => ({
      ok: true,
      status: 200,
      headers: { get: () => null },
      body: null,
      arrayBuffer: async () => new Uint8Array([1, 2, 3]).buffer,
      text: async () => "",
    }),
  });

  const out = await client.getPlantList();
  assert.ok(out instanceof Uint8Array);
  assert.deepEqual(Array.from(out), [1, 2, 3]);
});

test("fails when async-iterable response body exceeds maxResponseBytes", async () => {
  let destroyed = false;
  const client = new PatchClientV3({
    maxResponseBytes: 4,
    fetchFn: async () => ({
      ok: true,
      status: 200,
      headers: { get: () => null },
      body: {
        async *[Symbol.asyncIterator]() {
          yield Uint8Array.from([1, 2, 3]);
          yield Uint8Array.from([4, 5]);
        },
        destroy: () => {
          destroyed = true;
        },
      },
      arrayBuffer: async () => new Uint8Array().buffer,
      text: async () => "",
    }),
  });

  await assert.rejects(
    () => client.getPlantList(),
    (err) => {
      assert.ok(err instanceof PatchClientError);
      assert.equal(err.status, 200);
      assert.match(String(err.payload?.error ?? err.payload), /response exceeded 4 bytes/i);
      assert.equal(destroyed, true);
      return true;
    }
  );
});

test("fails for unsupported async-iterable chunk types", async () => {
  const client = new PatchClientV3({
    maxResponseBytes: 16,
    fetchFn: async () => ({
      ok: true,
      status: 200,
      headers: { get: () => null },
      body: {
        async *[Symbol.asyncIterator]() {
          yield { bad: true };
        },
      },
      arrayBuffer: async () => new Uint8Array().buffer,
      text: async () => "",
    }),
  });
  await assert.rejects(
    () => client.getPlantList(),
    (err) => {
      assert.ok(err instanceof PatchClientError);
      assert.equal(err.status, 200);
      assert.match(String(err.payload?.error ?? err.payload), /unsupported response body chunk type/i);
      return true;
    }
  );
});

test("cancels response body when content-length already exceeds maxResponseBytes", async () => {
  let canceled = false;
  const client = new PatchClientV3({
    maxResponseBytes: 4,
    fetchFn: async () => ({
      ok: true,
      status: 200,
      headers: { get: () => "5" },
      body: {
        cancel: async () => {
          canceled = true;
        },
      },
      arrayBuffer: async () => new Uint8Array([1, 2, 3, 4, 5]).buffer,
      text: async () => "",
    }),
  });

  await assert.rejects(
    () => client.getPlantList(),
    (err) => {
      assert.ok(err instanceof PatchClientError);
      assert.equal(err.status, 200);
      assert.equal(canceled, true);
      return true;
    }
  );
});

test("wraps JSON serialization errors in PatchClientError", async () => {
  const client = new PatchClientV3({
    fetchFn: async () =>
      new Response(JSON.stringify({ ok: true }), {
        status: 200,
        headers: { "content-type": "application/json" },
      }),
  });

  await assert.rejects(
    () => client.createPlant({ value: BigInt(1) }),
    (err) => {
      assert.ok(err instanceof PatchClientError);
      assert.equal(err.status, 0);
      assert.match(String(err.payload?.error ?? err.payload), /bigint/i);
      return true;
    }
  );
});

test("fails fast when timeoutMs is requested without AbortController support", async () => {
  const originalAbortController = globalThis.AbortController;
  globalThis.AbortController = undefined;
  try {
    const client = new PatchClientV3({
      fetchFn: async () => await new Promise(() => {}),
    });
    await assert.rejects(
      () => client.getPlantList(undefined, { timeoutMs: 10 }),
      (err) => {
        assert.ok(err instanceof PatchClientError);
        assert.equal(err.status, 0);
        assert.match(String(err.cause), /requires AbortController/i);
        return true;
      }
    );
  } finally {
    globalThis.AbortController = originalAbortController;
  }
});

test("maps body-read abort errors to status 0", async () => {
  const client = new PatchClientV3({
    fetchFn: async () => ({
      ok: true,
      status: 200,
      headers: { get: () => null },
      body: {
        getReader: () => ({
          read: async () => {
            const err = new Error("The operation was aborted");
            err.name = "AbortError";
            throw err;
          },
        }),
      },
      arrayBuffer: async () => new Uint8Array().buffer,
      text: async () => "",
    }),
  });

  await assert.rejects(
    () => client.getPlantList(undefined, { timeoutMs: 50 }),
    (err) => {
      assert.ok(err instanceof PatchClientError);
      assert.equal(err.status, 0);
      assert.match(String(err.payload?.error ?? err.payload), /abort/i);
      return true;
    }
  );
});

test("clears timeout timer when fetchFn throws synchronously", async () => {
  const client = new PatchClientV3({
    fetchFn: () => {
      throw new Error("sync failure");
    },
  });
  const started = Date.now();
  await assert.rejects(() => client.getPlantList(undefined, { timeoutMs: 2000 }), (err) => {
    assert.ok(err instanceof PatchClientError);
    assert.match(String(err.cause), /sync failure/);
    return true;
  });
  assert.ok(Date.now() - started < 500, "synchronous failure should not wait for timeout");
});
