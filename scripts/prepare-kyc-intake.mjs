#!/usr/bin/env node
/**
 * Stage photo-based ID intake for KYC (no chip). Creates gitignored manifest under passport-data/intake/.
 *
 * Usage:
 *   node scripts/prepare-kyc-intake.mjs --front id-front.jpg [--back id-back.jpg] [--selfie selfie.jpg]
 */
import { copyFileSync, mkdirSync, writeFileSync } from "node:fs";
import { basename, join } from "node:path";

const args = process.argv.slice(2);
const files = { front: null, back: null, selfie: null };

for (let i = 0; i < args.length; i++) {
  switch (args[i]) {
    case "--front":
      files.front = args[++i];
      break;
    case "--back":
      files.back = args[++i];
      break;
    case "--selfie":
      files.selfie = args[++i];
      break;
    default:
      console.error("Unknown option:", args[i]);
      process.exit(1);
  }
}

if (!files.front) {
  console.error(
    "Usage: prepare-kyc-intake.mjs --front <photo> [--back <photo>] [--selfie <photo>]"
  );
  process.exit(1);
}

const stamp = new Date().toISOString().replace(/[:.]/g, "-").slice(0, 19);
const root = join(process.cwd(), "passport-data", "intake", stamp);
mkdirSync(root, { recursive: true });

const photos = [];
for (const [role, src] of Object.entries(files)) {
  if (!src) continue;
  const dest = join(root, `${role}-${basename(src)}`);
  copyFileSync(src, dest);
  photos.push({ role, path: dest.replace(process.cwd() + "/", "") });
}

const manifest = {
  intakeType: "kyc-photo",
  createdAt: new Date().toISOString(),
  photos,
  nextSteps: [
    "Send manifest + photos to your KYC vendor (Persona, Sumsub, manual review)",
    "On approval, map result to { age, country_code, is_human }",
    "Run: ./scripts/attested-ready.sh <claims.json>",
  ],
  claimsTemplate: {
    age: 0,
    country_code: 0,
    is_human: true,
  },
};

const manifestPath = join(root, "manifest.json");
writeFileSync(manifestPath, JSON.stringify(manifest, null, 2) + "\n");
console.log("Intake dir:", root.replace(process.cwd() + "/", ""));
console.log("Manifest:  ", manifestPath.replace(process.cwd() + "/", ""));
console.log("");
console.log("Next: complete KYC off-chain, then:");
console.log("  node scripts/build-attested-claims.mjs --out passport-data/intake/claims.json ...");
console.log("  ./scripts/attested-ready.sh passport-data/intake/claims.json");
