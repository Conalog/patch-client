/**
 * Account role sent to the API via the `Account-Type` request header.
 */
export type AccountType = "viewer" | "manager" | "temporary";
export type MemberAccountType = "manager" | "viewer";
export type RegistryAssetType = "device" | "inverter" | "edge" | "panel" | "panel_group" | "sensor";
export type RegistryMapType =
  | "device"
  | "string"
  | "edge"
  | "inverter"
  | "combiner"
  | "panel"
  | "panel_group"
  | "tracker"
  | "sensor";
export type MetricSource = "device" | "inverter" | "ess" | "sensor";
export type MetricUnit =
  | "panel"
  | "inverter"
  | "ess"
  | "string"
  | "plant"
  | "temperature"
  | "insolation";
export type MetricInterval = "5m" | "15m" | "1h" | "1d" | "1M" | "1y";

/**
 * Allowed query parameter values.
 *
 * Arrays are encoded as repeated key/value pairs in the query string.
 */
export type QueryValue =
  | string
  | number
  | boolean
  | null
  | undefined
  | Array<string | number | boolean | null | undefined>;

/**
 * Generic JSON object payload used by this client.
 */
export type JsonObject = Record<string, unknown>;

export interface AuthWithPasswordBody {
  type: AccountType;
  password: string;
  email?: string;
  username?: string;
}

export interface AuthBody {
  readonly "$schema"?: string;
  name: string;
  token: string;
}

export interface OrganizationBody {
  id: string;
  name: string;
  icon?: string;
  logo?: string;
}

export interface OrgInfo extends OrganizationBody {
  owner?: string;
  updated?: string;
}

export interface AccountOutputBody {
  readonly "$schema"?: string;
  type: AccountType;
  name: string;
  organizations: OrganizationBody[] | null;
  email?: string;
  username?: string;
  metadata?: unknown;
}

export interface AuthOutputV3Body extends AccountOutputBody {
  token: string;
}

export interface AuthProvider {
  name: string;
  state: string;
  codeChallenge: string;
  codeChallengeMethod: string;
  authUrl: string;
}

export interface AuthMethodsBody {
  readonly "$schema"?: string;
  authProviders: AuthProvider[] | null;
}

export interface CreateOrganizationMemberRequestBody {
  type: MemberAccountType;
  name: string;
  email?: string;
  username?: string;
  metadata?: unknown;
}

export interface CreateAccountOutputBody extends AccountOutputBody {
  id: string;
  expired?: string;
  link?: string;
}

export interface OrganizationPermissionRequestBody {
  type: AccountType;
  email?: string;
  username?: string;
}

export type AssignOrganizationPermissionRequestBody = OrganizationPermissionRequestBody;
export type RemoveOrganizationPermissionRequestBody = OrganizationPermissionRequestBody;

export interface OrganizationPermissionOutputBody extends OrganizationPermissionRequestBody {
  readonly "$schema"?: string;
  plant_id: string;
}

export type OrgAddPermissionOutputBody = OrganizationPermissionOutputBody;
export type OrgRemovePermissionOutputBody = OrganizationPermissionOutputBody;

export interface CreatePlantInput {
  name: string;
  organizationId: string;
  metadata?: JsonObject;
}

export interface PlantBody {
  readonly "$schema"?: string;
  created: string;
  id: string;
  images: string[] | null;
  metadata: JsonObject;
  name: string;
  organization: string;
  organizationData: OrgInfo;
  updated: string;
  refPlant?: string;
}

export interface PlantBodyV3 extends Omit<PlantBody, "organization" | "organizationData"> {
  organization: OrgInfo;
}

export interface PlantsListV3OutputBody {
  readonly "$schema"?: string;
  totalPages: number;
  totalItems: number;
  page: number;
  perPage: number;
  items: PlantBodyV3[] | null;
}

export type CombinerItem = JsonObject;
export type InverterItem = JsonObject;
export type ModuleItem = JsonObject;

export interface ListOutputCombinerItemBody {
  readonly "$schema"?: string;
  items: CombinerItem[] | null;
}

export interface ListOutputInverterItemBody {
  readonly "$schema"?: string;
  items: InverterItem[] | null;
}

export interface ListOutputModuleItemBody {
  readonly "$schema"?: string;
  items: ModuleItem[] | null;
}

export interface BlueprintWriteBody {
  date: string;
  data: unknown;
  metadata?: unknown;
}

export interface BlueprintListItem {
  id: string;
  date: string;
  updated: string;
  created?: string;
}

export interface BlueprintRecordPayload extends BlueprintListItem {
  readonly "$schema"?: string;
  plant: string;
  metadata: unknown;
  data: unknown;
}

export interface CommentUserOutput {
  id: string;
  name: string;
  username: string;
  email: string;
}

export interface CommentActionBody {
  text: string;
  images?: string[];
  map_ids?: string[];
  related?: string;
}

export interface CommentEditBody {
  text?: string;
  images?: string[];
  map_ids?: string[];
  related?: string;
}

export interface CommentStateBody {
  transition: "resolve" | "reopen" | "archive" | "restore";
}

export interface CommentReadOutput {
  id: string;
  text: string;
  user: CommentUserOutput;
  created: string;
  updated: string;
  images?: string[] | null;
  map_ids?: string[] | null;
  parent?: string;
  related?: string;
  resolved?: string;
}

export interface CommentOutput extends CommentReadOutput {
  readonly "$schema"?: string;
  expand?: JsonObject;
}

export interface CreateFilterBody {
  name: string;
  map_ids: string[];
}

export interface RenameFilterBody {
  name: string;
}

export interface FilterOutput {
  readonly "$schema"?: string;
  id: string;
  name: string;
  map_ids: string[] | null;
  updated: string;
  created?: string;
  condition?: unknown;
}

export type FilterListItem = Omit<FilterOutput, "$schema">;

export interface AnomalyQuery {
  date: string;
  map_id?: string;
  map_type?: string;
  type?: string;
  severity?: string;
}

export interface RecordBody {
  id: string;
  plant_id: string;
  map_type: string;
  map_id: string;
  type: string;
  severity: string;
  detected: string;
  resolved: string;
}

export interface DeviceStateQuery {
  date: string;
  fields?: string[];
}

export interface DeviceStateRow {
  id: string;
  timestamp: number;
  date: string;
  is_forced_rapid_shutdown: boolean;
  is_forced_ref: boolean;
  is_forced_relay: boolean;
  is_rapid_shutdown: boolean;
  is_relay: boolean;
}

export interface DeviceStateBody {
  plant_id: string;
  date: string;
  data: DeviceStateRow[];
}

export interface HealthLevelCategory {
  count: number;
  ids?: string[];
}

export interface HealthLevelBody {
  readonly "$schema"?: string;
  best: HealthLevelCategory;
  caution: HealthLevelCategory;
  faulty: HealthLevelCategory;
}

export interface InverterLogsResponse {
  readonly "$schema"?: string;
  totalPages: number;
  totalSizes: number;
  page: number;
  perPage: number;
  items: InverterLogItem[] | null;
}

export interface InverterLogItem {
  plantId: string;
  level: string;
  inverterId: string;
  timestamp: string;
  message: unknown;
  raw: unknown;
}

export interface LatestDeviceBody {
  timestamp: string;
  asset_id: string;
  asset_type: string;
  map_id: string;
  map_type: string;
  edge_id: string;
  metrics: {
    i_out: number;
    v_in: number;
    v_out: number;
    temp: number;
  };
  state: Record<string, boolean>;
}

export interface InverterDataBody {
  timestamp: string;
  asset_id: string;
  asset_type: string;
  map_id: string;
  map_type: string;
  edge_id: string;
  plant_id: string;
  model: string;
  data: JsonObject;
}

export interface MetricsByDateQuery {
  before?: number;
  fields?: string[];
  id?: string[];
}

export interface MetricsBody {
  plant_id: string;
  unit: string;
  source: string;
  date: string;
  interval: string;
  data: JsonObject[] | null;
  before?: number;
}

export interface RegistryQuery {
  date?: string;
  asset_id?: string;
  map_id?: string;
  asset_type?: string;
  map_type?: string;
}

export interface RegistryMeta {
  fielder_name?: string;
  fielder_org?: string;
}

export interface RegistryOutputBody {
  asset_id: string;
  asset_type: RegistryAssetType;
  map_id: string;
  map_type: RegistryMapType;
  asset_model: JsonObject;
  registered: string;
  tag: JsonObject | string;
  unregistered: string;
  registered_meta?: RegistryMeta;
  unregistered_meta?: RegistryMeta;
}

export interface RegisterBody {
  asset_id: string;
  asset_type: RegistryAssetType;
  map_id: string;
  map_type: RegistryMapType;
  registered: string;
  asset_model?: JsonObject;
  registered_meta?: JsonObject | string;
  tag?: JsonObject | string;
}

export interface UnregisterBody {
  asset_id: string;
  asset_type: RegistryAssetType;
  map_id: string;
  map_type: RegistryMapType;
  unregistered: string;
  unregistered_meta?: JsonObject | string;
}

export interface StatModelCount {
  name: string;
  count: number;
}

export interface DeviceModelStat extends StatModelCount {
  installed_capacity_w: number;
}

export type InverterModelStat = DeviceModelStat;

export interface StatPoint {
  readonly "$schema"?: string;
  timestamp: string;
  installed_capacity_w: number;
  all_asset_models_registered: boolean;
  module_models: StatModelCount[] | null;
  device_models: DeviceModelStat[] | null;
  inverter_models: InverterModelStat[] | null;
}

export interface WeatherForecastDaily {
  img_1x?: string;
  img_2x?: string;
  img_4x?: string;
  precip_prob?: number;
  temp_max_c?: number;
  temp_min_c?: number;
  wmo_code?: number;
}

export interface WeatherForecastHour {
  time: string;
  img_1x?: string;
  img_2x?: string;
  img_4x?: string;
  precip_prob?: number;
  temp_c?: number;
  wmo_code?: number;
}

export type WeatherObservedDaily = WeatherForecastDaily;
export type WeatherObservedHour = WeatherForecastHour;

export interface WeatherForecastRow {
  local_date: string;
  daily: WeatherForecastDaily;
  hourly?: WeatherForecastHour[] | null;
}

export interface WeatherObservedRow {
  local_date: string;
  daily: WeatherObservedDaily;
  hourly?: WeatherObservedHour[] | null;
}

export interface ErrorModel {
  readonly "$schema"?: string;
  type?: string;
  title?: string;
  status?: number;
  detail?: string;
  instance?: string;
  errors?: JsonObject[] | null;
}

/**
 * Client-wide configuration for {@link PatchClientV3}.
 */
export interface ClientConfig {
  /**
   * API base URL.
   *
   * Defaults to `https://patch-api.conalog.com`.
   * The URL must not contain query string, fragment, or credentials.
   */
  baseUrl?: string;
  /**
   * Default bearer token used when `RequestOptions.accessToken` is not provided.
   */
  accessToken?: string;
  /**
   * Default account type used when `RequestOptions.accountType` is not provided.
   */
  accountType?: AccountType;
  /**
   * Headers merged into every request before per-request overrides.
   */
  defaultHeaders?: Record<string, string>;
  /**
   * Custom fetch implementation.
   *
   * Required when the runtime does not provide `globalThis.fetch`.
   */
  fetchFn?: FetchFn;
  /**
   * Allows non-loopback `http://` base URLs.
   *
   * This should only be enabled in controlled environments.
   */
  allowInsecureHttp?: boolean;
  /**
   * Maximum number of response bytes to read before failing.
   *
   * Defaults to 10 MiB. Set to `Infinity` to disable size checks.
   */
  maxResponseBytes?: number;
}

/**
 * Minimal `AbortSignal`-compatible type used by this package.
 */
export interface AbortSignalLike {
  readonly aborted: boolean;
  addEventListener(type: "abort", listener: () => void, options?: { once?: boolean }): void;
  removeEventListener(type: "abort", listener: () => void): void;
}

/**
 * Per-request overrides for auth, headers, and cancellation.
 */
export interface RequestOptions {
  /**
   * Overrides the client-level access token for one request.
   */
  accessToken?: string;
  /**
   * Overrides the client-level account type for one request.
   */
  accountType?: AccountType;
  /**
   * Additional headers merged last (highest precedence, case-insensitive).
   */
  headers?: Record<string, string>;
  /**
   * External abort signal for request cancellation.
   */
  signal?: AbortSignalLike;
  /**
   * Request timeout in milliseconds.
   *
   * Requires `AbortController` support in the current runtime.
   */
  timeoutMs?: number;
}

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
const DEFAULT_MAX_RESPONSE_BYTES = 10 << 20;

interface RequestInput {
  query?: object;
  body?: unknown;
  options?: RequestOptions;
}

export class PatchClientError extends Error {
  /**
   * HTTP status code.
   *
   * `0` indicates client-side failures such as network errors, timeouts,
   * serialization errors, or response parsing failures.
   */
  readonly status: number;
  /**
   * Parsed response payload for API failures or an internal error payload.
   */
  readonly payload: unknown;
  /**
   * HTTP method used for the failed request, when available.
   */
  readonly method?: string;
  /**
   * Full request URL for the failed request, when available.
   */
  readonly url?: string;

  /**
   * Creates a structured client error.
   */
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

/**
 * Handwritten client for PATCH Plant Data API v3.
 *
 * Every request method returns a parsed payload:
 * - JSON response: parsed JSON value
 * - text/xml/html response: `string`
 * - other content types: `Uint8Array`
 * - empty body (e.g. 204): `null`
 *
 * Request methods throw {@link PatchClientError} for API and network failures.
 */
export class PatchClientV3 {
  private readonly baseUrl: string;
  private readonly fetchFn: FetchFn;
  private readonly defaultHeaders: Record<string, string>;
  private readonly maxResponseBytes: number;
  private accessToken?: string;
  private accountType?: AccountType;

  /**
   * Creates a new API client instance.
   *
   * @param config Client configuration.
   * @throws {Error} If `baseUrl` is invalid or runtime `fetch` is unavailable.
   */
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
          : DEFAULT_MAX_RESPONSE_BYTES;
    }

    if (config.fetchFn) {
      this.fetchFn = config.fetchFn;
    } else if (typeof globalThis !== "undefined" && typeof globalThis.fetch === "function") {
      this.fetchFn = globalThis.fetch as unknown as FetchFn;
    } else {
      throw new Error("fetch is not available. Provide fetchFn in ClientConfig.");
    }
  }

  /**
   * Sets or clears the default bearer token used by subsequent requests.
   *
   * @param token Raw token or `Bearer ...` value. `undefined` clears it.
   */
  setAccessToken(token?: string): void {
    this.accessToken = token;
  }

  /**
   * Sets or clears the default account type used by subsequent requests.
   *
   * @param accountType Account type value. `undefined` clears it.
   */
  setAccountType(accountType?: AccountType): void {
    this.accountType = accountType;
  }

  async authenticateUser(payload: AuthWithPasswordBody): Promise<AuthOutputV3Body> {
    return this.request("POST", "/api/v3/account/auth-with-password", { body: payload });
  }

  async refreshUserToken(options?: RequestOptions): Promise<AuthBody> {
    return this.request("POST", "/api/v3/account/refresh-token", { options });
  }

  async getAccountInfo(options?: RequestOptions): Promise<AccountOutputBody> {
    return this.request("GET", "/api/v3/account/", { options });
  }

  async listOAuthMethods(
    query?: { provider?: string; redirect_url?: string },
    options?: RequestOptions
  ): Promise<AuthMethodsBody> {
    return this.request("GET", "/api/v3/account/auth-methods", { query, options });
  }

  async startOAuthLogin(
    query: { provider: string; redirect_url?: string },
    options?: RequestOptions
  ): Promise<string> {
    const method = "GET";
    const url = this.buildUrl("/api/v3/account/login-with-oauth2", query);
    const headers = mergeHeadersCaseInsensitive(
      { Accept: "application/json" },
      this.defaultHeaders,
      this.authHeaders(options),
      options?.headers
    );
    const { signal, cleanup, timeoutSupported } = createRequestSignal(
      options?.signal,
      options?.timeoutMs
    );
    const init: FetchInit = { method, headers, redirect: "manual" };
    if (signal) {
      init.signal = signal;
    }

    try {
      if (hasRequestedTimeout(options) && !timeoutSupported) {
        throw new Error("timeoutMs requires AbortController support in this runtime");
      }
      const response = await this.fetchFn(url.toString(), init);
      if (response.status === 302) {
        const location = response.headers.get("location");
        if (location) {
          return location;
        }
      }
      const payload = await parseResponse(response, this.maxResponseBytes);
      throw new PatchClientError(
        response.status,
        payload,
        "PATCH API OAuth login expected a 302 response with a Location header",
        { method, url: url.toString() }
      );
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

  async listCombinerModelInfo(options?: RequestOptions): Promise<ListOutputCombinerItemBody> {
    return this.request("GET", "/api/v3/model-info/combiners", { options });
  }

  async listInverterModelInfo(options?: RequestOptions): Promise<ListOutputInverterItemBody> {
    return this.request("GET", "/api/v3/model-info/inverters", { options });
  }

  async listModuleModelInfo(options?: RequestOptions): Promise<ListOutputModuleItemBody> {
    return this.request("GET", "/api/v3/model-info/modules", { options });
  }

  async createOrganizationMember(
    organizationId: string,
    payload: CreateOrganizationMemberRequestBody,
    options?: RequestOptions
  ): Promise<CreateAccountOutputBody> {
    return this.request("POST", `/api/v3/organizations/${encodePath(organizationId)}/members`, {
      body: payload,
      options,
    });
  }

  async assignPlantPermission(
    organizationId: string,
    plantId: string,
    payload: AssignOrganizationPermissionRequestBody,
    options?: RequestOptions
  ): Promise<OrgAddPermissionOutputBody> {
    return this.request(
      "POST",
      `/api/v3/organizations/${encodePath(organizationId)}/plants/${encodePath(plantId)}/permissions/grant`,
      { body: payload, options }
    );
  }

  async removePlantPermission(
    organizationId: string,
    plantId: string,
    payload: RemoveOrganizationPermissionRequestBody,
    options?: RequestOptions
  ): Promise<OrgRemovePermissionOutputBody> {
    return this.request(
      "POST",
      `/api/v3/organizations/${encodePath(organizationId)}/plants/${encodePath(plantId)}/permissions/revoke`,
      { body: payload, options }
    );
  }

  async getPlantList(
    query?: { page?: number; size?: number; full?: boolean },
    options?: RequestOptions
  ): Promise<PlantsListV3OutputBody> {
    return this.request("GET", "/api/v3/plants", { query, options });
  }

  async createPlant(payload: CreatePlantInput, options?: RequestOptions): Promise<PlantBody> {
    return this.request("POST", "/api/v3/plants", { body: payload, options });
  }

  async getPlantDetails(plantId: string, options?: RequestOptions): Promise<PlantBodyV3> {
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

  async listPlantBlueprints(
    plantId: string,
    options?: RequestOptions
  ): Promise<BlueprintListItem[] | null> {
    return this.request("GET", `/api/v3/plants/${encodePath(plantId)}/blueprints`, { options });
  }

  async recordPlantBlueprint(
    plantId: string,
    payload: BlueprintWriteBody,
    options?: RequestOptions
  ): Promise<BlueprintRecordPayload> {
    return this.request("POST", `/api/v3/plants/${encodePath(plantId)}/blueprints/record`, {
      body: payload,
      options,
    });
  }

  async getPlantBlueprintData(
    plantId: string,
    blueprintId: string,
    options?: RequestOptions
  ): Promise<BlueprintRecordPayload> {
    return this.request(
      "GET",
      `/api/v3/plants/${encodePath(plantId)}/blueprints/${encodePath(blueprintId)}`,
      { options }
    );
  }

  async listPlantComments(
    plantId: string,
    options?: RequestOptions
  ): Promise<CommentReadOutput[] | null> {
    return this.request("GET", `/api/v3/plants/${encodePath(plantId)}/comments`, { options });
  }

  async startPlantCommentThread(
    plantId: string,
    payload: CommentActionBody,
    options?: RequestOptions
  ): Promise<CommentOutput> {
    return this.request("POST", `/api/v3/plants/${encodePath(plantId)}/comments/start_thread`, {
      body: payload,
      options,
    });
  }

  async editPlantComment(
    plantId: string,
    commentId: string,
    payload: CommentEditBody,
    options?: RequestOptions
  ): Promise<CommentOutput> {
    return this.request(
      "POST",
      `/api/v3/plants/${encodePath(plantId)}/comments/${encodePath(commentId)}/edit`,
      { body: payload, options }
    );
  }

  async replyPlantComment(
    plantId: string,
    commentId: string,
    payload: CommentActionBody,
    options?: RequestOptions
  ): Promise<CommentOutput> {
    return this.request(
      "POST",
      `/api/v3/plants/${encodePath(plantId)}/comments/${encodePath(commentId)}/reply`,
      { body: payload, options }
    );
  }

  async changePlantCommentState(
    plantId: string,
    commentId: string,
    payload: CommentStateBody,
    options?: RequestOptions
  ): Promise<CommentOutput> {
    return this.request(
      "POST",
      `/api/v3/plants/${encodePath(plantId)}/comments/${encodePath(commentId)}/state`,
      { body: payload, options }
    );
  }

  async listPlantFilters(plantId: string, options?: RequestOptions): Promise<FilterListItem[] | null> {
    return this.request("GET", `/api/v3/plants/${encodePath(plantId)}/filters`, { options });
  }

  async createPlantFilter(
    plantId: string,
    payload: CreateFilterBody,
    options?: RequestOptions
  ): Promise<FilterOutput> {
    return this.request("POST", `/api/v3/plants/${encodePath(plantId)}/filters/create`, {
      body: payload,
      options,
    });
  }

  async deletePlantFilter(
    plantId: string,
    filterId: string,
    options?: RequestOptions
  ): Promise<null> {
    return this.request(
      "DELETE",
      `/api/v3/plants/${encodePath(plantId)}/filters/${encodePath(filterId)}`,
      { options }
    );
  }

  async renamePlantFilter(
    plantId: string,
    filterId: string,
    payload: RenameFilterBody,
    options?: RequestOptions
  ): Promise<FilterOutput> {
    return this.request(
      "POST",
      `/api/v3/plants/${encodePath(plantId)}/filters/${encodePath(filterId)}/rename`,
      { body: payload, options }
    );
  }

  async getPlantAnomalyTimeline(
    plantId: string,
    date: string,
    options?: RequestOptions
  ): Promise<RecordBody[] | null> {
    return this.request("GET", `/api/v3/plants/${encodePath(plantId)}/indicator/anomaly`, {
      query: { date },
      options,
    });
  }

  async getPlantAnomalyLogs(
    plantId: string,
    query: AnomalyQuery,
    options?: RequestOptions
  ): Promise<RecordBody[] | null> {
    return this.request("GET", `/api/v3/plants/${encodePath(plantId)}/indicator/anomaly/logs`, {
      query,
      options,
    });
  }

  async filterPlantAnomalyLogs(
    plantId: string,
    query: AnomalyQuery,
    options?: RequestOptions
  ): Promise<RecordBody[] | null> {
    return this.request(
      "GET",
      `/api/v3/plants/${encodePath(plantId)}/indicator/anomaly/logs/filter`,
      { query, options }
    );
  }

  async getPlantAnomalySnapshots(
    plantId: string,
    query: AnomalyQuery,
    options?: RequestOptions
  ): Promise<RecordBody[] | null> {
    return this.request(
      "GET",
      `/api/v3/plants/${encodePath(plantId)}/indicator/anomaly/snapshots`,
      { query, options }
    );
  }

  async getDeviceState(
    plantId: string,
    query: DeviceStateQuery,
    options?: RequestOptions
  ): Promise<DeviceStateBody> {
    return this.request("GET", `/api/v3/plants/${encodePath(plantId)}/indicator/device-state`, {
      query,
      options,
    });
  }

  async getAssetHealthLevel(
    plantId: string,
    unit: string,
    date: string,
    view?: "summary" | "detail",
    options?: RequestOptions
  ): Promise<HealthLevelBody> {
    return this.request(
      "GET",
      `/api/v3/plants/${encodePath(plantId)}/indicator/health-level/${encodePath(unit)}`,
      { query: { date, view }, options }
    );
  }

  async listInverterLogs(
    plantId: string,
    query?: { page?: number; size?: number },
    options?: RequestOptions
  ): Promise<InverterLogsResponse> {
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
  ): Promise<InverterLogsResponse> {
    return this.request(
      "GET",
      `/api/v3/plants/${encodePath(plantId)}/logs/inverters/${encodePath(inverterId)}`,
      { query, options }
    );
  }

  async getLatestDeviceMetrics(
    plantId: string,
    query?: { includeState?: boolean; ago?: number },
    options?: RequestOptions
  ): Promise<LatestDeviceBody[] | null> {
    return this.request("GET", `/api/v3/plants/${encodePath(plantId)}/metrics/device/latest`, {
      query,
      options,
    });
  }

  async getLatestInverterMetrics(
    plantId: string,
    options?: RequestOptions
  ): Promise<InverterDataBody[] | null> {
    return this.request("GET", `/api/v3/plants/${encodePath(plantId)}/metrics/inverter/latest`, {
      options,
    });
  }

  async getMetricsByDate(
    plantId: string,
    source: MetricSource,
    unit: MetricUnit,
    interval: MetricInterval,
    date: string,
    query?: MetricsByDateQuery,
    options?: RequestOptions
  ): Promise<MetricsBody> {
    return this.request(
      "GET",
      `/api/v3/plants/${encodePath(plantId)}/metrics/${encodePath(source)}/${encodePath(unit)}-${encodePath(interval)}`,
      {
        query: {
          date,
          before: query?.before,
          fields: query?.fields?.join(","),
          id: query?.id?.join(","),
        },
        options,
      }
    );
  }

  async getPlantRegistryTimeline(
    plantId: string,
    date: string,
    options?: RequestOptions
  ): Promise<RegistryOutputBody[] | null> {
    return this.request("GET", `/api/v3/plants/${encodePath(plantId)}/registry`, {
      query: { date },
      options,
    });
  }

  async getPlantRegistryLogs(
    plantId: string,
    query: RegistryQuery & { date: string },
    options?: RequestOptions
  ): Promise<RegistryOutputBody[] | null> {
    return this.request("GET", `/api/v3/plants/${encodePath(plantId)}/registry/logs`, {
      query,
      options,
    });
  }

  async filterPlantRegistryLogs(
    plantId: string,
    query?: RegistryQuery,
    options?: RequestOptions
  ): Promise<RegistryOutputBody[] | null> {
    return this.request("GET", `/api/v3/plants/${encodePath(plantId)}/registry/logs/filter`, {
      query,
      options,
    });
  }

  async registerAssetToPlant(
    plantId: string,
    payload: RegisterBody,
    options?: RequestOptions
  ): Promise<string> {
    return this.request("POST", `/api/v3/plants/${encodePath(plantId)}/registry/register`, {
      body: payload,
      options,
    });
  }

  async getPlantRegistrySnapshots(
    plantId: string,
    query: RegistryQuery & { date: string },
    options?: RequestOptions
  ): Promise<RegistryOutputBody[] | null> {
    return this.request("GET", `/api/v3/plants/${encodePath(plantId)}/registry/snapshots`, {
      query,
      options,
    });
  }

  async getPlantRegistryStat(
    plantId: string,
    date: string,
    options?: RequestOptions
  ): Promise<StatPoint> {
    return this.request("GET", `/api/v3/plants/${encodePath(plantId)}/registry/stat`, {
      query: { date },
      options,
    });
  }

  async unregisterAssetFromPlant(
    plantId: string,
    payload: UnregisterBody,
    options?: RequestOptions
  ): Promise<string> {
    return this.request("POST", `/api/v3/plants/${encodePath(plantId)}/registry/unregister`, {
      body: payload,
      options,
    });
  }

  async getPlantWeatherForecast(
    plantId: string,
    query?: { days?: number },
    options?: RequestOptions
  ): Promise<WeatherForecastRow[] | null> {
    return this.request("GET", `/api/v3/plants/${encodePath(plantId)}/weather/forecast`, {
      query,
      options,
    });
  }

  async getPlantWeatherObserved(
    plantId: string,
    query: { date: string; before?: number },
    options?: RequestOptions
  ): Promise<WeatherObservedRow[] | null> {
    return this.request("GET", `/api/v3/plants/${encodePath(plantId)}/weather/observed`, {
      query,
      options,
    });
  }

  private async request<T>(method: string, path: string, input: RequestInput = {}): Promise<T> {
    const url = this.buildUrl(path, input.query);
    const headers = mergeHeadersCaseInsensitive(
      { Accept: "application/json" },
      this.defaultHeaders,
      this.authHeaders(input.options),
      input.options?.headers
    );

    const init: FetchInit = { method, headers };

    if (input.body !== undefined) {
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
      if (hasRequestedTimeout(input.options) && !timeoutSupported) {
        throw new Error("timeoutMs requires AbortController support in this runtime");
      }
      const response = await this.fetchFn(url.toString(), init);
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

      return payload as T;
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

  private buildUrl(path: string, query: object = {}): URL {
    const url = new URL(this.baseUrl);
    const basePath = url.pathname.endsWith("/") ? url.pathname.slice(0, -1) : url.pathname;
    url.pathname = `${basePath}${path}`;
    for (const [key, value] of Object.entries(query) as Array<[string, QueryValue]>) {
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
    return url;
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
  // URL pathname normalization treats "." and ".." as traversal markers.
  // Reject those exact segments to prevent escaping intended API paths.
  if (value === "." || value === "..") {
    throw new Error("path segment must not be '.' or '..'");
  }
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

function hasRequestedTimeout(options?: RequestOptions): boolean {
  return (
    typeof options?.timeoutMs === "number" &&
    Number.isFinite(options.timeoutMs) &&
    options.timeoutMs > 0
  );
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
