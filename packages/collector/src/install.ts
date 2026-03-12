import { readFileSync, writeFileSync, existsSync, mkdirSync, chmodSync } from "fs";
import { join, dirname } from "path";
import { homedir } from "os";

export interface InstallOptions {
  hooksDir?: string;
  settingsPath?: string;
  uuidPath?: string;
  launchdDir?: string;
}

function preHookContent(uuidPath: string): string {
  return `#!/usr/bin/env bun
import { readFileSync, existsSync, mkdirSync, appendFileSync } from "fs";
import { homedir } from "os";
import { join, dirname } from "path";

const HOME = homedir();
const uuidPath = ${JSON.stringify(uuidPath)};
const uuid = existsSync(uuidPath) ? readFileSync(uuidPath, "utf8").trim() : "unknown";

let input = "";
const decoder = new TextDecoder();
for await (const chunk of Bun.stdin.stream()) {
  input += decoder.decode(chunk);
}

let parsed: Record<string, unknown> = {};
try { parsed = JSON.parse(input); } catch {}

const event = {
  user_uuid: uuid,
  phase: "pre",
  tool: (parsed.tool_name ?? "unknown") as string,
  ts: Date.now(),
  session_id: process.env.CLAUDE_SESSION_ID ?? "",
  project_path: process.env.CLAUDE_PROJECT_DIR ?? process.cwd(),
  input: input.slice(0, 2048),
};

const body = JSON.stringify(event);
try {
  const res = await fetch("http://localhost:51789/event", {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body,
    signal: AbortSignal.timeout(100),
  });
  if (!res.ok) throw new Error("http " + res.status);
} catch {
  const dir = join(HOME, ".sensei", uuid);
  if (!existsSync(dir)) mkdirSync(dir, { recursive: true });
  appendFileSync(join(dir, "events.jsonl"), body + "\\n");
}
`;
}

function postHookContent(uuidPath: string): string {
  return `#!/usr/bin/env bun
import { readFileSync, existsSync, mkdirSync, appendFileSync } from "fs";
import { homedir } from "os";
import { join } from "path";

const HOME = homedir();
const uuidPath = ${JSON.stringify(uuidPath)};
const uuid = existsSync(uuidPath) ? readFileSync(uuidPath, "utf8").trim() : "unknown";

let input = "";
const decoder = new TextDecoder();
for await (const chunk of Bun.stdin.stream()) {
  input += decoder.decode(chunk);
}

let parsed: Record<string, unknown> = {};
try { parsed = JSON.parse(input); } catch {}

const success = parsed.exit_code == null || parsed.exit_code === 0;
// Note: Claude's PostToolUse hook payload does not include duration_ms.
// The field is left as null (schema allows null) since the hook cannot measure it.
const event = {
  user_uuid: uuid,
  phase: "post",
  tool: (parsed.tool_name ?? "unknown") as string,
  ts: Date.now(),
  session_id: process.env.CLAUDE_SESSION_ID ?? "",
  project_path: process.env.CLAUDE_PROJECT_DIR ?? process.cwd(),
  success,
  duration_ms: null,  // not available from hook payload
  error: success ? null : String(parsed.tool_result ?? ""),
};

const body = JSON.stringify(event);
try {
  const res = await fetch("http://localhost:51789/event", {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body,
    signal: AbortSignal.timeout(100),
  });
  if (!res.ok) throw new Error("http " + res.status);
} catch {
  const dir = join(HOME, ".sensei", uuid);
  if (!existsSync(dir)) mkdirSync(dir, { recursive: true });
  appendFileSync(join(dir, "events.jsonl"), body + "\\n");
}
`;
}

function launchdPlist(bunPath: string, daemonScript: string): string {
  return `<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>Label</key>
  <string>com.sensei.collector</string>
  <key>ProgramArguments</key>
  <array>
    <string>${bunPath}</string>
    <string>${daemonScript}</string>
  </array>
  <key>RunAtLoad</key>
  <true/>
  <key>KeepAlive</key>
  <true/>
  <key>EnvironmentVariables</key>
  <dict>
    <key>SENSEI_COLLECTOR_PORT</key>
    <string>51789</string>
  </dict>
  <key>StandardOutPath</key>
  <string>${homedir()}/.sensei/collector.log</string>
  <key>StandardErrorPath</key>
  <string>${homedir()}/.sensei/collector.err</string>
</dict>
</plist>`;
}

export async function installHooks(opts: InstallOptions = {}): Promise<void> {
  const HOME = homedir();
  const hooksDir = opts.hooksDir ?? join(HOME, ".claude", "hooks");
  const settingsPath = opts.settingsPath ?? join(HOME, ".claude", "settings.json");
  const uuidPath = opts.uuidPath ?? join(HOME, ".sensei", "uuid");
  const launchdDir = opts.launchdDir ?? join(HOME, "Library", "LaunchAgents");

  mkdirSync(hooksDir, { recursive: true });
  mkdirSync(launchdDir, { recursive: true });

  // Write hook scripts
  const prePath = join(hooksDir, "sensei-pre-tool-use.ts");
  const postPath = join(hooksDir, "sensei-post-tool-use.ts");

  writeFileSync(prePath, preHookContent(uuidPath), "utf8");
  writeFileSync(postPath, postHookContent(uuidPath), "utf8");
  chmodSync(prePath, 0o755);
  chmodSync(postPath, 0o755);

  // Update settings.json
  let settings: Record<string, unknown> = {};
  if (existsSync(settingsPath)) {
    try {
      settings = JSON.parse(readFileSync(settingsPath, "utf8")) as Record<string, unknown>;
    } catch {}
  }
  const hooks = (settings.hooks as Record<string, unknown>) ?? {};
  hooks.PreToolUse = prePath;
  hooks.PostToolUse = postPath;
  settings.hooks = hooks;
  mkdirSync(dirname(settingsPath), { recursive: true });
  writeFileSync(settingsPath, JSON.stringify(settings, null, 2), "utf8");

  // Write launchd plist
  const bunPath = process.execPath; // path to bun binary
  const daemonScript = join(HOME, ".sensei", "collector-daemon.ts");
  const plistPath = join(launchdDir, "com.sensei.collector.plist");
  writeFileSync(plistPath, launchdPlist(bunPath, daemonScript), "utf8");
}
