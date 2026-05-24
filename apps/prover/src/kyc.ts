/** Attested claims shape (matches contract + scripts). */
export type AttestedClaimsJson = {
  age: number;
  country_code: number;
  is_human: boolean;
};

export type KycMapping = {
  wallet: string;
  claims: AttestedClaimsJson;
  vendor?: string;
};

export type KycIgnore = {
  status: "ignored";
  reason: string;
};

export type KycResult = KycMapping | KycIgnore;

function pickWallet(obj: Record<string, unknown>): string | null {
  const keys = [
    "wallet",
    "stellar_address",
    "stellarAddress",
    "subject",
    "externalUserId",
    "external_user_id",
    "user_id",
  ];
  for (const k of keys) {
    const v = obj[k];
    if (typeof v === "string" && v.startsWith("G") && v.length >= 56) return v;
  }
  const meta = obj.metadata;
  if (meta && typeof meta === "object" && !Array.isArray(meta)) {
    return pickWallet(meta as Record<string, unknown>);
  }
  const fields = obj.fields;
  if (fields && typeof fields === "object" && !Array.isArray(fields)) {
    for (const k of ["stellar-wallet", "stellar_wallet", "wallet"]) {
      const f = (fields as Record<string, unknown>)[k];
      if (f && typeof f === "object" && f !== null && "value" in f) {
        const val = (f as { value?: unknown }).value;
        if (typeof val === "string" && val.startsWith("G")) return val;
      }
    }
  }
  return null;
}

function pickCountryCode(obj: Record<string, unknown>): number | null {
  if (typeof obj.country_code === "number") return obj.country_code;
  if (typeof obj.countryCode === "number") return obj.countryCode;
  const nat = obj.nationality ?? obj.country;
  if (typeof nat === "string" && /^\d+$/.test(nat)) return Number(nat);
  return null;
}

function pickAge(obj: Record<string, unknown>): number | null {
  if (typeof obj.age === "number") return obj.age;
  if (typeof obj.age === "string" && /^\d+$/.test(obj.age)) return Number(obj.age);
  return null;
}

/** Generic webhook: `{ wallet, age, country_code, is_human }` or nested under `claims`. */
export function mapGenericWebhook(body: unknown): KycResult {
  if (!body || typeof body !== "object") {
    return { status: "ignored", reason: "empty body" };
  }
  const root = body as Record<string, unknown>;
  const claimsSrc =
    root.claims && typeof root.claims === "object"
      ? (root.claims as Record<string, unknown>)
      : root;
  const wallet = pickWallet(root) ?? pickWallet(claimsSrc);
  if (!wallet) {
    return { status: "ignored", reason: "missing stellar wallet (G…)" };
  }
  const age = pickAge(claimsSrc) ?? pickAge(root);
  const country = pickCountryCode(claimsSrc) ?? pickCountryCode(root);
  if (age == null || country == null) {
    return { status: "ignored", reason: "missing age or country_code" };
  }
  const isHuman = claimsSrc.is_human ?? claimsSrc.isHuman ?? root.is_human ?? true;
  return {
    wallet,
    vendor: "generic",
    claims: {
      age,
      country_code: country,
      is_human: Boolean(isHuman),
    },
  };
}

/** Persona inquiry approved — wallet in metadata / custom fields. */
export function mapPersonaWebhook(body: unknown): KycResult {
  if (!body || typeof body !== "object") {
    return { status: "ignored", reason: "empty body" };
  }
  const root = body as Record<string, unknown>;
  const data = root.data;
  if (!data || typeof data !== "object") {
    return mapGenericWebhook(body);
  }
  const attrs = (data as Record<string, unknown>).attributes;
  if (!attrs || typeof attrs !== "object") {
    return { status: "ignored", reason: "persona: no attributes" };
  }
  const status = (attrs as Record<string, unknown>).status;
  if (status !== "approved" && status !== "completed") {
    return { status: "ignored", reason: `persona: status=${String(status)}` };
  }
  const fields = (attrs as Record<string, unknown>).fields;
  const meta = (attrs as Record<string, unknown>).metadata;
  const merged: Record<string, unknown> = {
    ...(typeof meta === "object" && meta ? (meta as Record<string, unknown>) : {}),
    fields,
  };
  const wallet = pickWallet(merged) ?? pickWallet(attrs as Record<string, unknown>);
  if (!wallet) {
    return { status: "ignored", reason: "persona: missing wallet in metadata/fields" };
  }
  const age =
    pickAge(attrs as Record<string, unknown>) ??
    pickAge(merged) ??
    Number(process.env.KYC_DEFAULT_AGE ?? 25);
  const country =
    pickCountryCode(attrs as Record<string, unknown>) ??
    pickCountryCode(merged) ??
    Number(process.env.KYC_DEFAULT_COUNTRY ?? 840);
  return {
    wallet,
    vendor: "persona",
    claims: { age, country_code: country, is_human: true },
  };
}

/** Sumsub applicantReviewed GREEN — `externalUserId` is the Stellar address. */
export function mapSumsubWebhook(body: unknown): KycResult {
  if (!body || typeof body !== "object") {
    return { status: "ignored", reason: "empty body" };
  }
  const root = body as Record<string, unknown>;
  const type = root.type ?? root.reviewStatus;
  if (type !== "applicantReviewed" && type !== "applicantReviewedCompleted") {
    return { status: "ignored", reason: `sumsub: type=${String(type)}` };
  }
  const review = root.reviewResult ?? root.review;
  const answer =
    review && typeof review === "object"
      ? (review as Record<string, unknown>).reviewAnswer ??
        (review as Record<string, unknown>).reviewStatus
      : root.reviewAnswer;
  if (answer !== "GREEN" && answer !== "completed") {
    return { status: "ignored", reason: `sumsub: review=${String(answer)}` };
  }
  const wallet = pickWallet(root);
  if (!wallet) {
    return { status: "ignored", reason: "sumsub: externalUserId must be G… address" };
  }
  const info = root.info;
  const infoObj =
    info && typeof info === "object" ? (info as Record<string, unknown>) : {};
  const age =
    pickAge(infoObj) ??
    pickAge(root) ??
    Number(process.env.KYC_DEFAULT_AGE ?? 25);
  const country =
    pickCountryCode(infoObj) ??
    pickCountryCode(root) ??
    Number(process.env.KYC_DEFAULT_COUNTRY ?? 840);
  return {
    wallet,
    vendor: "sumsub",
    claims: { age, country_code: country, is_human: true },
  };
}
