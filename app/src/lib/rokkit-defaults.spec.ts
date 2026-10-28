import { describe, it, expect } from "vitest";
import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import config from "../../rokkit.config.js";
import dojoConfig from "../../../dojo/rokkit.config.js";
import websiteConfig from "../../../website/rokkit.config.js";
import { zenSumi } from "@rokkit/unocss/palettes";

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

// The user (2026-10-28): rokkit, sensei and the dōjō share one theme — rokkit's
// zen-sumi family, imported, not a copy (packages/sumi-palette was the third,
// drifted one). Each frontend may ADD tokens (the website's product hues, a
// scrim) but overrides none of rokkit's.
const ROKKIT_TOKENS = [
  "paper", "paper-soft", "paper-mute", "paper-edge", "ink", "ink-soft", "ink-mute", "ink-faint",
  "primary", "on-primary", "secondary", "tertiary", "accent", "success", "warning", "danger", "error", "info",
  "accent-soft", "success-soft", "warning-soft", "danger-soft", "error-soft", "info-soft",
];
type Cfg = { palettes?: unknown; skin?: unknown; overrides?: Record<string, unknown> };

describe.each([
  ["app", config],
  ["dōjō", dojoConfig],
  ["website", websiteConfig],
] as [string, Cfg][])("%s wears rokkit's zen-sumi", (_name, cfg) => {
  it("takes its palettes and skin from @rokkit/unocss/palettes", () => {
    expect(cfg.palettes).toEqual(zenSumi.palettes);
    expect(cfg.skin).toEqual(zenSumi.skin);
  });

  it("overrides none of rokkit's tokens", () => {
    const own = Object.keys(cfg.overrides ?? {}).filter((k) => ROKKIT_TOKENS.includes(k));
    expect(own).toEqual([]);
  });
});

