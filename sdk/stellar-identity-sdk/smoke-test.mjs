import {
  StellarIdentityClient,
  createSorobanRpcInvoke,
} from "./dist/index.js";

const CONTRACT_ID = "CBH7JX2HH2ZEKH4QGBTUTPBWRXJD5WRO3YDHUVJ2VM7Y6YXD26US5452";
const FUTURENET_RPC = "https://rpc-futurenet.stellar.org:443";

async function main() {
  console.log("=== Wraith Identity Core — Futurenet Smoke Test ===\n");

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

  console.log("3. Calling isAppRegistered('wave')...");
  const registered = await identity.isAppRegistered("wave");
  console.log(`   Registered: ${registered}`);
  console.log(`   ✓ App registration check: ${registered}\n`);

  console.log("4. Calling getPolicy('wave')...");
  const policy = await identity.getPolicy("wave");
  console.log(`   Policy:`, JSON.stringify(policy, null, 4));
  console.log(`   ✓ Policy retrieved\n`);

  console.log("5. Calling registerApp(new policy)...");
  const newPolicy = {
    owner: admin,
    minAge: 21,
    requireHumanity: true,
    sanctionsRoot: "0x" + "a".repeat(64),
    excludedCountries: [840],
    expirationWindow: 0,
    sanctionsEnabled: false,
  };
  const appId = await identity.registerApp(newPolicy);
  console.log(`   App ID: ${appId}`);
  console.log(`   ✓ New app registered\n`);

  console.log("6. Calling isAppRegistered(returned appId)...");
  const dripsRegistered = await identity.isAppRegistered(appId);
  console.log(`   Registered: ${dripsRegistered}`);
  console.log(`   ✓ Second app check: ${dripsRegistered}\n`);

  console.log("=== All smoke tests passed! ===");
}

main().catch((err) => {
  console.error("Smoke test failed:", err);
  process.exit(1);
});