import { describe, it, expect } from "vitest";
import { generateRunName } from "./names.js";

describe("generateRunName", () => {
  it("returns adjective-noun format", () => {
    const name = generateRunName();
    expect(name).toMatch(/^[a-z]+-[a-z]+$/);
  });

  it("returns different names on repeated calls (probabilistic)", () => {
    const names = new Set(Array.from({ length: 20 }, () => generateRunName()));
    expect(names.size).toBeGreaterThan(1);
  });
});
