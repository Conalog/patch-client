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
void client.listOAuthMethods({ provider: "google", redirect_url: "myscheme://callback" });
void client.getOAuth2LoginUrl("google", "myscheme://callback");
void client.listCombinerModelInfo();
void client.listInverterModelInfo();
void client.listModuleModelInfo();
void client.getDeviceState("unw4id41ud2p0wt", "2024-01-24", "seqnum");
void client.getPlantRegistryStat("unw4id41ud2p0wt", "2024-01-24");

// @ts-expect-error includeState must be boolean
void client.getLatestDeviceMetrics("unw4id41ud2p0wt", { includeState: "true" });

// @ts-expect-error ago must be number
void client.getLatestDeviceMetrics("unw4id41ud2p0wt", { ago: "15" });

// @ts-expect-error before must be number
void client.getMetricsByDate("unw4id41ud2p0wt", "device", "plant", "1d", "2024-01-24", { before: "1", fields: ["i_out"] });

// @ts-expect-error redirect_url must be string
void client.listOAuthMethods({ redirect_url: 123 });

// @ts-expect-error kind must be one of seqnum/relay/rsd
void client.getDeviceState("unw4id41ud2p0wt", "2024-01-24", "offline");
