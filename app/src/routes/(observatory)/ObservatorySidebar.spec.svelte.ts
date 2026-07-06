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

// The All|Focus control is a rokkit Toggle — its options are [data-toggle-option].
const segButton = (container: HTMLElement, label: string) =>
  [...container.querySelectorAll("[data-toggle-option]")].find(
    (b) => b.textContent?.trim() === label,
  ) as HTMLElement | undefined;

describe("ObservatorySidebar", () => {
  it("renders every rail entry and both cluster labels in All mode", () => {
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

  it("hides the Review group and Settings when Focus is selected", () => {
    const { container } = mount("/");
    expect(container).toMatchSnapshot();
    const focusBtn = segButton(container, "Focus");
    expect(focusBtn).toBeTruthy();
    focusBtn!.click();
    flushSync();
    const t = container.textContent ?? "";
    expect(t).not.toContain("Review");
    expect(t).not.toContain("Settings");
    expect(t).not.toContain("Sessions");
    // Anchors + "Needs you" remain.
    expect(t).toContain("Insights");
    expect(t).toContain("Projects");
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
