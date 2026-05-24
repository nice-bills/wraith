import { readFileSync } from "node:fs";
import { join, dirname } from "node:path";
import { fileURLToPath } from "node:url";

const __dirname = dirname(fileURLToPath(import.meta.url));
export const ROOT = join(__dirname, "../../..");

export type StableAppConfig = {
  appId: string;
  description?: string;
  policy: {
    min_age: number;
    require_humanity: boolean;
    excluded_countries: number[];
    expiration_window: number;
    sanctions_enabled: boolean;
    claim_layout: string;
  };
};

export function loadStableAppConfig(): StableAppConfig {
  const path =
    process.env.STABLE_APP_JSON ?? join(ROOT, "deployments/stable-app.json");
  return JSON.parse(readFileSync(path, "utf8")) as StableAppConfig;
}

export function stableAppId(): string {
  return process.env.STABLE_APP_ID ?? loadStableAppConfig().appId;
}

export function autoSubmitAttested(): boolean {
  return process.env.AUTO_SUBMIT_ATTESTED === "1";
}
