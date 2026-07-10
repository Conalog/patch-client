import { PatchClientV3, type AuthOutputV3Body, type WeatherForecastRow } from "../src";

const client = new PatchClientV3();

const auth: Promise<AuthOutputV3Body> = client.authenticateUser({
  type: "manager",
  email: "manager@example.com",
  password: "pw",
});

void auth;
void client.getPlantList({ page: 1, size: 20, full: true });
void client.assignPlantPermission("org123", "unw4id41ud2p0wt", {
  type: "viewer",
  username: "viewer1",
});
void client.startOAuthLogin({ provider: "google", redirect_url: "myscheme://callback" });
void client.createPlantFilter("unw4id41ud2p0wt", { name: "Inverter A", map_ids: ["inv-a"] });
const forecast: Promise<WeatherForecastRow[] | null> = client.getPlantWeatherForecast(
  "unw4id41ud2p0wt",
  { days: 3 }
);
void forecast;
void client.getLatestDeviceMetrics("unw4id41ud2p0wt", { includeState: true, ago: 15 });
void client.getMetricsByDate(
  "unw4id41ud2p0wt",
  "device",
  "plant",
  "1d",
  "2024-01-24",
  { before: 1, fields: ["i_out", "p"], id: ["pnl-001"] }
);

// @ts-expect-error login type must match the OpenAPI enum
void client.authenticateUser({ type: "admin", password: "pw" });

// @ts-expect-error plant permission now requires a plantId
void client.assignPlantPermission("org123", { type: "viewer", username: "viewer1" });

// @ts-expect-error filter map_ids must be strings
void client.createPlantFilter("unw4id41ud2p0wt", { name: "x", map_ids: [1] });

// @ts-expect-error includeState must be boolean
void client.getLatestDeviceMetrics("unw4id41ud2p0wt", { includeState: "true" });

// @ts-expect-error ago must be number
void client.getLatestDeviceMetrics("unw4id41ud2p0wt", { ago: "15" });

// @ts-expect-error before must be number
void client.getMetricsByDate("unw4id41ud2p0wt", "device", "plant", "1d", "2024-01-24", { before: "1", fields: ["i_out"] });

// @ts-expect-error metric source must match the OpenAPI enum
void client.getMetricsByDate("unw4id41ud2p0wt", "bad-source", "plant", "1d", "2024-01-24");
