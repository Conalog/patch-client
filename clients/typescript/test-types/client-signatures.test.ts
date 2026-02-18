import { PatchClientV3 } from "../src";

const client = new PatchClientV3();

void client.getLatestDeviceMetrics("unw4id41ud2p0wt", { includeState: true, ago: 15 });
void client.getMetricsByDate(
  "unw4id41ud2p0wt",
  "device",
  "plant",
  "1d",
  "2024-01-24",
  { before: 1, fields: ["i_out", "p"] }
);

// @ts-expect-error includeState must be boolean
void client.getLatestDeviceMetrics("unw4id41ud2p0wt", { includeState: "true" });

// @ts-expect-error ago must be number
void client.getLatestDeviceMetrics("unw4id41ud2p0wt", { ago: "15" });

// @ts-expect-error before must be number
void client.getMetricsByDate("unw4id41ud2p0wt", "device", "plant", "1d", "2024-01-24", { before: "1", fields: ["i_out"] });
