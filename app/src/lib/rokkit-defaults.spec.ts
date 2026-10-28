import { describe, it, expect } from "vitest";
import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import config from "../../rokkit.config.js";

// The user (2026-10-27): rokkit's zen-sumi styles are the look — no app
// overrides. The app had repainted `primary` in ink (Toggle's selected fill, the
// Switch's on track and the List's active icon came out black and white, not
// vermillion), forced List and Toggle label colours, resized the main sidebar's
// List and recoloured the primary Button.
const ROKKIT_PARTS = /\[data-(list|list-item|list-group|list-separator|item-[a-z]+|toggle|toggle-option|toggle-label|switch|switch-track|switch-thumb|button|tabs|select|tree|menu|dropdown|pill)\]/;

describe("rokkit's zen-sumi styles, as shipped", () => {
  it("app.css restyles no rokkit part", () => {
    const css = readFileSync(resolve(__dirname, "../app.css"), "utf8").replace(/\/\*[\s\S]*?\*\//g, "");
    const rules = css.split("}").map((r) => r.split("{")[0].trim()).filter((sel) => ROKKIT_PARTS.test(sel));
    expect(rules).toEqual([]);
  });

  it("the config leaves primary and on-primary to the skin", () => {
    const overrides = (config as { overrides?: Record<string, unknown> }).overrides ?? {};
    expect(Object.keys(overrides)).not.toContain("primary");
    expect(Object.keys(overrides)).not.toContain("on-primary");
  });
});
