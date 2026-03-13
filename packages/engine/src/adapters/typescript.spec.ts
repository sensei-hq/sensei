import { describe, it, expect } from "vitest";
import { resolve, dirname } from "path";
import { fileURLToPath } from "url";
import { TypeScriptAdapter } from "./typescript.js";

const _dir = dirname(fileURLToPath(import.meta.url));
const fixturePath = resolve(_dir, "fixtures/sample.ts");

describe("TypeScriptAdapter", () => {
  const adapter = new TypeScriptAdapter();

  it("handles .ts and .tsx extensions", () => {
    expect(adapter.extensions).toContain(".ts");
    expect(adapter.extensions).toContain(".tsx");
  });

  it("extracts exported function with docstring", async () => {
    const result = await adapter.parse({ path: "src/fixtures/sample.ts", absPath: fixturePath, mtime: 0, hash: "", size: 0 });
    const fn = result.symbols.find(s => s.name === "readTextFile");
    expect(fn).toBeDefined();
    expect(fn!.kind).toBe("function");
    expect(fn!.isExported).toBe(true);
    expect(fn!.docstring).toContain("Reads a file");
    expect(fn!.lineStart).toBeGreaterThan(0);
    expect(fn!.lineEnd).toBeGreaterThanOrEqual(fn!.lineStart);
  });

  it("extracts exported class", async () => {
    const result = await adapter.parse({ path: "src/fixtures/sample.ts", absPath: fixturePath, mtime: 0, hash: "", size: 0 });
    const cls = result.symbols.find(s => s.name === "FileCache");
    expect(cls).toBeDefined();
    expect(cls!.kind).toBe("class");
    expect(cls!.isExported).toBe(true);
  });

  it("extracts exported interface", async () => {
    const result = await adapter.parse({ path: "src/fixtures/sample.ts", absPath: fixturePath, mtime: 0, hash: "", size: 0 });
    const iface = result.symbols.find(s => s.name === "CacheOptions");
    expect(iface).toBeDefined();
    expect(iface!.kind).toBe("interface");
  });

  it("extracts exported type alias", async () => {
    const result = await adapter.parse({ path: "src/fixtures/sample.ts", absPath: fixturePath, mtime: 0, hash: "", size: 0 });
    const t = result.symbols.find(s => s.name === "CacheKey");
    expect(t).toBeDefined();
    expect(t!.kind).toBe("type");
  });

  it("does not include unexported function", async () => {
    const result = await adapter.parse({ path: "src/fixtures/sample.ts", absPath: fixturePath, mtime: 0, hash: "", size: 0 });
    const internal = result.symbols.find(s => s.name === "resolveFilePath");
    expect(internal).toBeUndefined(); // not exported — adapter skips it
  });

  it("extracts import from 'fs/promises'", async () => {
    const result = await adapter.parse({ path: "src/fixtures/sample.ts", absPath: fixturePath, mtime: 0, hash: "", size: 0 });
    const imp = result.imports.find(i => i.targetPath === "fs/promises");
    expect(imp).toBeDefined();
    expect(imp!.names).toContain("readFile");
  });
});
