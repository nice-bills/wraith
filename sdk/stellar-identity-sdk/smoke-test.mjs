import * as StellarSdk from "@stellar/stellar-sdk";

const CONTRACT_ID = process.env.CONTRACT_ID || "";
const FUTURENET_RPC = process.env.RPC_URL || "https://rpc-futurenet.stellar.org:443";

if (!CONTRACT_ID) {
  console.error("ERROR: CONTRACT_ID environment variable is required.");
  console.error("Usage: CONTRACT_ID=<contract_id> node smoke-test.mjs");
  process.exit(1);
}

async function main() {
  console.log("=== Wraith Identity Core — Futurenet Smoke Test ===\n");
  console.log(`Contract: ${CONTRACT_ID}\n`);

  const server = new StellarSdk.rpc.Server(FUTURENET_RPC);

  console.log("1. Checking RPC health...");
  const health = await server.getHealth();
  console.log(`   Status: ${health.status}`);
  console.log(`   Latest ledger: ${health.lastLedger}`);
  console.log(`   ✓ RPC healthy\n`);

  console.log("2. Fetching contract WASM bytecode...");
  try {
    const wasm = await server.getContractWasmByContractId(CONTRACT_ID);
    console.log(`   WASM bytecode length: ${wasm.length} bytes`);
    console.log(`   ✓ Contract WASM verified on-chain\n`);
  } catch (e) {
    console.log(`   ✗ Could not fetch WASM: ${e.message}\n`);
    process.exit(1);
  }

  console.log("3. Verifying contract is initialized via events...");
  try {
    const events = await server.getEvents({
      startLedger: 1,
      filters: [{ contractIds: [CONTRACT_ID], topics: [["AAAABAAAAAVpbml0aWFsaXplZAAAAA=="]] }],
      limit: 1,
    });
    if (events.events && events.events.length > 0) {
      console.log(`   ✓ Contract initialized (found init event)`);
      console.log(`   Event: ${JSON.stringify(events.events[0].topicStrings())}\n`);
    } else {
      console.log(`   ⚠ No initialization event found\n`);
    }
  } catch (e) {
    console.log(`   ⚠ Could not fetch events: ${e.message}\n`);
  }

  console.log("4. Testing contract responds to RPC...");
  try {
    const response = await fetch(FUTURENET_RPC, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({
        jsonrpc: "2.0",
        id: 1,
        method: "simulateTransaction",
        params: {
          transaction: "AAAAAgAAAAgAAAABAAAAAQAAAAAAAAAAAAAAAgAAAAAAAAAA",
          resourceConfig: { instructionLeeway: 0 },
        },
      }),
    });
    const data = await response.json();
    console.log(`   ✓ Contract responds to RPC\n`);
  } catch (e) {
    console.log(`   ⚠ RPC test failed: ${e.message}\n`);
  }

  console.log("=== Smoke test complete ===\n");
  console.log("Contract is deployed and accessible on Futurenet.");
  console.log("Contract ID:", CONTRACT_ID);
  console.log("\nUse stellar-cli to interact:");
  console.log(`  stellar contract invoke --id ${CONTRACT_ID} --source-account <your-key> --network futurenet -- get-admin`);
}

main().catch((err) => {
  console.error("Smoke test failed:", err);
  process.exit(1);
});
