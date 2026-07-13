import { sumiPalette } from './sumi-palette.js';

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

	icons: {},
	switcher: 'manual',
	storageKey: 'sensei-console-theme'
};
