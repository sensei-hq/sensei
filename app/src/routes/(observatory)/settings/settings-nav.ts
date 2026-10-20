/**
 * Settings rail data — pure, no runes.
 *
 * Mirrors observatory-nav.ts in shape (grouped rokkit List with a leaf-with-
 * separator tail) so ObservatorySidebar's rail interaction rules apply here
 * unchanged. Kept module-local so the structure + active-matching are
 * unit-testable without mounting the List or the SvelteKit page store.
 *
 * Sub-routes land in Slice B; this module only names them.
 */

/** A navigable rail entry. `value` mirrors `href` so List's value-match
 * (`proxy.value === value`) highlights the active route. */
export interface NavLink {
  kanji: string;
  /** The Solar Bold Duotone class shown in icon mode (#133). */
  icon: string;
  text: string;
  href: string;
  value: string;
  badge?: string | number;
}

/** A static cluster label with its children (List `collapsible={false}`). */
export interface NavGroup {
  text: string;
  children: NavLink[];
}

/** Thin divider rendered by List as `<hr data-list-separator>`. */
export interface NavSeparator {
  type: "separator";
}

export type NavEntry = NavLink | NavGroup | NavSeparator;

const link = (kanji: string, icon: string, text: string, href: string): NavLink => ({
  kanji,
  icon,
  text,
  href,
  value: href,
});

/**
 * Build the Settings rail. Three grouped clusters + a trailing Extensions leaf
 * after a separator. No badges yet — hooks in the shape so counts can land
 * later without a rail restructure.
 */
export function buildNavItems(): NavEntry[] {
  return [
    // You — identity and the assistants sensei talks to.
    {
      text: "You",
      children: [
        link("名", "i-solar-user-id-bold-duotone", "General", "/settings/general"),
        link("連", "i-solar-chat-round-dots-bold-duotone", "Assistants", "/settings/assistants"),
      ],
    },
    // Sources — where code + docs come from.
    {
      text: "Sources",
      children: [
        link("庵", "i-solar-folder-2-bold-duotone", "Roots", "/settings/roots"),
        link("組", "i-solar-folder-open-bold-duotone", "Projects", "/settings/projects"),
        link("書", "i-solar-book-bookmark-bold-duotone", "Libraries", "/settings/libraries"),
      ],
    },
    // Reasoning — models, chains, providers, and role assignments. Inference
    // already covers the role → chain wiring (InferenceAssignmentsPanel is a
    // live editor), so there is no separate Assignments entry. Providers is
    // the API-key surface for cloud routers (Keychain-backed).
    {
      text: "Reasoning",
      children: [
        link("器", "i-solar-tuning-2-bold-duotone", "Instruments", "/settings/instruments"),
        link("鍵", "i-solar-key-bold-duotone", "Providers", "/settings/providers"),
        link("想", "i-solar-cpu-bolt-bold-duotone", "Inference", "/settings/inference"),
      ],
    },
    // Measurement — what sensei measures, and why a metric is not current.
    // Deliberately not folded into Reasoning: that cluster is models, chains and
    // providers, i.e. how sensei THINKS. Metric activation is a cost decision
    // recorded by the dōjō that owns the repository, which is a different
    // question and would be buried next to Inference.
    {
      text: "Measurement",
      children: [link("測", "i-solar-chart-2-bold-duotone", "Metrics", "/settings/metrics")],
    },
    // Dōjō — the credential that reaches it, and what has been agreed. NOT the
    // connections editor: `/dojo/connections` already owns choosing WHICH dōjōs,
    // and rebuilding that list here would be a second copy to keep in step. These
    // two are what had no surface at all — a forge token could only be seen after
    // it died (the sign-in overlay) or from a terminal.
    {
      text: "Dōjō",
      children: [link("結", "i-solar-link-round-bold-duotone", "Connection", "/settings/dojo")],
    },
    { type: "separator" },
    link("拡", "i-solar-widget-add-bold-duotone", "Extensions", "/settings/extensions"),
  ];
}

/** Flatten every link from the rail. */
function allLinks(): NavLink[] {
  const out: NavLink[] = [];
  for (const e of buildNavItems()) {
    if ("href" in e) out.push(e);
    else if ("children" in e) out.push(...e.children);
  }
  return out;
}

/** Flatten every link href from the rail. */
function allHrefs(): string[] {
  return allLinks().map((l) => l.href);
}

/**
 * Which rail href should be highlighted for `pathname`. Exact match wins;
 * otherwise the longest href that is a path prefix. Empty string when nothing
 * matches — the caller decides whether to fall back to a default.
 */
export function resolveActiveHref(pathname: string): string {
  const hrefs = allHrefs();
  if (hrefs.includes(pathname)) return pathname;

  let best = "";
  for (const href of hrefs) {
    if (pathname.startsWith(href + "/") && href.length > best.length)
      best = href;
  }
  return best;
}

/** The icon of the rail entry at `href` (#133), for that screen's header. */
export function railIcon(href: string): string | undefined {
  return allLinks().find((l) => l.href === href)?.icon;
}
