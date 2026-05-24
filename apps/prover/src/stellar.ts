import { execFile } from "node:child_process";
import { promisify } from "node:util";
import { StellarIdentityClient, type InvokeContract } from "@wraith/stellar-identity-sdk";

const execFileAsync = promisify(execFile);

export type DeploymentConfig = {
  contractId: string;
  rpcUrl: string;
  networkPassphrase: string;
  network?: string;
};

export function createCliClient(deploy: DeploymentConfig): StellarIdentityClient {
  const network = deploy.network ?? process.env.STELLAR_NETWORK ?? "futurenet";
  const source = process.env.SOROBAN_SOURCE_ACCOUNT;
  if (!source) {
    throw new Error("SOROBAN_SOURCE_ACCOUNT is required for on-chain reads");
  }

  const invokeContract: InvokeContract = async (method, args) => {
    const { stdout } = await execFileAsync(
      "stellar",
      [
        "contract",
        "invoke",
        "--id",
        deploy.contractId,
        "--source-account",
        source,
        "--network",
        network,
        "--send=no",
        "--",
        method,
        ...cliArgsFromInvoke(method, args),
      ],
      { env: { ...process.env, STELLAR_RPC_URL: deploy.rpcUrl } },
    );
    return parseCliOutput(stdout);
  };

  return new StellarIdentityClient(invokeContract);
}

function cliArgsFromInvoke(method: string, args: unknown[]): string[] {
  switch (method) {
    case "is_verified": {
      const [appId, subject] = args as [string, string];
      return ["--app_id", appId, "--subject", subject];
    }
    case "get_record": {
      const [appId, subject] = args as [string, string];
      return ["--app_id", appId, "--subject", subject];
    }
    case "set_app_approval": {
      const [appId, approved] = args as [string, boolean];
      return ["--app_id", appId, "--approved", String(approved)];
    }
    case "is_app_registered": {
      const [appId] = args as [string];
      return ["--app_id", appId];
    }
    case "is_approved_app": {
      const [appId] = args as [string];
      return ["--app_id", appId];
    }
    default:
      throw new Error(`prover API read path does not map method: ${method}`);
  }
}

function parseCliOutput(stdout: string): unknown {
  const trimmed = stdout.trim();
  if (trimmed === "true") return true;
  if (trimmed === "false") return false;
  if (trimmed === "null" || trimmed === "") return null;
  try {
    return JSON.parse(trimmed);
  } catch {
    return trimmed.replace(/^"|"$/g, "");
  }
}
