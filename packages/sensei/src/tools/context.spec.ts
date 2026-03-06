import { describe, it, expect, beforeEach, afterEach } from "vitest";
import { mkdirSync, writeFileSync, rmSync } from "fs";
import { join } from "path";
import { loadContext, recommendNext } from "./context.js";

const TMP = "/tmp/sensei-test-context";

beforeEach(() => {
  mkdirSync(join(TMP, ".index"), { recursive: true });
  writeFileSync(join(TMP, ".llmspec.yaml"), `
project: ctx-test
description: A context test project
stack: [typescript, react]
entry_points:
  - path: src/index.ts
    role: server entry
shortcuts:
  dev: bun run dev
`);
  writeFileSync(join(TMP, ".index/symbol-map.json"), JSON.stringify({
    "src/auth.ts": { L0: ["login(): Promise<User>"], L1: [], L2: [] },
    "src/billing/invoice.ts": { L0: ["createInvoice(): Invoice"], L1: [], L2: [] },
  }));
  writeFileSync(join(TMP, ".index/patterns.md"), "# Patterns\n\n## Repo Pattern\nUse repos for DB.");
  writeFileSync(join(TMP, ".index/stack.md"), "# Stack\n\n- typescript\n- react");
});

afterEach(() => rmSync(TMP, { recursive: true, force: true }));

describe("loadContext", () => {
  it("returns orientation slice", async () => {
    const slice = await loadContext(TMP, "orientation");
    expect(slice.content).toContain("ctx-test");
    expect(slice.content).toContain("typescript");
    expect(slice.tokenEstimate).toBeLessThan(250);
  });

  it("returns patterns slice", async () => {
    const slice = await loadContext(TMP, "patterns");
    expect(slice.content).toContain("Repo Pattern");
  });

  it("returns module-scoped slice", async () => {
    const slice = await loadContext(TMP, "src/billing");
    expect(slice.content).toContain("createInvoice");
    expect(slice.content).not.toContain("login");
  });

  it("returns empty message for unknown scope", async () => {
    const slice = await loadContext(TMP, "src/nonexistent");
    expect(slice.content).toContain("No exports");
  });
});

describe("recommendNext", () => {
  it("recommends L0 for list/find tasks", async () => {
    const rec = await recommendNext(TMP, "list all functions in auth module");
    expect(rec).toContain("L0");
    expect(rec).toContain("list_exports");
  });

  it("recommends L3 for fix/bug tasks", async () => {
    const rec = await recommendNext(TMP, "fix the bug in validateToken");
    expect(rec).toContain("L3");
  });

  it("recommends orientation for unknown tasks", async () => {
    const rec = await recommendNext(TMP, "do something vague");
    expect(rec).toContain("get_llmspec");
  });
});
