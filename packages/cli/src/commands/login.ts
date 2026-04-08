// packages/cli/src/commands/login.ts
import { writeFile, mkdir, readFile, unlink } from "fs/promises";
import { existsSync } from "fs";
import { execFile } from "child_process";
import { join } from "path";
import { homedir } from "os";
import yaml from "js-yaml";

const CRED_DIR = join(homedir(), ".config", "sensei");
const CRED_PATH = join(CRED_DIR, "credentials.yaml");

export interface PlatformCredentials {
  access_token: string;
  refresh_token: string;
  email: string;
  account_id: string;
  account_slug: string;
  account_type: string;
  role: string;
}

export async function loadPlatformCredentials(): Promise<PlatformCredentials | null> {
  try {
    const raw = await readFile(CRED_PATH, "utf-8");
    const parsed = yaml.load(raw) as Record<string, unknown>;
    if (!parsed?.access_token) return null;
    return parsed as PlatformCredentials;
  } catch {
    return null;
  }
}

export async function savePlatformCredentials(creds: PlatformCredentials): Promise<void> {
  await mkdir(CRED_DIR, { recursive: true });
  await writeFile(CRED_PATH, yaml.dump(creds), "utf-8");
}

export async function clearPlatformCredentials(): Promise<void> {
  if (existsSync(CRED_PATH)) {
    const raw = await readFile(CRED_PATH, "utf-8");
    const parsed = yaml.load(raw) as Record<string, unknown>;
    // Preserve supabase_service_key if present, remove platform auth fields
    if (parsed?.supabase_service_key) {
      await writeFile(CRED_PATH, yaml.dump({ supabase_service_key: parsed.supabase_service_key }), "utf-8");
    } else {
      await unlink(CRED_PATH);
    }
  }
}

function openBrowser(url: string): void {
  const cmd = process.platform === "darwin" ? "open"
    : process.platform === "win32" ? "cmd"
    : "xdg-open";
  const args = process.platform === "win32" ? ["/c", "start", url] : [url];
  execFile(cmd, args, { timeout: 3000 }, () => {/* ignore errors — user can open manually */});
}

export async function loginCommand(platformUrl?: string): Promise<void> {
  const baseUrl = platformUrl ?? process.env.SENSEI_PLATFORM_URL ?? "https://app.sensei.dev";
  const redirectPort = 7788;
  const redirectUri = `http://localhost:${redirectPort}/callback`;
  const loginUrl = `${baseUrl}/connect/cli?redirect_uri=${encodeURIComponent(redirectUri)}`;

  console.log("Opening browser for login…");
  console.log(`\nIf your browser didn't open, visit:\n  ${loginUrl}\n`);
  openBrowser(loginUrl);

  // Wait for callback on loopback
  const { createServer } = await import("http");
  const token = await new Promise<string>((resolve, reject) => {
    const timeout = setTimeout(() => {
      server.close();
      reject(new Error("Login timed out after 5 minutes"));
    }, 5 * 60 * 1000);

    const server = createServer((req, res) => {
      const reqUrl = new URL(req.url ?? "/", `http://localhost:${redirectPort}`);
      const accessToken = reqUrl.searchParams.get("access_token");
      if (accessToken) {
        clearTimeout(timeout);
        res.writeHead(200, { "Content-Type": "text/html" });
        res.end("<html><body><h2>Login successful! You can close this tab.</h2></body></html>");
        server.close();
        resolve(accessToken);
      } else {
        res.writeHead(400);
        res.end("Missing access_token");
        clearTimeout(timeout);
        server.close();
        reject(new Error("No access_token in callback"));
      }
    });

    server.listen(redirectPort, "127.0.0.1");
  });

  // Resolve account context from platform API
  const verifyRes = await fetch(`${baseUrl}/api/connect/verify`, {
    headers: { Authorization: `Bearer ${token}` },
  });
  if (!verifyRes.ok) throw new Error(`Auth verification failed: ${verifyRes.status}`);

  const info = await verifyRes.json() as {
    userId: string; email: string;
    accountId: string; accountSlug: string; accountType: string; role: string;
    refreshToken?: string;
  };

  const creds: PlatformCredentials = {
    access_token: token,
    refresh_token: info.refreshToken ?? "",
    email: info.email,
    account_id: info.accountId,
    account_slug: info.accountSlug,
    account_type: info.accountType,
    role: info.role,
  };

  await savePlatformCredentials(creds);
  console.log(`\nLogged in as ${creds.email} | ${creds.account_slug} (${creds.account_type}) | ${creds.role}`);
}

export async function logoutCommand(): Promise<void> {
  const creds = await loadPlatformCredentials();
  if (!creds) {
    console.log("Not logged in.");
    return;
  }
  await clearPlatformCredentials();
  console.log(`Logged out (${creds.email}).`);
}

export async function whoamiCommand(): Promise<void> {
  const creds = await loadPlatformCredentials();
  if (!creds) {
    console.log("Not logged in. Run `sensei login` to authenticate.");
    return;
  }
  console.log(`${creds.email} | ${creds.account_slug} (${creds.account_type}) | ${creds.role}`);
}
