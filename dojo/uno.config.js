import { defineConfig } from 'unocss';
import { presetRokkit } from '@rokkit/unocss';
import config from './rokkit.config.js';

// presetRokkit already includes extractorSvelte, transformerDirectives and
// transformerVariantGroup. Safelist the kavach auth icons (rendered via
// dynamic `i-auth-{name}` class names in @kavach/ui's AuthProvider, which
// UnoCSS can't see statically).
//
// `theme` mirrors `app/uno.config.js` verbatim — per the shared design system
// (docs/architecture/frontend-svelte-guidelines.md §1.8 per-surface config
// parity). This maps `text-sm`→13px, `text-base`→15px, `text-lg`→17px, … onto
// the design's 8-stop scale so the utilities ARE the design system; without it
// `text-sm` falls back to UnoCSS's default 14px and screens hand-code
// `font-size`. Source of truth for every value: the mockup spec
// (docs/mockups/Zen-Sumi Design System/colors_and_type.css).
export default defineConfig({
	presets: [presetRokkit(config)],
	safelist: ['i-auth-magic', 'i-auth-email', 'i-auth-github', 'i-auth-google'],
	theme: {
		// UnoCSS tuple-short form `[fontSize, lineHeight]` — the object form emits
		// the JS key `lineHeight:` literally into CSS (invalid property) and floods
		// the build with [unsupported-css-property] warnings.
		fontSize: {
			xs: ['11px', '1.4'],
			sm: ['13px', '1.5'],
			base: ['15px', '1.6'],
			lg: ['17px', '1.5'],
			xl: ['22px', '1.2'],
			'2xl': ['28px', '1.2'],
			'3xl': ['40px', '1.2'],
			'4xl': ['56px', '1.05']
		},
		letterSpacing: {
			tight: '-0.02em',
			normal: '0',
			wide: '0.18em'
		},
		lineHeight: {
			tight: '1.2',
			snug: '1.4',
			normal: '1.6',
			loose: '1.75'
		},
		borderRadius: {
			sm: '4px',
			DEFAULT: '6px',
			lg: '10px',
			full: '9999px'
		},
		transitionDuration: {
			fast: '120ms',
			DEFAULT: '180ms',
			slow: '280ms'
		},
		transitionTimingFunction: {
			DEFAULT: 'cubic-bezier(0.2, 0.6, 0.2, 1)'
		},
		boxShadow: {
			sm: '0 1px 2px oklch(var(--color-ink-z9) / 0.04)',
			DEFAULT:
				'0 1px 3px oklch(var(--color-ink-z9) / 0.06), 0 8px 24px oklch(var(--color-ink-z9) / 0.06)',
			lg: '0 24px 60px oklch(var(--color-ink-z9) / 0.18)'
		}
	}
});
