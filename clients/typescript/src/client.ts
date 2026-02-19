export type AccountType = "viewer" | "manager" | "admin";

export type QueryValue =
  | string
  | number
  | boolean
  | null
  | undefined
  | Array<string | number | boolean | null | undefined>;
export type JsonObject = Record<string, unknown>;

export interface ClientConfig {
  baseUrl?: string;
  accessToken?: string;
  accountType?: AccountType;
  defaultHeaders?: Record<string, string>;
  fetchFn?: FetchFn;
  allowInsecureHttp?: boolean;
  maxResponseBytes?: number;
}

export interface AbortSignalLike {
  readonly aborted: boolean;
  addEventListener(type: "abort", listener: () => void, options?: { once?: boolean }): void;
  removeEventListener(type: "abort", listener: () => void): void;
}

export interface RequestOptions {
  accessToken?: string;
  accountType?: AccountType;
  headers?: Record<string, string>;
  signal?: AbortSignalLike;
  timeoutMs?: number;
}

type UploadFormData = { append(name: string, value: unknown, fileName?: string): unknown };
type FetchResponseHeaders = { get(name: string): string | null };
type FetchResponse = {
  ok: boolean;
  status: number;
  headers: FetchResponseHeaders;
  text(): Promise<string>;
  arrayBuffer(): Promise<ArrayBuffer>;
  body?: {
    getReader?: () => {
      read: () => Promise<{ done: boolean; value?: Uint8Array }>;
      cancel?: () => Promise<void>;
    };
    cancel?: () => Promise<void>;
    destroy?: (error?: Error) => void;
    [Symbol.asyncIterator]?: () => AsyncIterator<unknown>;
  } | null;
};
type FetchInit = {
  method?: string;
  headers?: Record<string, string>;
  body?: unknown;
  signal?: AbortSignalLike;
  redirect?: "follow" | "error" | "manual";
};
type FetchFn = (input: string, init?: FetchInit) => Promise<FetchResponse>;

interface RequestInput {
  query?: Record<string, QueryValue>;
  body?: unknown;
  formData?: UploadFormData;
  options?: RequestOptions;
}

export class PatchClientError extends Error {
  readonly status: number;
  readonly payload: unknown;
  readonly method?: string;
  readonly url?: string;

  constructor(
    status: number,
    payload: unknown,
    message?: string,
    context?: { method?: string; url?: string }
  ) {
    super(message ?? `PATCH API request failed with status ${status}`);
    this.status = status;
    this.payload = payload;
    this.method = context?.method;
    this.url = context?.url;
  }
}

export class PatchClientV3 {
  private readonly baseUrl: string;
  private readonly fetchFn: FetchFn;
  private readonly defaultHeaders: Record<string, string>;
  private readonly maxResponseBytes: number;
  private accessToken?: string;
  private accountType?: AccountType;

  constructor(config: ClientConfig = {}) {
    const normalizedBaseUrl = (config.baseUrl ?? "https://patch-api.conalog.com").replace(/\/$/, "");
    // Validate base URL at construction time to fail fast on invalid config.
    // URL instances are serialized back to string and used for path joining per request.
    const parsedBaseUrl = new URL(normalizedBaseUrl);
    if (
      parsedBaseUrl.protocol === "http:" &&
      !config.allowInsecureHttp &&
      !isLoopbackHost(parsedBaseUrl.hostname)
    ) {
      throw new Error("insecure http baseUrl requires allowInsecureHttp=true");
    }
    if (parsedBaseUrl.search || parsedBaseUrl.hash) {
      throw new Error("baseUrl must not include query or fragment");
    }
    if (parsedBaseUrl.username || parsedBaseUrl.password) {
      throw new Error("baseUrl must not include credentials");
    }
    this.baseUrl = parsedBaseUrl.toString().replace(/\/$/, "");
    this.accessToken = config.accessToken;
    this.accountType = config.accountType;
    this.defaultHeaders = { ...(config.defaultHeaders ?? {}) };
    if (config.maxResponseBytes === Number.POSITIVE_INFINITY) {
      this.maxResponseBytes = Number.POSITIVE_INFINITY;
    } else {
      this.maxResponseBytes =
        typeof config.maxResponseBytes === "number" &&
        Number.isFinite(config.maxResponseBytes) &&
        config.maxResponseBytes > 0
          ? config.maxResponseBytes
          : 10 << 20;
    }

    if (config.fetchFn) {
      this.fetchFn = config.fetchFn;
    } else if (typeof globalThis !== "undefined" && typeof globalThis.fetch === "function") {
      this.fetchFn = globalThis.fetch as unknown as FetchFn;
    } else {
      throw new Error("fetch is not available. Provide fetchFn in ClientConfig.");
    }
  }

  setAccessToken(token?: string): void {
    this.accessToken = token;
  }

  setAccountType(accountType?: AccountType): void {
    this.accountType = accountType;
  }

  async authenticateUser(payload: JsonObject): Promise<unknown> {
    return this.request("POST", "/api/v3/account/auth-with-password", { body: payload });
  }

  async refreshUserToken(options?: RequestOptions): Promise<unknown> {
    return this.request("POST", "/api/v3/account/refresh-token", { options });
  }

  async getAccountInfo(options?: RequestOptions): Promise<unknown> {
    return this.request("GET", "/api/v3/account/", { options });
  }

  async createOrganizationMember(
    organizationId: string,
    payload: JsonObject,
    options?: RequestOptions
  ): Promise<unknown> {
    return this.request("POST", `/api/v3/organizations/${encodePath(organizationId)}/members`, {
      body: payload,
      options,
    });
  }

  async assignPlantPermission(
    organizationId: string,
    payload: JsonObject,
    options?: RequestOptions
  ): Promise<unknown> {
    return this.request(
      "POST",
      `/api/v3/organizations/${encodePath(organizationId)}/permissions`,
      {
        body: payload,
        options,
      }
    );
  }

  async getPlantList(
    query?: { page?: number; size?: number },
    options?: RequestOptions
  ): Promise<unknown> {
    return this.request("GET", "/api/v3/plants", { query, options });
  }

  async createPlant(payload: JsonObject, options?: RequestOptions): Promise<unknown> {
    return this.request("POST", "/api/v3/plants", { body: payload, options });
  }

  async getPlantDetails(plantId: string, options?: RequestOptions): Promise<unknown> {
    return this.request("GET", `/api/v3/plants/${encodePath(plantId)}`, { options });
  }

  async getPlantBlueprint(
    plantId: string,
    date: string,
    options?: RequestOptions
  ): Promise<unknown> {
    return this.request("GET", `/api/v3/plants/${encodePath(plantId)}/blueprint`, {
      query: { date },
      options,
    });
  }

  async uploadPlantFiles(
    plantId: string,
    formData: UploadFormData,
    options?: RequestOptions
  ): Promise<unknown> {
    return this.request("POST", `/api/v3/plants/${encodePath(plantId)}/files`, {
      formData,
      options,
    });
  }

  async getAssetHealthLevel(
    plantId: string,
    unit: string,
    date: string,
    view?: string,
    options?: RequestOptions
  ): Promise<unknown> {
    return this.request(
      "GET",
      `/api/v3/plants/${encodePath(plantId)}/indicator/health-level/${encodePath(unit)}`,
      {
        query: { date, view },
        options,
      }
    );
  }

  async getPanelSeqnum(plantId: string, date: string, options?: RequestOptions): Promise<unknown> {
    return this.request("GET", `/api/v3/plants/${encodePath(plantId)}/indicator/seqnum`, {
      query: { date },
      options,
    });
  }

  async listInverterLogs(
    plantId: string,
    query?: { page?: number; size?: number },
    options?: RequestOptions
  ): Promise<unknown> {
    return this.request("GET", `/api/v3/plants/${encodePath(plantId)}/logs/inverter`, {
      query,
      options,
    });
  }

  async listInverterLogsById(
    plantId: string,
    inverterId: string,
    query?: { page?: number; size?: number },
    options?: RequestOptions
  ): Promise<unknown> {
    return this.request(
      "GET",
      `/api/v3/plants/${encodePath(plantId)}/logs/inverters/${encodePath(inverterId)}`,
      {
        query,
        options,
      }
    );
  }

  async getLatestDeviceMetrics(
    plantId: string,
    query?: { includeState?: boolean; ago?: number },
    options?: RequestOptions
  ): Promise<unknown> {
    return this.request("GET", `/api/v3/plants/${encodePath(plantId)}/metrics/device/latest`, {
      query,
      options,
    });
  }

  async getLatestInverterMetrics(plantId: string, options?: RequestOptions): Promise<unknown> {
    return this.request("GET", `/api/v3/plants/${encodePath(plantId)}/metrics/inverter/latest`, {
      options,
    });
  }

  async getMetricsByDate(
    plantId: string,
    source: string,
    unit: string,
    interval: string,
    date: string,
    query?: { before?: number; fields?: string[] },
    options?: RequestOptions
  ): Promise<unknown> {
    return this.request(
      "GET",
      `/api/v3/plants/${encodePath(plantId)}/metrics/${encodePath(source)}/${encodePath(unit)}-${encodePath(interval)}`,
      {
        query: { date, before: query?.before, fields: query?.fields?.join(",") },
        options,
      }
    );
  }

  async getAssetRegistrationOnPlant(
    plantId: string,
    recordType: string,
    date: string,
    query?: { asset_id?: string; map_id?: string },
    options?: RequestOptions
  ): Promise<unknown> {
    return this.request(
      "GET",
      `/api/v3/plants/${encodePath(plantId)}/registry/${encodePath(recordType)}`,
      {
        query: { date, asset_id: query?.asset_id, map_id: query?.map_id },
        options,
      }
    );
  }

  private async request(method: string, path: string, input: RequestInput = {}): Promise<unknown> {
    const url = new URL(this.baseUrl);
    const basePath = url.pathname.endsWith("/") ? url.pathname.slice(0, -1) : url.pathname;
    url.pathname = `${basePath}${path}`;
    const query = input.query ?? {};
    for (const [key, value] of Object.entries(query)) {
      if (value === undefined || value === null) {
        continue;
      }
      if (Array.isArray(value)) {
        for (const item of value) {
          if (item !== undefined && item !== null) {
            url.searchParams.append(key, String(item));
          }
        }
      } else {
        url.searchParams.set(key, String(value));
      }
    }

    const headers = mergeHeadersCaseInsensitive(
      { Accept: "application/json" },
      this.defaultHeaders,
      this.authHeaders(input.options),
      input.options?.headers
    );

    const init: FetchInit = { method, headers };

    if (input.formData) {
      init.body = input.formData as unknown;
      deleteHeaderCaseInsensitive(headers, "content-type");
    } else if (input.body !== undefined) {
      deleteHeaderCaseInsensitive(headers, "content-type");
      headers["Content-Type"] = "application/json";
      try {
        init.body = JSON.stringify(input.body);
      } catch (err) {
        throw new PatchClientError(
          0,
          { error: err instanceof Error ? err.message : String(err) },
          undefined,
          { method, url: url.toString() }
        );
      }
    }
    if (hasHeaderCaseInsensitive(headers, "authorization") || init.body !== undefined) {
      // Prevent credential leakage on 30x redirects for auth-bearing or body-bearing requests.
      init.redirect = "manual";
    }

    const { signal, cleanup, timeoutSupported } = createRequestSignal(
      input.options?.signal,
      input.options?.timeoutMs
    );
    if (signal) {
      init.signal = signal;
    }

    try {
      const response = await this.fetchWithTimeout(
        url.toString(),
        init,
        input.options?.timeoutMs,
        timeoutSupported
      );
      let payload: unknown;
      try {
        payload = await parseResponse(response, this.maxResponseBytes);
      } catch (err) {
        const parseErrorPayload = { error: err instanceof Error ? err.message : String(err) };
        if (isAbortOrTimeoutError(err)) {
          const timeoutErr = new PatchClientError(0, parseErrorPayload, undefined, {
            method,
            url: url.toString(),
          });
          (timeoutErr as Error & { cause?: unknown }).cause = err;
          throw timeoutErr;
        }
        const parseErr = new PatchClientError(response.status, parseErrorPayload, undefined, {
          method,
          url: url.toString(),
        });
        (parseErr as Error & { cause?: unknown }).cause = err;
        throw parseErr;
      }

      if (!response.ok) {
        throw new PatchClientError(response.status, payload, undefined, {
          method,
          url: url.toString(),
        });
      }

      return payload;
    } catch (err) {
      if (err instanceof PatchClientError) {
        throw err;
      }
      const networkError = new PatchClientError(
        0,
        null,
        `PATCH API request failed: ${method} ${url.toString()}`,
        { method, url: url.toString() }
      );
      (networkError as Error & { cause?: unknown }).cause = err;
      throw networkError;
    } finally {
      cleanup();
    }
  }

  private async fetchWithTimeout(
    url: string,
    init: FetchInit,
    timeoutMs?: number,
    timeoutSupported = true
  ): Promise<FetchResponse> {
    const hasTimeout = typeof timeoutMs === "number" && Number.isFinite(timeoutMs) && timeoutMs > 0;
    if (!hasTimeout) {
      return this.fetchFn(url, init);
    }
    if (!timeoutSupported) {
      throw new Error("timeoutMs requires AbortController support in this runtime");
    }
    return await new Promise<FetchResponse>((resolve, reject) => {
      const timer = setTimeout(() => {
        reject(new Error(`request timed out after ${timeoutMs}ms`));
      }, timeoutMs);
      let fetchPromise: Promise<FetchResponse>;
      try {
        fetchPromise = Promise.resolve(this.fetchFn(url, init));
      } catch (err) {
        clearTimeout(timer);
        reject(err);
        return;
      }
      fetchPromise.then(
        (resp) => {
          clearTimeout(timer);
          resolve(resp);
        },
        (err) => {
          clearTimeout(timer);
          reject(err);
        }
      );
    });
  }

  private authHeaders(options?: RequestOptions): Record<string, string> {
    const headers: Record<string, string> = {};
    const token = options?.accessToken ?? this.accessToken;
    const accountType = options?.accountType ?? this.accountType;

    if (token) {
      const normalizedToken = token.trim();
      if (normalizedToken) {
        headers.Authorization = /^bearer\s+/i.test(normalizedToken)
          ? normalizedToken
          : `Bearer ${normalizedToken}`;
      }
    }
    if (accountType) {
      headers["Account-Type"] = accountType;
    }

    return headers;
  }
}

function encodePath(value: string): string {
  return encodeURIComponent(value);
}

async function parseResponse(response: FetchResponse, maxResponseBytes: number): Promise<unknown> {
  const contentType = (response.headers.get("content-type") ?? "").toLowerCase();
  if (hasNoResponseBodyStatus(response.status)) {
    return null;
  }
  const bytes = await readResponseBytesWithLimit(response, maxResponseBytes);
  if (bytes.length === 0) {
    return null;
  }
  if (contentType.includes("application/json") || contentType.includes("+json")) {
    const text = decodeUtf8(bytes);
    try {
      return JSON.parse(text) as unknown;
    } catch {
      return text;
    }
  }
  if (
    contentType.startsWith("text/") ||
    contentType.includes("xml") ||
    contentType.includes("html")
  ) {
    return decodeUtf8(bytes);
  }
  return bytes;
}

function hasNoResponseBodyStatus(status: number): boolean {
  return (status >= 100 && status < 200) || status === 204 || status === 205 || status === 304;
}

async function readResponseBytesWithLimit(
  response: FetchResponse,
  maxResponseBytes: number
): Promise<Uint8Array> {
  if (!Number.isFinite(maxResponseBytes)) {
    return new Uint8Array(await response.arrayBuffer());
  }

  const contentLengthHeader = response.headers.get("content-length");
  let parsedLength: number | null = null;
  if (contentLengthHeader) {
    parsedLength = Number(contentLengthHeader);
    if (Number.isFinite(parsedLength) && parsedLength > maxResponseBytes) {
      await cancelResponseBody(response);
      throw new Error(`response exceeded ${maxResponseBytes} bytes`);
    }
  }

  if (response.body && typeof response.body.getReader === "function") {
    const reader = response.body.getReader();
    const chunks: Uint8Array[] = [];
    let total = 0;
    while (true) {
      const { done, value } = await reader.read();
      if (done) {
        break;
      }
      const chunk = value ?? new Uint8Array();
      total += chunk.byteLength;
      if (total > maxResponseBytes) {
        await safeCancelReader(reader);
        throw new Error(`response exceeded ${maxResponseBytes} bytes`);
      }
      chunks.push(chunk);
    }
    return concatChunks(chunks, total);
  }

  if (
    response.body &&
    typeof response.body[Symbol.asyncIterator] === "function"
  ) {
    const chunks: Uint8Array[] = [];
    let total = 0;
    for await (const value of response.body as AsyncIterable<unknown>) {
      const chunk = toUint8Array(value);
      total += chunk.byteLength;
      if (total > maxResponseBytes) {
        await cancelResponseBody(response, new Error(`response exceeded ${maxResponseBytes} bytes`));
        throw new Error(`response exceeded ${maxResponseBytes} bytes`);
      }
      chunks.push(chunk);
    }
    return concatChunks(chunks, total);
  }

  if (!Number.isFinite(parsedLength)) {
    throw new Error("response size cannot be bounded in this runtime (missing streaming body)");
  }

  const bytes = new Uint8Array(await response.arrayBuffer());
  if (bytes.length > maxResponseBytes) {
    throw new Error(`response exceeded ${maxResponseBytes} bytes`);
  }
  return bytes;
}

function toUint8Array(value: unknown): Uint8Array {
  if (value instanceof Uint8Array) {
    return value;
  }
  if (value instanceof ArrayBuffer) {
    return new Uint8Array(value);
  }
  if (ArrayBuffer.isView(value)) {
    return new Uint8Array(value.buffer, value.byteOffset, value.byteLength);
  }
  if (typeof value === "string") {
    return new TextEncoder().encode(value);
  }
  throw new Error(`unsupported response body chunk type: ${describeChunkType(value)}`);
}

function describeChunkType(value: unknown): string {
  if (value === null) {
    return "null";
  }
  if (value === undefined) {
    return "undefined";
  }
  if (typeof value === "object" && value.constructor?.name) {
    return value.constructor.name;
  }
  return typeof value;
}

async function safeCancelReader(reader: { cancel?: () => Promise<void> }): Promise<void> {
  try {
    await reader.cancel?.();
  } catch {
    // Swallow cancellation errors to preserve original size-limit failure.
  }
}

async function cancelResponseBody(response: FetchResponse, reason?: Error): Promise<void> {
  const body = response.body;
  if (!body) {
    return;
  }
  try {
    if (typeof body.cancel === "function") {
      await body.cancel();
      return;
    }
    if (typeof body.destroy === "function") {
      body.destroy(reason);
    }
  } catch {
    // Best effort only.
  }
}

function concatChunks(chunks: Uint8Array[], total: number): Uint8Array {
  const out = new Uint8Array(total);
  let offset = 0;
  for (const chunk of chunks) {
    out.set(chunk, offset);
    offset += chunk.byteLength;
  }
  return out;
}

function decodeUtf8(bytes: Uint8Array): string {
  const decoder = new TextDecoder("utf-8");
  return decoder.decode(bytes);
}

function deleteHeaderCaseInsensitive(headers: Record<string, string>, targetKey: string): void {
  const normalizedTarget = targetKey.toLowerCase();
  for (const key of Object.keys(headers)) {
    if (key.toLowerCase() === normalizedTarget) {
      delete headers[key];
    }
  }
}

function hasHeaderCaseInsensitive(headers: Record<string, string>, targetKey: string): boolean {
  const normalizedTarget = targetKey.toLowerCase();
  for (const key of Object.keys(headers)) {
    if (key.toLowerCase() === normalizedTarget) {
      return true;
    }
  }
  return false;
}

function isAbortOrTimeoutError(err: unknown): boolean {
  if (!(err instanceof Error)) {
    return false;
  }
  const name = (err.name ?? "").toLowerCase();
  const message = (err.message ?? "").toLowerCase();
  return (
    name.includes("abort") ||
    name.includes("timeout") ||
    message.includes("abort") ||
    message.includes("timed out") ||
    message.includes("timeout")
  );
}

function mergeHeadersCaseInsensitive(
  ...sources: Array<Record<string, string> | undefined>
): Record<string, string> {
  const merged: Array<[string, string]> = [];
  const positionByLowerKey = new Map<string, number>();

  for (const source of sources) {
    if (!source) {
      continue;
    }
    for (const [key, value] of Object.entries(source)) {
      const normalized = key.toLowerCase();
      const existing = positionByLowerKey.get(normalized);
      if (existing === undefined) {
        positionByLowerKey.set(normalized, merged.length);
        merged.push([key, value]);
      } else {
        merged[existing] = [key, value];
      }
    }
  }

  return Object.fromEntries(merged);
}

function isLoopbackHost(hostname: string): boolean {
  const normalized = hostname.toLowerCase().replace(/^\[/, "").replace(/\]$/, "");
  return (
    normalized === "localhost" ||
    normalized === "127.0.0.1" ||
    normalized === "::1" ||
    normalized === "0:0:0:0:0:0:0:1"
  );
}

function createRequestSignal(
  externalSignal?: AbortSignalLike,
  timeoutMs?: number
): { signal?: AbortSignalLike; cleanup: () => void; timeoutSupported: boolean } {
  const hasTimeout = typeof timeoutMs === "number" && Number.isFinite(timeoutMs) && timeoutMs > 0;
  if (!externalSignal && !hasTimeout) {
    return { cleanup: () => {}, timeoutSupported: true };
  }

  if (!hasTimeout && externalSignal) {
    return { signal: externalSignal, cleanup: () => {}, timeoutSupported: true };
  }

  const AbortControllerCtor = (globalThis as {
    AbortController?: new () => { signal: AbortSignalLike; abort(): void };
  }).AbortController;
  if (!AbortControllerCtor) {
    return { signal: externalSignal, cleanup: () => {}, timeoutSupported: false };
  }

  const controller = new AbortControllerCtor();
  let timeoutId: ReturnType<typeof setTimeout> | undefined;
  let onExternalAbort: (() => void) | undefined;

  if (externalSignal) {
    if (externalSignal.aborted) {
      controller.abort();
    } else {
      onExternalAbort = () => controller.abort();
      externalSignal.addEventListener("abort", onExternalAbort, { once: true });
    }
  }

  if (hasTimeout) {
    timeoutId = setTimeout(() => {
      controller.abort();
    }, timeoutMs);
  }

  return {
    signal: controller.signal,
    timeoutSupported: true,
    cleanup: () => {
      if (timeoutId !== undefined) {
        clearTimeout(timeoutId);
      }
      if (externalSignal && onExternalAbort) {
        externalSignal.removeEventListener("abort", onExternalAbort);
      }
    },
  };
}
