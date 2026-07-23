// dojo2 kit — Solar iconify class map (single source of truth for icon CSS).
//
// The mockup (docs/mockups/Sensei/lib/dojo2/dojo2-kit.jsx `K2Icon`) painted
// every glyph as a remote `<img src=".../solar:{name}-bold-duotone.svg">`. In
// the app we render Solar through UnoCSS's preset-icons (`i-solar:*`, wired in
// uno.config.js against the build-time `@iconify-json/solar` JSON — no network
// fetch), which is a pure-CSS mask tinted by the element's text color.
//
// Icon.svelte receives a *logical* bare name (`folder`, `eye`, `command`, …).
// This map turns each into a full, static `i-solar:{name}-{variant}` class.
// Two reasons the map is explicit rather than string-interpolated at the call
// site:
//   1. UnoCSS scans source statically — it can only emit CSS for class names it
//      can *see* as literals. A runtime `i-solar:${name}` would generate nothing
//      (the original i-glyph bug). Listing the full classes here lets both the
//      static scanner see them AND lets uno.config.js safelist them by spreading
//      `ICON_CLASSES` (belt-and-suspenders — the class is picked at runtime).
//   2. Solar names are variant-suffixed. We pin ONE variant, `-linear`, across
//      the set: it is a crisp single-tone outline that tints cleanly with
//      `currentColor`. (`-bold-duotone`, the mockup's remote-SVG variant, uses a
//      second low-opacity fill that washes out when masked to a single color.)
//
// Every name below was verified to exist in @iconify-json/solar@1.2.x with a
// `-linear` variant — there are no substitutions. Adding an icon: add its
// logical name here (and it's automatically safelisted).

const VARIANT = '-linear';

/** Logical icon names used across the dojo2 kit, screens and nav. */
const NAMES = [
	// two-plane nav (personal + org) ids
	'widget-4',
	'eye',
	'folder',
	'scale',
	'box',
	'check-circle',
	'checklist-minimalistic',
	'chat-round-line',
	'users-group-two-rounded',
	'upload-square',
	'buildings-2',
	'inbox',
	'clipboard-check',
	'book-2',
	'case-round',
	'shield-warning',
	'document-text',
	'users-group-rounded',
	'shield-check',
	'key',
	'clipboard-list',
	'pulse',
	'card',
	// chrome + kit primitives
	'bell',
	'danger-triangle',
	'magnifer',
	'pen-2',
	'command',
	'arrow-right-up',
	'arrow-right',
	'alt-arrow-down',
	'alt-arrow-left',
	'alt-arrow-right',
	'close-circle',
	'layers-minimalistic',
	'refresh-circle',
	// screen action icons
	'add-circle',
	'bill-list',
	'document',
	'download-minimalistic',
	'download-square',
	'eye-closed',
	'link-circle',
	'lock-keyhole',
	'minus-circle',
	'pin',
	'restart',
	'trash-bin-minimalistic',
	'tuning-2',
	// stance dials
	'cpu-bolt',
	'share-circle',
	// knowledge catalog kinds
	'user-hands',
	'star',
	// membership roles
	'code',
	'settings',
	'user',
	'shield',
	// kit Solar names carried over from the mockup alias table
	'code-2',
	'shield-user',
	// misc route/data icons
	'hourglass'
] as const;

export type IconName = (typeof NAMES)[number];

/** Logical name → full `i-solar:{name}-linear` utility class. */
export const SOLAR: Record<string, string> = Object.fromEntries(
	NAMES.map((n) => [n, `i-solar:${n}${VARIANT}`])
);

/** Fallback for an unmapped name — a neutral square so nothing renders blank. */
export const ICON_FALLBACK = `i-solar:widget-4${VARIANT}`;

/**
 * Every `i-solar:*` class this app can render, for the uno.config.js safelist.
 * UnoCSS picks the class at runtime, so the safelist guarantees generation.
 */
export const ICON_CLASSES: string[] = [...Object.values(SOLAR), ICON_FALLBACK];

/** Resolve a logical icon name to its static `i-solar:*` utility class. */
export function iconClass(name: string): string {
	return SOLAR[name] ?? ICON_FALLBACK;
}
