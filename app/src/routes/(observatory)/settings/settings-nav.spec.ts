import { describe, it, expect } from "vitest";
import {
  buildNavItems,
  resolveActiveHref,
  type NavGroup,
  type NavLink,
} from "./settings-nav";

const groups = (entries: ReturnType<typeof buildNavItems>): NavGroup[] =>
  entries.filter((e): e is NavGroup => "children" in e);
const links = (entries: ReturnType<typeof buildNavItems>): NavLink[] =>
  entries.filter((e): e is NavLink => "href" in e);
const allLinks = (entries: ReturnType<typeof buildNavItems>): NavLink[] => [
  ...links(entries),
  ...groups(entries).flatMap((g) => g.children),
];
const byHref = (entries: ReturnType<typeof buildNavItems>, href: string) =>
  allLinks(entries).find((l) => l.href === href);

describe("settings buildNavItems", () => {
  it("exposes five grouped clusters + a trailing leaf", () => {
    const entries = buildNavItems();
    expect(groups(entries).map((g) => g.text)).toEqual([
      "You",
      "Sources",
      "Reasoning",
      "Measurement",
      "Dōjō",
    ]);
    expect(links(entries).map((l) => l.href)).toContain("/settings/extensions");
  });

  it("groups Measurement → Metrics, separate from Reasoning", () => {
    // Not folded into Reasoning: that cluster is models, chains and providers —
    // how sensei THINKS. Which metrics compute, and why one is not current, is a
    // different question, and grouping them together would bury it.
    const measurement = groups(buildNavItems()).find(
      (g) => g.text === "Measurement",
    )!;
    expect(measurement.children.map((c) => c.href)).toEqual([
      "/settings/metrics",
    ]);
  });

  it("groups You → General + Assistants", () => {
    const you = groups(buildNavItems()).find((g) => g.text === "You")!;
    expect(you.children.map((c) => c.href)).toEqual([
      "/settings/general",
      "/settings/assistants",
    ]);
  });

  it("groups Sources → Roots + Projects + Libraries", () => {
    const sources = groups(buildNavItems()).find((g) => g.text === "Sources")!;
    expect(sources.children.map((c) => c.href)).toEqual([
      "/settings/roots",
      "/settings/projects",
      "/settings/libraries",
    ]);
  });

  it("groups Reasoning → Instruments + Providers + Inference (no Assignments — Inference is a live role→chain editor)", () => {
    const reasoning = groups(buildNavItems()).find(
      (g) => g.text === "Reasoning",
    )!;
    expect(reasoning.children.map((c) => c.href)).toEqual([
      "/settings/instruments",
      "/settings/providers",
      "/settings/inference",
    ]);
    expect(byHref(buildNavItems(), "/settings/assignments")).toBeUndefined();
  });

  it("groups Dōjō → the credential + sync surface, not the connections editor", () => {
    // `/dojo/connections` (managing WHICH dōjōs) stays in the observatory where it
    // already lives; this settings entry is the credential standing and what has
    // been agreed — the two things that had no surface at all. Rebuilding the
    // membership list here would be a second copy to keep in step.
    const dojo = groups(buildNavItems()).find((g) => g.text === "Dōjō")!;
    expect(dojo.children.map((c) => c.href)).toEqual(["/settings/dojo"]);
    expect(byHref(buildNavItems(), "/settings/dojo/connections")).toBeUndefined();
  });

  it("renders a separator before Extensions", () => {
    const entries = buildNavItems();
    const sepIdx = entries.findIndex(
      (e) => "type" in e && e.type === "separator",
    );
    const extIdx = entries.findIndex(
      (e) => "href" in e && e.href === "/settings/extensions",
    );
    expect(sepIdx).toBeGreaterThanOrEqual(0);
    expect(extIdx).toBeGreaterThan(sepIdx);
  });

  it("sets each link value equal to its href (for List active matching)", () => {
    for (const l of allLinks(buildNavItems())) {
      expect(l.value).toBe(l.href);
    }
  });

  it("gives each link a meaning kanji, not a counter", () => {
    for (const l of allLinks(buildNavItems())) {
      expect(l.kanji.length).toBeGreaterThan(0);
      expect(l.kanji).not.toMatch(/^\d+$/);
    }
  });

  it("does not leak badges before they are wired", () => {
    for (const l of allLinks(buildNavItems())) {
      expect(l.badge).toBeUndefined();
    }
  });
});

describe("settings resolveActiveHref", () => {
  it("matches the exact route", () => {
    expect(resolveActiveHref("/settings/general")).toBe("/settings/general");
    expect(resolveActiveHref("/settings/inference")).toBe(
      "/settings/inference",
    );
  });

  it("matches a nested sub-route to its parent nav entry", () => {
    // Anticipates future nested pages like /settings/projects/abc-123.
    expect(resolveActiveHref("/settings/projects/abc-123")).toBe(
      "/settings/projects",
    );
  });

  it("keeps Metrics highlighted on a per-repository sub-route", () => {
    expect(resolveActiveHref("/settings/metrics/github.com-acme-api")).toBe(
      "/settings/metrics",
    );
  });

  it("returns empty string for an unknown route", () => {
    expect(resolveActiveHref("/nope")).toBe("");
    expect(resolveActiveHref("/settings")).toBe("");
  });
});
