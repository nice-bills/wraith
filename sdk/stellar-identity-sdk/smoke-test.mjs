import {
  StellarIdentityClient,
  createSorobanRpcInvoke,
} from "./dist/index.js";

const CONTRACT_ID = process.env.CONTRACT_ID || "";
const FUTURENET_RPC = process.env.RPC_URL || "https://rpc-futurenet.stellar.org:443";

if (!CONTRACT_ID) {
  console.error("ERROR: CONTRACT_ID environment variable is required.");
  console.error("Usage: CONTRACT_ID=<contract_id> node smoke-test.mjs");
  process.exit(1);
}

async function main() {
  console.log("=== Wraith Identity Core — Futurenet Smoke Test (read-only) ===\n");
  console.log(`Contract: ${CONTRACT_ID}\n`);

  const transport = async (payload) => {
    const res = await fetch(FUTURENET_RPC, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(payload),
    });
    const json = await res.json();
    return json;
  };

  const identity = new StellarIdentityClient(
    createSorobanRpcInvoke(CONTRACT_ID, transport)
  );

  console.log("1. Calling getAdmin()...");
  const admin = await identity.getAdmin();
  console.log(`   Admin: ${admin}`);
  console.log(`   ✓ getAdmin returned: ${admin.slice(0, 8)}...\n`);

  console.log("2. Calling getProver()...");
  const prover = await identity.getProver();
  console.log(`   Prover: ${prover}`);
  console.log(`   ✓ getProver returned: ${prover.slice(0, 8)}...\n`);

  console.log("3. Calling isAppRegistered('test-app')...");
  const registered = await identity.isAppRegistered("test-app");
  console.log(`   Registered: ${registered}`);
  console.log(`   ✓ App registration check: ${registered}\n`);

  console.log("4. Calling getPolicy('test-app')...");
  const policy = await identity.getPolicy("test-app");
  if (policy) {
    console.log(`   Policy:`, JSON.stringify(policy, null, 4));
    console.log(`   ✓ Policy retrieved\n`);
  } else {
    console.log(`   No policy found (app not registered)\n`);
  }

  console.log("=== All smoke tests passed! ===");
  console.log("\nNOTE: This smoke test is read-only. For write operations,");
  console.log("use the deploy script or stellar-cli directly with proper auth.");
}

main().catch((err) => {
  console.error("Smoke test failed:", err);
  process.exit(1);
});