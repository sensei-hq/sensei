import { describe, it, expect } from "vitest";
import { readdirSync, readFileSync } from "node:fs";
import { join, relative, resolve } from "node:path";
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


// A role's -soft tint stays light in both modes, so its bright text reads 2.9:1 on it by day and
// worse by night (axe, 2026-10-28). Text on a tint is `text-on-{role}-soft` (rokkit ≥ 1.12): the
// role darkened toward black, AA on every tint in both modes.
const ROLES = ["accent", "success", "warning", "danger", "error", "info"];
const FRONTENDS = ["app/src", "dojo/src", "website/src"];

function sources(dir: string): string[] {
  return readdirSync(dir, { withFileTypes: true }).flatMap((e) => {
    const path = join(dir, e.name);
    if (e.isDirectory()) return e.name === "__snapshots__" ? [] : sources(path);
    return /\.(svelte|ts)$/.test(e.name) && !/\.spec\./.test(e.name) ? [path] : [];
  });
}

describe("text on a soft tint", () => {
  const root = resolve(__dirname, "../../..");
  const files = FRONTENDS.flatMap((d) => sources(join(root, d)));

  it("finds the frontends' sources", () => {
    expect(files.length).toBeGreaterThan(300);
  });

  it("never draws a role's bright text on its own tint", () => {
    const found = files.flatMap((f) => {
      // Comments describe classes; only code draws them.
      const text = readFileSync(f, "utf8")
        .replace(/\/\*[\s\S]*?\*\/|<!--[\s\S]*?-->/g, (c) => c.replace(/[^\n]/g, " "))
        .replace(/(^|\s)\/\/.*$/gm, "$1");
      const lines = text.split("\n").flatMap((line, i) =>
        // A tone table may list a role's paper text beside its tint — when it also gives the tint's own.
        ROLES.filter(
          (r) =>
            line.includes(`bg-${r}-soft`) &&
            new RegExp(`(?<![\\w-])text-${r}(?![\\w-])`).test(line) &&
            !line.includes(`text-on-${r}-soft`),
        ).map(
          (r) => `${relative(root, f)}:${i + 1} text-${r} on bg-${r}-soft`,
        ),
      );
      const directives = ROLES.filter(
        (r) => text.includes(`class:bg-${r}-soft=`) && text.includes(`class:text-${r}=`),
      ).map((r) => `${relative(root, f)} class:text-${r} beside class:bg-${r}-soft`);
      return [...lines, ...directives];
    });
    expect(found).toEqual([]);
  });
});
