const test = require("node:test");
const assert = require("node:assert/strict");
const http = require("node:http");
const { Readable } = require("node:stream");
const { PatchClientV3 } = require("../dist");

test("fieldwork commands validate idempotency before network and preserve auth", async () => {
  let called = false; let headers;
  const client = new PatchClientV3({ accessToken: "token", accountType: "manager", fetchFn: async (_url, init) => { called = true; headers = new Headers(init.headers); return new Response("{}", { headers: { "content-type": "application/json" } }); } });
  await assert.rejects(() => client.fieldworkWorkCreate({}, {}), /Idempotency-Key/);
  assert.equal(called, false);
  await client.fieldworkWorkCreate({ title: "x" }, { idempotencyKey: "command-01" });
  assert.equal(called, true);
  assert.equal(headers.get("authorization"), "Bearer token");
  assert.equal(headers.get("idempotency-key"), "command-01");
  await assert.rejects(() => client.fieldworkWorkCreate({}, { idempotencyKey: "a".repeat(7) }), /Idempotency-Key/);
  await client.fieldworkWorkCreate({}, { idempotencyKey: "a".repeat(8) });
  await client.fieldworkWorkCreate({}, { idempotencyKey: "a".repeat(128) });
  await assert.rejects(() => client.fieldworkWorkCreate({}, { idempotencyKey: "a".repeat(129) }), /Idempotency-Key/);
  await assert.rejects(() => client.fieldworkWorkCreate({}, { idempotencyKey: "command-01\n" }), /Idempotency-Key/);
  await assert.rejects(() => client.fieldworkWorkCreate(undefined, { idempotencyKey: "command-01" }), /body must be/);
  await assert.rejects(() => client.fieldworkWorkCreate({}, undefined), /options are required/);
});

test("fieldwork uses encoded paths and comma-joined arrays", async () => {
  let url = "";
  const client = new PatchClientV3({ fetchFn: async (input) => { url = input; return new Response("{}", { headers: { "content-type": "application/json" } }); } });
  await client.getMetricsByDate("plant/a", "device", "plant", "1d", "2026-01-01", { fields: ["a", "b"] });
  assert.match(url, /plant%2Fa/);
  assert.match(url, /fields=a%2Cb/);
});

test("uploads native FormData without a JSON content type", async () => {
  let body; let contentType;
  const client = new PatchClientV3({ fetchFn: async (_input, init) => { body = init.body; contentType = new Headers(init.headers).get("content-type"); return new Response("{}", { headers: { "content-type": "application/json" } }); } });
  await client.uploadPlantFiles("plant-1", { file: new Blob(["x"]), name: "report" });
  assert.ok(body instanceof FormData);
  assert.equal(contentType, null);
});

test("fieldwork events returns its native stream and validates unread watch", async () => {
  const stream = new ReadableStream({ start(controller) { controller.enqueue(new TextEncoder().encode("data: x\\n\\n")); } });
  const client = new PatchClientV3({ fetchFn: async () => new Response(stream, { headers: { "content-type": "text/event-stream" } }) });
  const body = await client.fieldworkEvents({ watch: "unread" });
  assert.ok(body.getReader);
  await body.cancel();
  await assert.rejects(() => client.fieldworkEvents({ watch: "unread", work_id: "w" }), /must not include/);
});

test("fieldwork stream timeout ends after connection while caller cancellation remains active", async () => {
  let signal;
  const stream = new ReadableStream();
  const client = new PatchClientV3({ fetchFn: async (_input, init) => { signal = init.signal; return new Response(stream, { headers: { "content-type": "text/event-stream" } }); } });
  const caller = new AbortController();
  await client.fieldworkEvents({}, { timeoutMs: 5, signal: caller.signal });
  await new Promise((resolve) => setTimeout(resolve, 15));
  assert.equal(signal.aborted, false);
  caller.abort();
  assert.equal(signal.aborted, true);
});

test("fieldwork stream uses the default connection timeout budget", async () => {
  let signal;
  const client = new PatchClientV3({ fetchFn: async (_input, init) => { signal = init.signal; return new Response(new ReadableStream(), { headers: { "content-type": "text/event-stream" } }); } });
  const body = await client.fieldworkEvents();
  assert.ok(signal);
  await body.cancel();
});

test("attachment bytes stay raw and signed downloads omit auth headers", async () => {
  const seen = [];
  const client = new PatchClientV3({
    accessToken: "token",
    accountType: "manager",
    defaultHeaders: { Authorization: "Bearer default", "Account-Type": "manager" },
    fetchFn: async (_input, init) => {
      seen.push(new Headers(init.headers));
      return new Response('{"not":"parsed"}', { headers: { "content-type": "application/json" } });
    },
  });
  const content = await client.fieldworkAttachmentContent({ work_id: "w", object_key: "o" });
  const download = await client.fieldworkAttachmentDownload({ work_id: "w", object_key: "o", expires: "1", signature: "s" }, { headers: { Authorization: "Bearer override", "Account-Type": "manager" } });
  assert.deepEqual([...content], [...new TextEncoder().encode('{"not":"parsed"}')]);
  assert.deepEqual([...download], [...new TextEncoder().encode('{"not":"parsed"}')]);
  assert.equal(seen[0].get("authorization"), "Bearer token");
  assert.equal(seen[1].get("authorization"), null);
  assert.equal(seen[1].get("account-type"), null);
});

test("accountType null removes inherited account type headers", async () => {
  let headers;
  const client = new PatchClientV3({ accountType: "manager", defaultHeaders: { "Account-Type": "manager" }, fetchFn: async (_input, init) => { headers = new Headers(init.headers); return new Response("{}", { headers: { "content-type": "application/json" } }); } });
  await client.fieldworkParticipantSessionCreate({}, { accountType: null });
  assert.equal(headers.get("account-type"), null);
});

test("fieldwork stream requires SSE content type and releases its abort listener on cancel", async () => {
  let cancelled = false;
  const wrongType = new PatchClientV3({ fetchFn: async () => ({ ok: true, status: 200, headers: new Headers({ "content-type": "application/json" }), text: async () => "x", arrayBuffer: async () => new ArrayBuffer(0), body: { cancel: async () => { cancelled = true; } } }) });
  await assert.rejects(() => wrongType.fieldworkEvents(), /text\/event-stream/);
  assert.equal(cancelled, true);
  let added = 0; let removed = 0; let listener;
  const signal = { aborted: false, addEventListener(_type, fn) { added++; listener = fn; }, removeEventListener(_type, fn) { if (fn === listener) removed++; } };
  const client = new PatchClientV3({ fetchFn: async () => new Response(new ReadableStream(), { headers: { "content-type": "text/event-stream" } }) });
  const body = await client.fieldworkEvents({}, { timeoutMs: 20, signal });
  await body.cancel();
  assert.equal(added, 1);
  assert.equal(removed, 1);
});

test("fieldwork stream releases its abort listener at EOF and read errors", async () => {
  const trackedSignal = () => {
    let listener; let removed = 0;
    return { signal: { aborted: false, addEventListener(_type, fn) { listener = fn; }, removeEventListener(_type, fn) { if (fn === listener) removed++; } }, removed: () => removed };
  };
  const eof = trackedSignal();
  const eofClient = new PatchClientV3({ fetchFn: async () => new Response(new ReadableStream({ start(controller) { controller.close(); } }), { headers: { "content-type": "text/event-stream" } }) });
  const eofBody = await eofClient.fieldworkEvents({}, { timeoutMs: 20, signal: eof.signal });
  await eofBody.getReader().read();
  assert.equal(eof.removed(), 1);
  const failed = trackedSignal();
  const failedClient = new PatchClientV3({ fetchFn: async () => new Response(new ReadableStream({ pull() { throw new Error("stream failed"); } }), { headers: { "content-type": "text/event-stream" } }) });
  const failedBody = await failedClient.fieldworkEvents({}, { timeoutMs: 20, signal: failed.signal });
  await assert.rejects(() => failedBody.getReader().read(), /stream failed/);
  assert.equal(failed.removed(), 1);
});

test("fieldwork streams async-iterable bodies with cleanup and abort forwarding", async () => {
  const trackedSignal = () => {
    let listener; let removed = 0;
    return { signal: { aborted: false, addEventListener(_type, fn) { listener = fn; }, removeEventListener(_type, fn) { if (fn === listener) removed++; } }, abort() { listener(); }, removed: () => removed };
  };
  const response = (body) => ({ ok: true, status: 200, headers: new Headers({ "content-type": "text/event-stream" }), text: async () => "", arrayBuffer: async () => new ArrayBuffer(0), body });
  const eofSignal = trackedSignal();
  const eofClient = new PatchClientV3({ fetchFn: async () => response({ async *[Symbol.asyncIterator]() { yield "x"; } }) });
  const eofIterator = (await eofClient.fieldworkEvents({}, { timeoutMs: 20, signal: eofSignal.signal }))[Symbol.asyncIterator]();
  assert.deepEqual([...((await eofIterator.next()).value)], [...new TextEncoder().encode("x")]);
  assert.equal((await eofIterator.next()).done, true);
  assert.equal(eofSignal.removed(), 1);
  const errorSignal = trackedSignal();
  const errorClient = new PatchClientV3({ fetchFn: async () => response({ async *[Symbol.asyncIterator]() { yield "x"; throw new Error("async stream failed"); } }) });
  const errorIterator = (await errorClient.fieldworkEvents({}, { timeoutMs: 20, signal: errorSignal.signal }))[Symbol.asyncIterator]();
  await errorIterator.next();
  await assert.rejects(() => errorIterator.next(), /async stream failed/);
  assert.equal(errorSignal.removed(), 1);
  let abortedSignal;
  const abortSignal = trackedSignal();
  const abortClient = new PatchClientV3({ fetchFn: async (_url, init) => { abortedSignal = init.signal; return response({ async *[Symbol.asyncIterator]() { yield "x"; } }); } });
  const abortIterator = (await abortClient.fieldworkEvents({}, { timeoutMs: 20, signal: abortSignal.signal }))[Symbol.asyncIterator]();
  await abortIterator.next();
  abortSignal.abort();
  assert.equal(abortedSignal.aborted, true);
  await abortIterator.return();
  assert.equal(abortSignal.removed(), 1);
});

test("async-iterable stream cancellation destroys a blocked Node readable", async () => {
  let nextStarted;
  const nextStartedPromise = new Promise((resolve) => { nextStarted = resolve; });
  const source = new Readable({ read() { nextStarted(); } });
  source.push(Buffer.from([1]));
  const client = new PatchClientV3({ fetchFn: async () => ({ ok: true, status: 200, headers: new Headers({ "content-type": "text/event-stream" }), text: async () => "", arrayBuffer: async () => new ArrayBuffer(0), body: source }) });
  const originalReadableStream = globalThis.ReadableStream;
  try {
    globalThis.ReadableStream = undefined;
    const body = await client.fieldworkEvents();
    const iterator = body[Symbol.asyncIterator]();
    assert.deepEqual([...((await iterator.next()).value)], [1]);
    const pendingNext = iterator.next();
    await nextStartedPromise;
    await body.cancel();
    await pendingNext.catch(() => {});
    assert.equal(source.destroyed, true);
  } finally {
    globalThis.ReadableStream = originalReadableStream;
  }
});

test("async-iterable stream closes its iterator on invalid chunks", async () => {
  let closed = false;
  const source = { async *[Symbol.asyncIterator]() { try { yield {}; } finally { closed = true; } } };
  const client = new PatchClientV3({ fetchFn: async () => ({ ok: true, status: 200, headers: new Headers({ "content-type": "text/event-stream" }), text: async () => "", arrayBuffer: async () => new ArrayBuffer(0), body: source }) });
  const iterator = (await client.fieldworkEvents())[Symbol.asyncIterator]();
  await assert.rejects(() => iterator.next(), /unsupported response body chunk type/);
  assert.equal(closed, true);
});

test("native fetch sends signed downloads without auth and serializes multipart uploads", async (t) => {
  const requests = [];
  const bytes = Buffer.from([0, 255, 1, 128]);
  const server = http.createServer(async (request, response) => {
    const chunks = [];
    for await (const chunk of request) chunks.push(chunk);
    requests.push({ url: request.url, headers: request.headers, body: Buffer.concat(chunks) });
    if (request.url.startsWith("/api/v3/fieldwork/attachments/download")) {
      response.writeHead(200, { "content-type": "application/json" });
      response.end(bytes);
      return;
    }
    response.writeHead(200, { "content-type": "application/json" });
    response.end("{}");
  });
  await new Promise((resolve, reject) => { server.once("error", reject); server.listen(0, "127.0.0.1", resolve); });
  t.after(async () => {
    server.closeAllConnections?.();
    await new Promise((resolve) => server.close(resolve));
  });
  const { port } = server.address();
  const client = new PatchClientV3({
    baseUrl: `http://127.0.0.1:${port}`,
    allowInsecureHttp: true,
    accessToken: "token",
    accountType: "manager",
    defaultHeaders: { Authorization: "Bearer default", "Account-Type": "manager" },
  });
  const downloaded = await client.fieldworkAttachmentDownload(
    { work_id: "w", object_key: "o", expires: "1", signature: "s" },
    { headers: { Authorization: "Bearer override", "Account-Type": "manager" } }
  );
  await client.uploadPlantFiles("plant-1", { file: new Blob([new Uint8Array([1, 2])]), filename: "report.bin", name: "report" });
  assert.deepEqual([...downloaded], [...bytes]);
  assert.equal(requests[0].headers.authorization, undefined);
  assert.equal(requests[0].headers["account-type"], undefined);
  assert.match(requests[0].url, /work_id=w/);
  assert.match(requests[1].headers["content-type"], /^multipart\/form-data; boundary=/);
  const wire = requests[1].body.toString("latin1");
  assert.match(wire, /name="filename"; filename="report.bin"/);
  assert.match(wire, /name="name"\r\n\r\nreport\r\n/);
});
