import { describe, it, expect } from "vitest";
import {
  buildNavItems,
  resolveActiveHref,
  type NavGroup,
  type NavLink,
} from "./observatory-nav";

// Helpers to read the heterogeneous entry list without leaning on indices.
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

describe("buildNavItems", () => {
  it("shows anchors (top-level) + every cluster", () => {
    const entries = buildNavItems();
    expect(groups(entries).map((g) => g.text)).toEqual(["Needs you", "Review"]);

    // Anchors are top-level links; Settings is the trailing top-level link.
    const topHrefs = links(entries).map((l) => l.href);
    expect(topHrefs).toContain("/"); // Today
    expect(topHrefs).toContain("/projects");
    expect(topHrefs).toContain("/settings"); // Settings

    // Intake moved to the project window (a run always starts IN a project), so
    // it is NOT an observatory anchor anymore. Today is the leading anchor now.
    expect(topHrefs[0]).toBe("/");
    expect(byHref(entries, "/intake")).toBeUndefined();
  });

  it("renders a separator before Settings", () => {
    const entries = buildNavItems();
    const sepIdx = entries.findIndex(
      (e) => "type" in e && e.type === "separator",
    );
    const prefIdx = entries.findIndex(
      (e) => "href" in e && e.href === "/settings",
    );
    expect(sepIdx).toBeGreaterThanOrEqual(0);
    expect(prefIdx).toBeGreaterThan(sepIdx);
  });

  it('maps Memories to /learnings and labels it "Memories"', () => {
    const mem = byHref(buildNavItems(), "/learnings");
    expect(mem?.text).toBe("Memories");
    expect(mem?.kanji).toBe("覚");
  });

  it('labels /settings "Settings"', () => {
    expect(byHref(buildNavItems(), "/settings")?.text).toBe(
      "Settings",
    );
  });

  it("does not fabricate an Impact alert or badge (was a hardcoded MOCK)", () => {
    const impact = byHref(buildNavItems(), "/impact");
    expect(impact?.kanji).toBe("果");
    expect(impact?.alert).toBeUndefined();
    expect(impact?.badge).toBeUndefined();
  });

  it("carries no fabricated badge on any non-Projects rail entry", () => {
    for (const l of allLinks(buildNavItems())) {
      if (l.href === "/projects") continue;
      expect(l.badge, `${l.href} must not carry a mock badge`).toBeUndefined();
    }
  });

  it("sets each link value equal to its href (for List active matching)", () => {
    for (const link of allLinks(buildNavItems())) {
      expect(link.value).toBe(link.href);
    }
  });

  it("uses the project count as the Projects badge", () => {
    expect(
      byHref(buildNavItems({ projectCount: 12 }), "/projects")
        ?.badge,
    ).toBe(12);
  });

  /** Which group a href sits under, so "in the Review group" is actually tested
   *  rather than merely asserted in the test name. */
  const groupOf = (href: string) =>
    groups(buildNavItems()).find((g) =>
      (g.children ?? []).some((c) => "href" in c && c.href === href),
    )?.text;

  it("surfaces Consolidation (Tier-2 ruleset review) in the Review group", () => {
    const c = byHref(buildNavItems(), "/consolidation");
    expect(groupOf("/consolidation")).toBe("Review");
    expect(c?.text).toBe("Consolidation");
    expect(c?.kanji).toBe("統");
    expect(c?.value).toBe("/consolidation");
  });

  it("surfaces Dōjō connections in the Review group", () => {
    const dojo = byHref(buildNavItems(), "/dojo/connections");
    expect(groupOf("/dojo/connections")).toBe("Review");
    expect(dojo?.text).toBe("Dōjō");
    expect(dojo?.kanji).toBe("結");
  });

  it("surfaces Collective sharing next to Dōjō in the Review group", () => {
    const sharing = byHref(buildNavItems(), "/dojo/sharing");
    expect(groupOf("/dojo/sharing")).toBe("Review");
    expect(sharing?.text).toBe("Sharing");
    expect(sharing?.kanji).toBe("群");
  });

  it("surfaces Share review (the upstream publish-gate) in the Review group", () => {
    const review = byHref(buildNavItems(), "/share-review");
    expect(groupOf("/share-review")).toBe("Review");
    expect(review?.text).toBe("Share review");
    expect(review?.kanji).toBe("送");
    expect(review?.value).toBe("/share-review");
  });
});

describe("resolveActiveHref", () => {
  it("matches the exact route", () => {
    expect(resolveActiveHref("/")).toBe("/");
    expect(resolveActiveHref("/projects")).toBe("/projects");
    expect(resolveActiveHref("/learnings")).toBe("/learnings");
  });

  it("matches a nested route to its parent nav entry", () => {
    expect(resolveActiveHref("/projects/abc-123")).toBe("/projects");
  });

  it("does not let Today (/) swallow every route", () => {
    expect(resolveActiveHref("/sessions")).toBe("/sessions");
    expect(resolveActiveHref("/projects/abc")).not.toBe("/");
  });

  it("returns empty string for an unknown route", () => {
    expect(resolveActiveHref("/nope")).toBe("");
  });
});
