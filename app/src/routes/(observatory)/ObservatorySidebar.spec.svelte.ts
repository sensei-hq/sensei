// @vitest-environment jsdom
import { describe, it, expect, afterEach } from "vitest";
import { flushSync } from "svelte";
import { mountComponent } from "$lib/test-mount.js";
import ObservatorySidebar from "./ObservatorySidebar.svelte";

let cleanup: Array<() => void> = [];
afterEach(() => {
  cleanup.forEach((fn) => fn());
  cleanup = [];
});

const mount = (pathname = "/") => {
  const m = mountComponent(ObservatorySidebar, { port: 7744, pathname });
  cleanup.push(m.destroy);
  // List expands its non-collapsible groups in a post-mount $effect; flush it so
  // group children are in the DOM (in the real app this runs before first paint).
  flushSync();
  return m;
};

describe("ObservatorySidebar", () => {
  it("renders every rail entry and both cluster labels", () => {
    const { container } = mount("/");
    const t = container.textContent ?? "";
    for (const label of [
      "Today",
      "Workspaces",
      "Insights",
      "Memories",
      "Impact",
      "Traceability",
      "Upgrades",
      "Sessions",
      "Libraries",
      "Instruments",
      "Logs",
      "Settings",
    ]) {
      expect(t).toContain(label);
    }
    expect(t).toContain("Needs you");
    expect(t).toContain("Review");
  });

  it('marks the active route with aria-current="page"', () => {
    const { container } = mount("/workspaces");
    const active = container.querySelector('[aria-current="page"]');
    expect(active?.getAttribute("href")).toBe("/workspaces");
  });

  it("highlights the parent entry for a nested route", () => {
    const { container } = mount("/workspaces/abc-123");
    expect(
      container.querySelector('[aria-current="page"]')?.getAttribute("href"),
    ).toBe("/workspaces");
  });

  it("has no All|Focus control, and always shows the whole rail", () => {
    // The toggle is gone. It hid Review and Settings, which bought little on a
    // rail this short, and the rokkit Toggle it used has a dark-mode contrast
    // bug in the `zen-sumi` style: the hover/focus rule is not guarded against
    // `[data-selected]` and is MORE specific than the selected rule, so
    // focusing the chosen segment replaced its `primary`/`on-primary` pair with
    // `paper-mute`/`ink-soft` — muted on muted, invisible in dark mode.
    //
    // Asserted rather than merely deleted, so the control cannot creep back
    // without this failing.
    const { container } = mount("/");
    expect(container.querySelectorAll("[data-toggle-option]")).toHaveLength(0);

    const t = container.textContent ?? "";
    for (const label of ["Review", "Settings", "Sessions", "Insights", "Workspaces"]) {
      expect(t, label).toContain(label);
    }
  });

  it("shows the workspace count badge when provided", () => {
    const m = mountComponent(ObservatorySidebar, {
      port: 7744,
      pathname: "/",
      workspaceCount: 12,
    });
    cleanup.push(m.destroy);
    flushSync();
    const workspacesLink = m.container.querySelector('a[href="/workspaces"]');
    expect(workspacesLink?.textContent).toContain("12");
  });

  it("omits the workspace badge when the count is not yet loaded", () => {
    const m = mountComponent(ObservatorySidebar, { port: 7744, pathname: "/" });
    cleanup.push(m.destroy);
    flushSync();
    const workspacesLink = m.container.querySelector('a[href="/workspaces"]');
    // No digits in the Workspaces row before the count loads.
    expect(workspacesLink?.textContent).not.toMatch(/\d/);
  });

  it("shows the daemon footer with the live port", () => {
    const { container } = mount("/");
    const t = container.textContent ?? "";
    expect(t).toContain("daemon · running");
    expect(t).toContain("7744");
  });
});

// #133: the rail follows the one preference — kanji as literal text, or the
// Solar class as an icon — through the same List slot.
describe("ObservatorySidebar glyphs", () => {
  const mountIn = (glyphs: "kanji" | "icon") => {
    const m = mountComponent(ObservatorySidebar, { port: 7744, pathname: "/", glyphs });
    cleanup.push(m.destroy);
    flushSync();
    return m.container;
  };

  it("shows kanji by default", () => {
    const c = mountIn("kanji");
    const literals = [...c.querySelectorAll("[data-item-icon-literal]")].map((e) => e.textContent);
    expect(literals).toContain("家");
    expect(c.querySelector("[data-item-icon]")).toBeNull();
  });

  it("shows icons in icon mode", () => {
    const c = mountIn("icon");
    const icons = [...c.querySelectorAll("[data-item-icon]")].map((e) => e.className);
    expect(icons.some((cls) => cls.includes("i-solar-home-smile-bold-duotone"))).toBe(true);
    expect(c.querySelector("[data-item-icon-literal]")).toBeNull();
  });
});

// An identity that needs you is said in the footer while it lasts (user, 2026-10-29),
// instead of an overlay over the screen; clicking it goes where it can be handled.
describe("ObservatorySidebar — attention in the footer", () => {
  it("shows the sentence while something needs you, and clicking it opens the identities", () => {
    let opened = 0;
    const m = mountComponent(ObservatorySidebar, {
      port: 7744,
      pathname: "/",
      attention: "Sync paused — 1 identity needs sign-in",
      onAttention: () => opened++,
    });
    cleanup.push(m.destroy);
    flushSync();
    const row = m.container.querySelector<HTMLButtonElement>("[data-attention]");
    expect(row?.textContent).toContain("Sync paused — 1 identity needs sign-in");
    row!.click();
    expect(opened).toBe(1);
  });

  it("shows nothing when nothing needs you", () => {
    const { container } = mount("/");
    expect(container.querySelector("[data-attention]")).toBeNull();
  });
});
