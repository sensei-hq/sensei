// @vitest-environment jsdom
import { describe, it, expect, afterEach } from "vitest";
import { flushSync } from "svelte";
import { mountComponent } from "$lib/test-mount.js";
import ObservatorySidebar from "../../routes/(observatory)/ObservatorySidebar.svelte";
import SettingsSidebar from "../../routes/(preferences)/settings/SettingsSidebar.svelte";
import WorkspaceSidebar from "../../routes/(workspace)/workspace/[id]/WorkspaceSidebar.svelte";

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

  it("the workspace sections are List links, marking the current one", () => {
    const m = mountComponent(WorkspaceSidebar, workspace);
    cleanup.push(m.destroy);
    flushSync();
    const links = [...m.container.querySelectorAll("[data-list] a[data-list-item]")] as HTMLAnchorElement[];
    expect(links.map((a) => a.getAttribute("href"))).toContain("/workspace/w1/overview");
    expect(links.length).toBe(14);
  });
});
