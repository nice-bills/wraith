import Fastify from "fastify";
import { readFileSync } from "node:fs";
import { join, dirname } from "node:path";
import { fileURLToPath } from "node:url";
import type { StellarIdentityClient } from "@wraith/stellar-identity-sdk";
import { createCliClient, type DeploymentConfig } from "./stellar.js";

const __dirname = dirname(fileURLToPath(import.meta.url));
const ROOT = join(__dirname, "../../..");

const STABLE_APP_ID = process.env.STABLE_APP_ID ?? "wraith";
const PORT = Number(process.env.PORT ?? 8787);

const pending = new Map<string, { wallet: string; claims: Record<string, unknown>; at: string }>();

export function loadDeployment(): DeploymentConfig {
  const path = process.env.DEPLOYMENT_JSON ?? join(ROOT, "deployments/futurenet.json");
  const raw = JSON.parse(readFileSync(path, "utf8")) as DeploymentConfig & { network?: string };
  return {
    contractId: process.env.CONTRACT_ID ?? raw.contractId,
    rpcUrl: process.env.RPC_URL ?? raw.rpcUrl,
    networkPassphrase:
      process.env.NETWORK_PASSPHRASE ??
      raw.networkPassphrase ??
      "Test SDF Future Network ; October 2022",
    network: process.env.STELLAR_NETWORK ?? raw.network,
  };
}

export type ProverDeps = {
  client?: StellarIdentityClient;
  deployment?: DeploymentConfig;
};

export async function buildServer(deps: ProverDeps = {}) {
  const deploy = deps.deployment ?? loadDeployment();
  const client = deps.client ?? createCliClient(deploy);

  const app = Fastify({ logger: false });

  app.get("/health", async () => ({
    ok: true,
    contractId: deploy.contractId,
    stableAppId: STABLE_APP_ID,
  }));

  app.get<{ Params: { wallet: string }; Querystring: { appId?: string } }>(
    "/verify/:wallet",
    async (req) => {
      const appId = req.query.appId ?? STABLE_APP_ID;
      const verified = await client.isVerified(appId, req.params.wallet);
      const record = verified
        ? await client.getRecord(appId, req.params.wallet)
        : null;
      return { appId, wallet: req.params.wallet, verified, record };
    },
  );

  app.post<{ Body: { appId: string; approved: boolean } }>(
    "/admin/approve",
    async (req) => {
      const { appId, approved } = req.body ?? {};
      if (!appId) {
        return app.status(400).send({ error: "appId required" });
      }
      await client.setAppApproval(appId, approved ?? true);
      return { appId, approved: approved ?? true };
    },
  );

  app.post<{
    Body: { wallet: string; age?: number; country_code?: number; is_human?: boolean };
  }>("/webhook/kyc", async (req) => {
    const { wallet, age, country_code, is_human } = req.body ?? {};
    if (!wallet) {
      return app.status(400).send({ error: "wallet required" });
    }
    const id = `${wallet}:${Date.now()}`;
    pending.set(id, {
      wallet,
      claims: { age, country_code, is_human },
      at: new Date().toISOString(),
    });
    return { status: "pending", id, next: "./scripts/attested-ready.sh" };
  });

  app.get("/admin/pending", async () => ({
    count: pending.size,
    items: [...pending.entries()].map(([id, v]) => ({ id, ...v })),
  }));

  return app;
}

async function main() {
  const app = await buildServer();
  await app.listen({ port: PORT, host: "0.0.0.0" });
}

if (process.argv[1] === fileURLToPath(import.meta.url)) {
  main().catch((err) => {
    console.error(err);
    process.exit(1);
  });
}
