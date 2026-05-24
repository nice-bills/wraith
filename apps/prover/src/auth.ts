import type { FastifyRequest } from "fastify";

export class WebhookAuthError extends Error {
  statusCode = 401;
}

/** Optional shared secret for KYC webhooks (`x-wraith-webhook-secret` or `Authorization: Bearer …`). */
export function verifyWebhookSecret(req: FastifyRequest): void {
  const secret = process.env.PROVER_WEBHOOK_SECRET;
  if (!secret) return;

  const header = req.headers["x-wraith-webhook-secret"];
  if (typeof header === "string" && header === secret) return;

  const auth = req.headers.authorization;
  if (typeof auth === "string" && auth === `Bearer ${secret}`) return;

  throw new WebhookAuthError("invalid webhook secret");
}
