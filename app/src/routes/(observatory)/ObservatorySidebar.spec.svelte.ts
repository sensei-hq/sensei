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
      "Projects",
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
    const { container } = mount("/projects");
    const active = container.querySelector('[aria-current="page"]');
    expect(active?.getAttribute("href")).toBe("/projects");
  });

  it("highlights the parent entry for a nested route", () => {
    const { container } = mount("/projects/abc-123");
    expect(
      container.querySelector('[aria-current="page"]')?.getAttribute("href"),
    ).toBe("/projects");
  });

  it("has no All|Focus control, and always shows the whole rail", () => {
    // The toggle is gone. It hid Review and Settings, which bought little on a
    // rail this short, and the rokkit Toggle it used has a dark-mode contrast
    // bug in the `zen-sumi` style: the hover/focus rule is not guarded against
    // `[data-selected]` and is MORE specific than the selected rule, so
    // focusing the chosen segment replaced its `primary`/`on-primary` pair with
    // `paper-mute`/`ink-mute` — muted on muted, invisible in dark mode.
    //
    // Asserted rather than merely deleted, so the control cannot creep back
    // without this failing.
    const { container } = mount("/");
    expect(container.querySelectorAll("[data-toggle-option]")).toHaveLength(0);

    const t = container.textContent ?? "";
    for (const label of ["Review", "Settings", "Sessions", "Insights", "Projects"]) {
      expect(t, label).toContain(label);
    }
  });

  it("shows the project count badge when provided", () => {
    const m = mountComponent(ObservatorySidebar, {
      port: 7744,
      pathname: "/",
      projectCount: 12,
    });
    cleanup.push(m.destroy);
    flushSync();
    const projectsLink = m.container.querySelector('a[href="/projects"]');
    expect(projectsLink?.textContent).toContain("12");
  });

  it("omits the project badge when the count is not yet loaded", () => {
    const m = mountComponent(ObservatorySidebar, { port: 7744, pathname: "/" });
    cleanup.push(m.destroy);
    flushSync();
    const projectsLink = m.container.querySelector('a[href="/projects"]');
    // No digits in the Projects row before the count loads.
    expect(projectsLink?.textContent).not.toMatch(/\d/);
  });

  it("shows the daemon footer with the live port", () => {
    const { container } = mount("/");
    const t = container.textContent ?? "";
    expect(t).toContain("daemon · running");
    expect(t).toContain("7744");
  });
});
