import { describe, it } from "node:test";
import assert from "node:assert/strict";
import { buildServer } from "./index.js";
import { StellarIdentityClient } from "@wraith/stellar-identity-sdk";

function mockClient(): StellarIdentityClient {
  return new StellarIdentityClient(async (method) => {
    if (method === "is_verified") return false;
    if (method === "get_record") return null;
    if (method === "set_app_approval") return undefined;
    throw new Error(`unexpected method ${method}`);
  });
}

describe("prover API", () => {
  it("GET /health returns contract metadata", async () => {
    const app = await buildServer({
      deployment: {
        contractId: "CDILCFGJMHXUU2SXNICUVMNFFTAXKL6ULNJ44NEPGMNIGFOA44O3F22W",
        rpcUrl: "https://rpc-futurenet.stellar.org:443",
        networkPassphrase: "Test SDF Future Network ; October 2022",
      },
      client: mockClient(),
    });
    const res = await app.inject({ method: "GET", url: "/health" });
    assert.equal(res.statusCode, 200);
    const body = res.json();
    assert.equal(body.ok, true);
    assert.ok(body.contractId);
    await app.close();
  });

  it("GET /verify uses mock client", async () => {
    const app = await buildServer({ client: mockClient() });
    const res = await app.inject({
      method: "GET",
      url: "/verify/GDYLBKGXBZ4QWVKZGGEWEZ2CY4BNVULZ62JNI6JYGOENLNH6I5RC5SA5",
    });
    assert.equal(res.statusCode, 200);
    assert.equal(res.json().verified, false);
    await app.close();
  });
});
