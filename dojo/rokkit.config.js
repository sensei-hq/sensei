import { zenSumi } from '@rokkit/unocss/palettes';

// Logical icon names the dojo2 kit renders (bare Solar names from the mockup's
// K2Icon). Each becomes an `icons.overrides` bare-name shortcut → the full
// `i-solar:{name}-bold-duotone` utility; presetRokkit auto-safelists override keys, so
// `<span class="{name}">` (applied dynamically in Icon.svelte) still generates
// the CSS mask — no manual uno.config safelist, no runtime resolver.
const ICON_NAMES = [
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
];

/**
 * Dōjō console — Rokkit config: rokkit's zen-sumi family, the same theme as the desktop
 * app and the website, plus the Solar and brand icon collections.
 */
export default {
	// rokkit's zen-sumi family — palettes and skin imported, never copied
	// (user 2026-10-28); no overrides of rokkit's tokens.
	palettes: zenSumi.palettes,
	colorSpace: 'oklch',
	skin: zenSumi.skin,

	typography: {
		sans: "'Inter Variable', 'Inter', system-ui, -apple-system, sans-serif",
		mono: "'JetBrains Mono', 'SF Mono', Menlo, monospace",
		display: "'Fraunces', 'Iowan Old Style', Georgia, serif",
		kanji: "'Yu Mincho', 'Hiragino Mincho ProN', 'Songti SC', serif"
	},

	shape: {
		radius: 'soft'
	},

	// Register the full Solar iconify set so `i-solar:*` classes resolve
	// (presetRokkit already includes presetIcons; keys become the UnoCSS
	// collection prefix). dojo2's kit renders bare Solar names (mockup K2Icon)
	// through this collection — build-time CSS masks, no network fetch.
	// `overrides` maps each bare logical name → its `i-solar:{name}-bold-duotone`
	// utility (the rokkit bare-name-shortcut idiom); override keys are
	// auto-safelisted by presetRokkit, so Icon.svelte can apply the bare class
	// dynamically and UnoCSS still generates the CSS — no runtime resolver, no
	// manual uno.config safelist. `-bold-duotone` matches the mockup's K2Icon
	// (Solar bold-duotone) — a two-tone fill that reads richer than a flat outline.
	icons: {
		solar: '@iconify-json/solar/icons.json',
		// Brand logos for the sign-in providers (GitHub OAuth · Google). Referenced
		// as `i-simple-icons:{name}` (e.g. the GitHub button in DojoSignIn).
		'simple-icons': '@iconify-json/simple-icons/icons.json',
		overrides: Object.fromEntries(ICON_NAMES.map((n) => [n, `i-solar:${n}-bold-duotone`]))
	},
	switcher: 'manual',
	storageKey: 'sensei-dojo-theme'
};
