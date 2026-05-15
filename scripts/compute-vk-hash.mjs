#!/usr/bin/env node
/** Mirrors contract compute_vk_hash: SHA-256(alpha||beta||gamma||delta||ic...). */
import { createHash } from "node:crypto";
import { readFileSync } from "node:fs";

const payload = JSON.parse(readFileSync(process.argv[2], "utf8"));
const vk = payload.verification_key;

function strip0x(h) {
  return Buffer.from(h.replace(/^0x/, ""), "hex");
}

const parts = [
  vk.alpha,
  vk.beta,
  vk.gamma,
  vk.delta,
  ...vk.ic,
].map(strip0x);

const hash = createHash("sha256").update(Buffer.concat(parts)).digest("hex");
process.stdout.write(hash);
