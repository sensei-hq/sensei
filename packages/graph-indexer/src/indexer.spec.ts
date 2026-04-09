import { describe, it, expect, beforeAll, afterAll } from "vitest";
import { writeFile, mkdir, rm } from "node:fs/promises";
import { join } from "node:path";
import { randomBytes } from "node:crypto";
import { homedir } from "node:os";
import { type Database, type Connection } from "kuzu";
import { extractTaggedComments } from "./parser.js";
import { ensureSchemaWithConn } from "./schema.js";
import { indexRepo } from "./indexer.js";
import { getSymbol } from "./layers.js";
import { detectDrift } from "./drift.js";

const FIXTURE_CONTENT = `
// WHY: we use a custom parser here because the standard one doesn't handle edge cases
export function parseToken(input: string): string {
  return input.trim();
}

// DECISION: always validate before calling parseToken
export function processInput(raw: string): string {
  const validated = validateInput(raw);
  return parseToken(validated);
}

function validateInput(s: string): string {
  if (!s) throw new Error('empty');
  return s;
}

export interface TokenConfig {
  maxLength: number;
  allowEmpty: boolean;
}
`;

let tmpDir: string;
let fixtureFile: string;
let db: Database;
let conn: Connection;
const PROJECT = "test-project";
const REPO_ID = `test-${randomBytes(4).toString("hex")}`;

async function openKuzuDb(
  dbPath: string
): Promise<{ db: Database; conn: Connection }> {
  const kuzuMod = await import("kuzu");
  const rDb = new kuzuMod.Database(dbPath);
  const rConn = new kuzuMod.Connection(rDb);
  return { db: rDb, conn: rConn };
}

beforeAll(async () => {
  // Create temp dir
  tmpDir = join("/tmp", `sensei-test-${randomBytes(4).toString("hex")}`);
  await mkdir(tmpDir, { recursive: true });

  fixtureFile = join(tmpDir, "fixture.ts");
  await writeFile(fixtureFile, FIXTURE_CONTENT);

  // Set up kuzu db for direct unit tests
  const dbPath = join(tmpDir, "test.kuzu");
  const { db: d, conn: c } = await openKuzuDb(dbPath);
  db = d;
  conn = c;
  await ensureSchemaWithConn(conn);
});

afterAll(async () => {
  await conn.close();
  await db.close();
  await rm(tmpDir, { recursive: true, force: true });

  // Clean up the repo db created by indexRepo
  const repoDbDir = join(homedir(), ".sensei", "projects", REPO_ID);
  await rm(repoDbDir, { recursive: true, force: true }).catch(() => {});
});

describe("extractTaggedComments", () => {
  it("should detect WHY and DECISION comments", () => {
    const comments = extractTaggedComments(fixtureFile, FIXTURE_CONTENT);
    expect(comments.length).toBeGreaterThanOrEqual(2);
    const tags = comments.map((c) => c.tag);
    expect(tags).toContain("WHY");
    expect(tags).toContain("DECISION");
  });

  it("should capture comment text", () => {
    const comments = extractTaggedComments(fixtureFile, FIXTURE_CONTENT);
    const whyComment = comments.find((c) => c.tag === "WHY");
    expect(whyComment?.text).toContain("custom parser");
  });
});

describe("indexRepo + getSymbol", () => {
  let indexResult: Awaited<ReturnType<typeof indexRepo>>;
  let rDb: Database;
  let rConn: Connection;

  beforeAll(async () => {
    indexResult = await indexRepo({
      repoPath: tmpDir,
      repoId: REPO_ID,
      project: PROJECT,
      include: ["**/*.ts"],
      exclude: ["**/*.spec.ts", "**/*.d.ts"],
    });

    // Open connection to indexed DB for querying
    const dbPath = join(homedir(), ".sensei", "projects", REPO_ID, "graph.kuzu");
    const opened = await openKuzuDb(dbPath);
    rDb = opened.db;
    rConn = opened.conn;
  });

  afterAll(async () => {
    await rConn.close();
    await rDb.close();
  });

  it("should index files", () => {
    expect(indexResult.filesIndexed).toBeGreaterThan(0);
  });

  it("should index functions", () => {
    // TypeScriptAdapter only indexes exported functions (parseToken, processInput)
    expect(indexResult.functionsIndexed).toBeGreaterThanOrEqual(2);
  });

  it("should index types", () => {
    expect(indexResult.typesIndexed).toBeGreaterThanOrEqual(1);
  });

  it("should return L0 symbol (name, kind, file, line)", async () => {
    const symbol = await getSymbol(rConn, "parseToken", PROJECT, 0);
    expect(symbol).not.toBeNull();
    expect(symbol?.name).toBe("parseToken");
    expect(symbol?.kind).toBe("function");
    expect(symbol?.file).toBe(fixtureFile);
    expect(symbol?.line).toBeGreaterThan(0);
  });

  it("should return L1 symbol (sig, docstring)", async () => {
    const symbol = await getSymbol(rConn, "parseToken", PROJECT, 1);
    expect(symbol).not.toBeNull();
    expect(symbol?.sig).toBeDefined();
    expect(symbol?.sig?.length).toBeGreaterThan(0);
  });

  it("should return L2 symbol (callers, callees)", async () => {
    const symbol = await getSymbol(rConn, "parseToken", PROJECT, 2);
    expect(symbol).not.toBeNull();
    expect(symbol?.callers).toBeDefined();
    expect(Array.isArray(symbol?.callees)).toBe(true);
  });

  it("should find callers of parseToken", async () => {
    const symbol = await getSymbol(rConn, "parseToken", PROJECT, 2);
    expect(symbol).not.toBeNull();
    // processInput calls parseToken
    const callerNames = symbol?.callers?.map((c) => c.name) ?? [];
    expect(callerNames).toContain("processInput");
  });
});

describe("detectDrift", () => {
  let driftDb: Database;
  let driftConn: Connection;
  const DRIFT_REPO_ID = `drift-test-${randomBytes(4).toString("hex")}`;

  beforeAll(async () => {
    await indexRepo({
      repoPath: tmpDir,
      repoId: DRIFT_REPO_ID,
      project: "drift-project",
      include: ["**/*.ts"],
      exclude: ["**/*.spec.ts", "**/*.d.ts"],
    });

    const dbPath = join(
      homedir(),
      ".sensei",
      "projects",
      DRIFT_REPO_ID,
      "graph.kuzu"
    );
    const opened = await openKuzuDb(dbPath);
    driftDb = opened.db;
    driftConn = opened.conn;
  });

  afterAll(async () => {
    await driftConn.close();
    await driftDb.close();
    const repoDbDir = join(homedir(), ".sensei", "projects", DRIFT_REPO_ID);
    await rm(repoDbDir, { recursive: true, force: true }).catch(() => {});
  });

  it("should return an array from detectDrift", async () => {
    const drifts = await detectDrift(driftConn, "drift-project");
    expect(Array.isArray(drifts)).toBe(true);
  });

  it("should detect drift for DECISION comment referencing parseToken in processInput", async () => {
    const drifts = await detectDrift(driftConn, "drift-project");
    // The DECISION comment says "always validate before calling parseToken"
    // "parseToken" is mentioned in the comment for processInput
    // but processInput's signature doesn't contain "parsetoken"
    // so it should be flagged as drift
    const hasDecisionDrift = drifts.some(
      (d) => d.tag === "DECISION" && d.issue.includes("parsetoken")
    );
    expect(hasDecisionDrift).toBe(true);
  });
});
