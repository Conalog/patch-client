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
  fetchFn?: typeof fetch;
}

export interface RequestOptions {
  accessToken?: string;
  accountType?: AccountType;
  headers?: Record<string, string>;
}

type UploadFormData = FormData | { append(name: string, value: unknown, fileName?: string): unknown };

interface RequestInput {
  query?: Record<string, QueryValue>;
  body?: unknown;
  formData?: UploadFormData;
  options?: RequestOptions;
}

export class PatchClientError extends Error {
  readonly status: number;
  readonly payload: unknown;

  constructor(status: number, payload: unknown, message?: string) {
    super(message ?? `PATCH API request failed with status ${status}`);
    this.status = status;
    this.payload = payload;
  }
}

export class PatchClientV3 {
  private readonly baseUrl: string;
  private readonly fetchFn: typeof fetch;
  private readonly defaultHeaders: Record<string, string>;
  private accessToken?: string;
  private accountType?: AccountType;

  constructor(config: ClientConfig = {}) {
    const normalizedBaseUrl = (config.baseUrl ?? "https://patch-api.conalog.com").replace(/\/$/, "");
    // Validate base URL at construction time to fail fast on invalid config.
    // URL instances are serialized back to string and used for path joining per request.
    this.baseUrl = new URL(normalizedBaseUrl).toString().replace(/\/$/, "");
    this.accessToken = config.accessToken;
    this.accountType = config.accountType;
    this.defaultHeaders = { ...(config.defaultHeaders ?? {}) };

    if (config.fetchFn) {
      this.fetchFn = config.fetchFn;
    } else if (typeof fetch !== "undefined") {
      this.fetchFn = fetch;
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
        query: { date, before: query?.before, fields: query?.fields },
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
    const url = new URL(`${this.baseUrl}${path}`);
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

    const headers: Record<string, string> = {
      Accept: "application/json",
      ...this.defaultHeaders,
      ...this.authHeaders(input.options),
      ...(input.options?.headers ?? {}),
    };

    const init: RequestInit = { method, headers };

    if (input.formData) {
      init.body = input.formData as unknown as BodyInit;
      delete headers["Content-Type"];
    } else if (input.body !== undefined) {
      headers["Content-Type"] = "application/json";
      init.body = JSON.stringify(input.body);
    }

    const response = await this.fetchFn(url.toString(), init);
    const payload = await parseResponse(response);

    if (!response.ok) {
      throw new PatchClientError(response.status, payload);
    }

    return payload;
  }

  private authHeaders(options?: RequestOptions): Record<string, string> {
    const headers: Record<string, string> = {};
    const token = options?.accessToken ?? this.accessToken;
    const accountType = options?.accountType ?? this.accountType;

    if (token) {
      headers.Authorization = token.startsWith("Bearer ") ? token : `Bearer ${token}`;
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

async function parseResponse(response: Response): Promise<unknown> {
  const contentType = response.headers.get("content-type") ?? "";
  if (response.status === 204) {
    return null;
  }
  const text = await response.text();
  if (text.length === 0) {
    return null;
  }
  if (contentType.includes("application/json") || contentType.includes("+json")) {
    try {
      return JSON.parse(text) as unknown;
    } catch {
      return text;
    }
  }
  return text;
}
