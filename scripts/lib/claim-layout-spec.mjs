import { readFileSync } from "node:fs";
import { join, dirname } from "node:path";
import { fileURLToPath } from "node:url";
import { ageFromBirthYymmdd } from "./country-codes.mjs";

const __dirname = dirname(fileURLToPath(import.meta.url));
const SPEC_PATH = join(__dirname, "../../deployments/claim-layout-spec.json");

let cached;
export function loadClaimLayoutSpec() {
  if (!cached) {
    cached = JSON.parse(readFileSync(SPEC_PATH, "utf8"));
  }
  return cached;
}

/** Derive claims from decimal public signals per spec layout name. */
export function deriveClaimsFromSpec(layoutName, signals, currentYymmdd) {
  const spec = loadClaimLayoutSpec();
  const layout = spec.layouts[layoutName];
  if (!layout) throw new Error(`unknown layout: ${layoutName}`);

  if (layoutName === "standard") {
    return {
      age: Number(signals[layout.ageIndex]),
      country_code: Number(signals[layout.countryIndex]),
      is_human: Number(signals[layout.humanIndex]) !== 0,
    };
  }

  if (layoutName === "rarimoLayoutStub") {
    const birth = Number(signals[layout.birthDateIndex]);
    const country = Number(signals[layout.countryIndex]);
    return {
      age: ageFromBirthYymmdd(String(birth), String(currentYymmdd)),
      country_code: country,
      is_human: true,
    };
  }

  if (layoutName === "rarimoPhase2") {
    const birth = Number(signals[layout.birthDateIndex]);
    const nationality = Number(signals[layout.nationalityIndex]);
    const citizenship = Number(signals[layout.citizenshipIndex]);
    const country = nationality !== 0 ? nationality : citizenship;
    return {
      age: ageFromBirthYymmdd(String(birth), String(currentYymmdd)),
      country_code: country,
      is_human: true,
    };
  }

  throw new Error(`unsupported layout: ${layoutName}`);
}
