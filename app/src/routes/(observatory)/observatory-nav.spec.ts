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
  it("shows anchors (top-level) + every cluster in the full (All) rail", () => {
    const entries = buildNavItems({ focus: false });
    expect(groups(entries).map((g) => g.text)).toEqual(["Needs you", "Review"]);

    // Anchors are top-level links; Settings is the trailing top-level link.
    const topHrefs = links(entries).map((l) => l.href);
    expect(topHrefs).toContain("/"); // Today
    expect(topHrefs).toContain("/projects");
    expect(topHrefs).toContain("/settings"); // Settings
  });

  it("hides Review group, separator and Settings in Focus", () => {
    const entries = buildNavItems({ focus: true });
    expect(groups(entries).map((g) => g.text)).toEqual(["Needs you"]);
    expect(byHref(entries, "/settings")).toBeUndefined();
    expect(byHref(entries, "/sessions")).toBeUndefined();
    expect(entries.some((e) => "type" in e && e.type === "separator")).toBe(
      false,
    );
  });

  it("keeps anchors and the Needs you group visible in Focus", () => {
    const entries = buildNavItems({ focus: true });
    expect(byHref(entries, "/")).toBeDefined();
    expect(byHref(entries, "/projects")).toBeDefined();
    expect(byHref(entries, "/impact")).toBeDefined();
  });

  it("renders a separator before Settings in the full rail", () => {
    const entries = buildNavItems({ focus: false });
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
    const mem = byHref(buildNavItems({ focus: false }), "/learnings");
    expect(mem?.text).toBe("Memories");
    expect(mem?.kanji).toBe("覚");
  });

  it('labels /settings "Settings"', () => {
    expect(byHref(buildNavItems({ focus: false }), "/settings")?.text).toBe(
      "Settings",
    );
  });

  it("flags Impact with an alert", () => {
    const impact = byHref(buildNavItems({ focus: false }), "/impact");
    expect(impact?.alert).toBe(true);
    expect(impact?.kanji).toBe("果");
  });

  it("sets each link value equal to its href (for List active matching)", () => {
    for (const link of allLinks(buildNavItems({ focus: false }))) {
      expect(link.value).toBe(link.href);
    }
  });

  it("uses the project count as the Projects badge", () => {
    expect(
      byHref(buildNavItems({ focus: false, projectCount: 12 }), "/projects")
        ?.badge,
    ).toBe(12);
  });

  it("surfaces Dōjō connections in the Review group (hidden in Focus)", () => {
    const dojo = byHref(buildNavItems({ focus: false }), "/dojo/connections");
    expect(dojo?.text).toBe("Dōjō");
    expect(dojo?.kanji).toBe("結");
    expect(byHref(buildNavItems({ focus: true }), "/dojo/connections")).toBeUndefined();
  });

  it("surfaces Collective sharing next to Dōjō in the Review group (hidden in Focus)", () => {
    const sharing = byHref(buildNavItems({ focus: false }), "/dojo/sharing");
    expect(sharing?.text).toBe("Sharing");
    expect(sharing?.kanji).toBe("群");
    expect(byHref(buildNavItems({ focus: true }), "/dojo/sharing")).toBeUndefined();
  });

  it("surfaces Share review (the upstream publish-gate) in the Review group (hidden in Focus)", () => {
    const review = byHref(buildNavItems({ focus: false }), "/share-review");
    expect(review?.text).toBe("Share review");
    expect(review?.kanji).toBe("送");
    expect(review?.value).toBe("/share-review");
    expect(byHref(buildNavItems({ focus: true }), "/share-review")).toBeUndefined();
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
