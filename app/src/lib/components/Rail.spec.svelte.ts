// @vitest-environment jsdom
import { describe, it, expect, afterEach } from "vitest";
import { flushSync, tick } from "svelte";
import { mountComponent } from "$lib/test-mount.js";
import ObservatorySidebar from "../../routes/(observatory)/ObservatorySidebar.svelte";
import SettingsSidebar from "../../routes/(preferences)/settings/SettingsSidebar.svelte";
import WorkspaceSidebar from "../../routes/(workspace)/workspace/[id]/WorkspaceSidebar.svelte";
import RailMenuButton from "./RailMenuButton.svelte";
import { railDrawer } from "./rail-drawer.svelte.js";

// The user (2026-10-28): the settings, main-window and workspace-window sidebars
// must be the same rokkit List with the same styles. They are one component —
// `Rail` — so they cannot drift: same frame, same List, same glyph rule.
let cleanup: Array<() => void> = [];
afterEach(() => {
  cleanup.forEach((fn) => fn());
  cleanup = [];
});

const workspace = { workspaceId: "w1", name: "sensei", icon: { kind: "kanji" as const, glyph: "場" }, ftr14d: null };
const mounts = {
  main: () => mountComponent(ObservatorySidebar, { port: 7744, pathname: "/" }),
  settings: () => mountComponent(SettingsSidebar, { pathname: "/settings/general" }),
  workspace: () => mountComponent(WorkspaceSidebar, workspace),
};

function rail(name: keyof typeof mounts) {
  const m = mounts[name]();
  cleanup.push(m.destroy);
  flushSync();
  return m.container.querySelector("aside[data-rail]") as HTMLElement | null;
}

describe("the three sidebars are one Rail", () => {
  it.each(Object.keys(mounts) as (keyof typeof mounts)[])("%s renders the Rail with a rokkit List", (name) => {
    const aside = rail(name);
    expect(aside, `${name} is not a Rail`).not.toBeNull();
    expect(aside!.querySelector("[data-list]")).not.toBeNull();
  });

  it("all three share one frame", () => {
    const classes = (Object.keys(mounts) as (keyof typeof mounts)[]).map((n) => rail(n)?.className);
    expect(new Set(classes).size).toBe(1);
  });

  it("only the nav list scrolls; the header and footer stay where they are", () => {
    const m = mountComponent(WorkspaceSidebar, workspace);
    cleanup.push(m.destroy);
    flushSync();
    const rail = m.container.querySelector("[data-rail]") as HTMLElement;
    const nav = rail.querySelector("[data-rail-nav]") as HTMLElement;
    expect(rail.className).toContain("overflow-hidden");
    expect([nav.className.includes("overflow-y-auto"), nav.className.includes("min-h-0"), nav.className.includes("flex-1")]).toEqual([true, true, true]);
  });

  it("the workspace sections are List links, marking the current one", () => {
    const m = mountComponent(WorkspaceSidebar, workspace);
    cleanup.push(m.destroy);
    flushSync();
    const links = [...m.container.querySelectorAll("[data-list] a[data-list-item]")] as HTMLAnchorElement[];
    expect(links.map((a) => a.getAttribute("href"))).toContain("/workspace/w1/overview");
    // 14 sections, then the Diagrams subsection's 8 diagrams.
    expect(links.length).toBe(22);
  });
});

// #420: below md (768px) the Rail is a drawer — off screen until the menu opens it, over the
// content with a backdrop; from md up it sits beside the content as before.

describe("on a phone the Rail is a drawer", () => {
  afterEach(() => railDrawer.close());

  it("sits off screen below md and beside the content from md up", () => {
    const aside = rail("main")!;
    for (const c of ["fixed", "-translate-x-full", "md:static", "md:translate-x-0"]) expect(aside.classList, c).toContain(c);
    expect(document.querySelector("[data-rail-backdrop]")).toBeNull();
  });

  it("the menu button opens it over the content; the backdrop closes it", async () => {
    const button = mountComponent(RailMenuButton, {});
    cleanup.push(button.destroy);
    const aside = rail("main")!;
    const open = button.container.querySelector("button[data-rail-menu]") as HTMLButtonElement;
    expect(open.classList).toContain("md:hidden");
    expect(open.getAttribute("aria-label")).toBe("Open the menu");
    expect(open.getAttribute("aria-expanded")).toBe("false");
    open.click();
    await tick();
    expect(aside.classList).toContain("translate-x-0");
    expect(aside.classList).not.toContain("-translate-x-full");
    expect(open.getAttribute("aria-expanded")).toBe("true");
    const backdrop = document.querySelector("[data-rail-backdrop]") as HTMLElement;
    expect(backdrop).not.toBeNull();
    backdrop.click();
    await tick();
    expect(aside.classList).toContain("-translate-x-full");
  });

  it("Escape closes it", async () => {
    const aside = rail("main")!;
    railDrawer.toggle();
    await tick();
    expect(aside.classList).toContain("translate-x-0");
    window.dispatchEvent(new KeyboardEvent("keydown", { key: "Escape" }));
    await tick();
    expect(aside.classList).toContain("-translate-x-full");
  });

  it("a navigation closes it (each layout calls navigated() after a navigation)", async () => {
    const aside = rail("settings")!;
    railDrawer.toggle();
    await tick();
    railDrawer.navigated();
    await tick();
    expect(railDrawer.open).toBe(false);
    expect(aside.classList).toContain("-translate-x-full");
  });
});
