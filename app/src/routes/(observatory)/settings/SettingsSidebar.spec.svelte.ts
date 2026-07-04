// @vitest-environment jsdom
import { describe, it, expect, afterEach } from "vitest";
import { flushSync } from "svelte";
import { mountComponent } from "$lib/test-mount.js";
import SettingsSidebar from "./SettingsSidebar.svelte";

let cleanup: Array<() => void> = [];
afterEach(() => {
  cleanup.forEach((fn) => fn());
  cleanup = [];
});

const mount = (pathname = "/settings/general") => {
  const m = mountComponent(SettingsSidebar, { pathname });
  cleanup.push(m.destroy);
  // List expands its non-collapsible groups in a post-mount $effect; flush it so
  // group children are in the DOM (mirrors the observatory sidebar pattern).
  flushSync();
  return m;
};

describe("SettingsSidebar", () => {
  it("renders every rail entry and every group label", () => {
    const { container } = mount();
    const t = container.textContent ?? "";
    for (const label of [
      "General",
      "Assistants",
      "Roots",
      "Projects",
      "Libraries",
      "Instruments",
      "Inference",
      "Extensions",
    ]) {
      expect(t).toContain(label);
    }
    for (const group of ["You", "Sources", "Reasoning"]) {
      expect(t).toContain(group);
    }
  });

  it('marks the active route with aria-current="page"', () => {
    const { container } = mount("/settings/inference");
    const active = container.querySelector('[aria-current="page"]');
    expect(active?.getAttribute("href")).toBe("/settings/inference");
  });

  it("highlights the parent entry for a nested sub-route", () => {
    const { container } = mount("/settings/projects/abc-123");
    expect(
      container.querySelector('[aria-current="page"]')?.getAttribute("href"),
    ).toBe("/settings/projects");
  });

  it("shows the Settings eyebrow header", () => {
    const { container } = mount();
    expect(container.textContent).toContain("Settings");
  });
});
