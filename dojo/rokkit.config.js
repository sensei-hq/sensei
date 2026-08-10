import { sumiPalette } from '../packages/sumi-palette/index.js';

// Logical icon names the dojo2 kit renders (bare Solar names from the mockup's
// K2Icon). Each becomes an `icons.overrides` bare-name shortcut → the full
// `i-solar:{name}-linear` utility; presetRokkit auto-safelists override keys, so
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
 * Dōjō console — Rokkit named-token config.
 *
 * Ports the desktop app's Zen/Sumi named-token vocabulary (paper / ink /
 * primary / accent + status) so the console shares one visual system with the
 * rest of sensei. Dual-palette dark mode via kami (light) / sumi (dark).
 *
 * The console is the Dōjō SaaS surface, so `accent` is kavach purple
 * (murasaki) — matching the mockup's convention that vermillion is the sensei
 * product brand and purple is reserved for the auth/Dōjō plane. Everything
 * else follows the app's overrides so the token names resolve identically.
 */
export default {
	palettes: sumiPalette,
	colorSpace: 'oklch',

	skin: {
		surface: { light: 'kami', dark: 'sumi' },
		ink: { light: 'kami', dark: 'sumi' },
		primary: 'shu',
		secondary: 'murasaki',
		accent: 'shu',
		success: 'hisui',
		warning: 'kohaku',
		danger: 'beni',
		error: 'beni',
		info: 'ai'
	},

	overrides: {
		// ── Surface (paper) ──────────────────────────────────────────
		paper: { light: 'kami.100', dark: 'sumi.50' },
		'paper-soft': { light: 'kami.200', dark: 'sumi.100' },
		'paper-mute': { light: 'kami.300', dark: 'sumi.200' },
		'paper-edge': { light: 'kami.400', dark: 'sumi.300' },

		// ── Ink (text-zone shades) ───────────────────────────────────
		ink: { light: 'kami.900', dark: 'sumi.900' },
		'ink-soft': { light: 'kami.700', dark: 'sumi.800' },
		'ink-mute': { light: 'kami.600', dark: 'sumi.700' },
		'ink-faint': { light: 'kami.500', dark: 'sumi.600' },

		// ── Accent — vermillion (shared with the sensei product brand) ─
		accent: { light: 'shu.500', dark: 'shu.400' },

		// ── Primary named token — ink-colored CTA (ink-on-paper button) ─
		primary: { light: 'kami.900', dark: 'sumi.900' },
		'on-primary': { light: 'kami.100', dark: 'sumi.50' },

		// ── Status — lighten for legibility in dark mode ──────────────
		success: { light: 'hisui.500', dark: 'hisui.400' },
		warning: { light: 'kohaku.500', dark: 'kohaku.400' },
		danger: { light: 'beni.500', dark: 'beni.400' },
		info: { light: 'ai.500', dark: 'ai.400' }
	},

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
	// `overrides` maps each bare logical name → its `i-solar:{name}-linear`
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
