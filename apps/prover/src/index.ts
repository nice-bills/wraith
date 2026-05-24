import Fastify from "fastify";
import { readFileSync } from "node:fs";
import { join, dirname } from "node:path";
import { fileURLToPath } from "node:url";
import type { StellarIdentityClient } from "@wraith/stellar-identity-sdk";
import { prepareAttestedRecord, submitAttestedRecord } from "./attested.js";
import { verifyWebhookSecret, WebhookAuthError } from "./auth.js";
import { autoSubmitAttested, loadStableAppConfig, stableAppId } from "./config.js";
import { normalizeAttestedClaims } from "./country-code.js";
import {
  mapGenericWebhook,
  mapPersonaWebhook,
  mapSumsubWebhook,
  type KycMapping,
  type KycResult,
} from "./kyc.js";
import { createCliClient, type DeploymentConfig } from "./stellar.js";

const __dirname = dirname(fileURLToPath(import.meta.url));
const ROOT = join(__dirname, "../../..");

export type PendingKyc = KycMapping & {
  id: string;
  at: string;
  prepared?: Awaited<ReturnType<typeof prepareAttestedRecord>>;
};

const pending = new Map<string, PendingKyc>();

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

async function ingestKyc(
  mapped: KycMapping,
  opts: { autoSubmit: boolean },
): Promise<Record<string, unknown>> {
  const id = `${mapped.wallet}:${Date.now()}`;
  const prepared = await prepareAttestedRecord({
    subject: mapped.wallet,
    claims: mapped.claims,
    skipRegister: true,
  });
  const entry: PendingKyc = {
    ...mapped,
    id,
    at: new Date().toISOString(),
    prepared,
  };
  pending.set(id, entry);

  if (opts.autoSubmit) {
    const submitted = await submitAttestedRecord({
      subject: mapped.wallet,
      claims: mapped.claims,
      nullifier: prepared.nullifier,
      skipRegister: true,
    });
    pending.delete(id);
    return { status: "verified", vendor: mapped.vendor, ...submitted, prepared };
  }

  return {
    status: "pending",
    id,
    vendor: mapped.vendor,
    wallet: mapped.wallet,
    claims: mapped.claims,
    prepared,
    next: "POST /admin/submit-attested { \"id\": \"…\" } (requires SOROBAN_SOURCE_ACCOUNT)",
  };
}

function isMapping(result: KycResult): result is KycMapping {
  return "wallet" in result && "claims" in result;
}

export async function buildServer(deps: ProverDeps = {}) {
  const deploy = deps.deployment ?? loadDeployment();
  const appId = stableAppId();
  const stable = loadStableAppConfig();
  const client = deps.client ?? createCliClient(deploy);

  const app = Fastify({ logger: false });

  app.setErrorHandler((err, _req, reply) => {
    if (err instanceof WebhookAuthError) {
      return reply.status(401).send({ error: err.message });
    }
    throw err;
  });

  app.get("/health", async () => ({
    ok: true,
    contractId: deploy.contractId,
    stableAppId: appId,
    autoSubmit: autoSubmitAttested(),
  }));

  app.get("/config/stable-app", async () => stable);

  app.get<{ Params: { wallet: string }; Querystring: { appId?: string } }>(
    "/verify/:wallet",
    async (req) => {
      const queryApp = req.query.appId ?? appId;
      const verified = await client.isVerified(queryApp, req.params.wallet);
      const record = verified
        ? await client.getRecord(queryApp, req.params.wallet)
        : null;
      return { appId: queryApp, wallet: req.params.wallet, verified, record };
    },
  );

  app.post<{ Body: { appId: string; approved: boolean } }>(
    "/admin/approve",
    async (req) => {
      const { appId: bodyApp, approved } = req.body ?? {};
      const target = bodyApp ?? appId;
      if (!target) {
        return app.status(400).send({ error: "appId required" });
      }
      await client.setAppApproval(target, approved ?? true);
      return { appId: target, approved: approved ?? true };
    },
  );

  app.post<{
    Body: {
      wallet: string;
      age?: number;
      country_code?: number | string;
      country?: string;
      is_human?: boolean;
    };
  }>("/webhook/kyc", async (req) => {
    verifyWebhookSecret(req);
    const mapped = mapGenericWebhook(req.body);
    if (!isMapping(mapped)) return mapped;
    return ingestKyc(mapped, { autoSubmit: autoSubmitAttested() });
  });

  app.post("/webhook/persona", async (req) => {
    verifyWebhookSecret(req);
    const mapped = mapPersonaWebhook(req.body);
    if (!isMapping(mapped)) return mapped;
    return ingestKyc(mapped, { autoSubmit: autoSubmitAttested() });
  });

  app.post("/webhook/sumsub", async (req) => {
    verifyWebhookSecret(req);
    const mapped = mapSumsubWebhook(req.body);
    if (!isMapping(mapped)) return mapped;
    return ingestKyc(mapped, { autoSubmit: autoSubmitAttested() });
  });

  app.post<{
    Body: {
      wallet: string;
      age: number;
      country_code?: number | string;
      country?: string;
      is_human?: boolean;
      submit?: boolean;
    };
  }>("/attested/prepare", async (req) => {
    const { wallet, age, country_code, country, is_human, submit } = req.body ?? {};
    if (!wallet || age == null) {
      return app.status(400).send({
        error: "wallet and age required; country as country_code or country (ISO)",
      });
    }
    let claims: { age: number; country_code: number; is_human: boolean };
    try {
      claims = normalizeAttestedClaims({
        age,
        country_code: country_code ?? country,
        is_human,
      });
    } catch (e) {
      const msg = e instanceof Error ? e.message : String(e);
      return app.status(400).send({ error: msg });
    }
    const prepared = await prepareAttestedRecord({
      subject: wallet,
      claims,
      skipRegister: true,
    });
    if (submit) {
      const submitted = await submitAttestedRecord({
        subject: wallet,
        claims,
        nullifier: prepared.nullifier,
        skipRegister: true,
      });
      return { status: "verified", prepared, submitted };
    }
    return { status: "prepared", prepared };
  });

  app.post<{ Body: { id: string } }>("/admin/submit-attested", async (req) => {
    const { id } = req.body ?? {};
    if (!id) {
      return app.status(400).send({ error: "id required" });
    }
    const entry = pending.get(id);
    if (!entry) {
      return app.status(404).send({ error: "unknown pending id" });
    }
    const submitted = await submitAttestedRecord({
      subject: entry.wallet,
      claims: entry.claims,
      nullifier: entry.prepared?.nullifier,
      skipRegister: true,
    });
    pending.delete(id);
    return { status: "verified", id, ...submitted };
  });

  app.get("/admin/pending", async () => ({
    count: pending.size,
    items: [...pending.values()],
  }));

  return app;
}

async function main() {
  const port = Number(process.env.PORT ?? 8787);
  const app = await buildServer();
  await app.listen({ port, host: "0.0.0.0" });
}

if (process.argv[1] === fileURLToPath(import.meta.url)) {
  main().catch((err) => {
    console.error(err);
    process.exit(1);
  });
}
