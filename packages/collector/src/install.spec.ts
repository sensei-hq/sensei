import { describe, it, expect, beforeEach, afterEach } from "vitest";
import { mkdirSync, rmSync, existsSync, readFileSync, statSync } from "fs";
import { join } from "path";
import { tmpdir } from "os";
import { installHooks } from "./install.js";

const TMP = join(tmpdir(), `sensei-install-test-${Date.now()}`);
const HOOKS_DIR = join(TMP, "hooks");
const SETTINGS_PATH = join(TMP, "settings.json");
const UUID_PATH = join(TMP, "uuid");
const LAUNCHD_DIR = join(TMP, "LaunchAgents");

beforeEach(() => {
  mkdirSync(TMP, { recursive: true });
  mkdirSync(HOOKS_DIR, { recursive: true });
});

afterEach(() => rmSync(TMP, { recursive: true, force: true }));

describe("installHooks", () => {
  it("creates pre-tool-use and post-tool-use hook scripts", async () => {
    await installHooks({ hooksDir: HOOKS_DIR, settingsPath: SETTINGS_PATH, uuidPath: UUID_PATH, launchdDir: LAUNCHD_DIR });

    expect(existsSync(join(HOOKS_DIR, "sensei-pre-tool-use.ts"))).toBe(true);
    expect(existsSync(join(HOOKS_DIR, "sensei-post-tool-use.ts"))).toBe(true);
  });

  it("hook scripts are executable", async () => {
    await installHooks({ hooksDir: HOOKS_DIR, settingsPath: SETTINGS_PATH, uuidPath: UUID_PATH, launchdDir: LAUNCHD_DIR });

    const mode = statSync(join(HOOKS_DIR, "sensei-pre-tool-use.ts")).mode;
    expect(mode & 0o111).toBeTruthy(); // at least one execute bit set
  });

  it("hook scripts start with a bun shebang", async () => {
    await installHooks({ hooksDir: HOOKS_DIR, settingsPath: SETTINGS_PATH, uuidPath: UUID_PATH, launchdDir: LAUNCHD_DIR });

    const preContent = readFileSync(join(HOOKS_DIR, "sensei-pre-tool-use.ts"), "utf8");
    const postContent = readFileSync(join(HOOKS_DIR, "sensei-post-tool-use.ts"), "utf8");
    expect(preContent.startsWith("#!/usr/bin/env bun")).toBe(true);
    expect(postContent.startsWith("#!/usr/bin/env bun")).toBe(true);
  });

  it("registers hooks in settings.json (creating it if absent)", async () => {
    await installHooks({ hooksDir: HOOKS_DIR, settingsPath: SETTINGS_PATH, uuidPath: UUID_PATH, launchdDir: LAUNCHD_DIR });

    const settings = JSON.parse(readFileSync(SETTINGS_PATH, "utf8")) as Record<string, unknown>;
    const hooks = settings.hooks as Record<string, unknown>;
    expect(hooks).toBeDefined();
    expect(hooks.PreToolUse).toBeDefined();
    expect(hooks.PostToolUse).toBeDefined();
  });

  it("merges into existing settings.json without overwriting other keys", async () => {
    const { writeFileSync } = await import("fs");
    writeFileSync(SETTINGS_PATH, JSON.stringify({ theme: "dark", hooks: {} }));

    await installHooks({ hooksDir: HOOKS_DIR, settingsPath: SETTINGS_PATH, uuidPath: UUID_PATH, launchdDir: LAUNCHD_DIR });

    const settings = JSON.parse(readFileSync(SETTINGS_PATH, "utf8")) as Record<string, unknown>;
    expect(settings.theme).toBe("dark");
  });

  it("creates launchd plist on macOS path", async () => {
    await installHooks({ hooksDir: HOOKS_DIR, settingsPath: SETTINGS_PATH, uuidPath: UUID_PATH, launchdDir: LAUNCHD_DIR });
    expect(existsSync(join(LAUNCHD_DIR, "com.sensei.collector.plist"))).toBe(true);
    const plist = readFileSync(join(LAUNCHD_DIR, "com.sensei.collector.plist"), "utf8");
    expect(plist).toContain("com.sensei.collector");
    expect(plist).toContain("51789");
  });
});
