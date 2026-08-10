import { defineConfig } from 'unocss'
import { presetRokkit } from '@rokkit/unocss'
import config from './rokkit.config.js'

// Per-product accent utilities are applied via dynamic class names
// (class="kanji text-{p.id}"), which UnoCSS can't see at build time — safelist
// them so text-/bg-/border-<product> always generate.
const PRODUCTS = ['sensei', 'torii', 'seiki', 'gateway', 'dbd', 'rokkit', 'kavach', 'magpie', 'kata', 'burne']
const productSafelist = PRODUCTS.flatMap((p) => [`text-${p}`, `bg-${p}`, `border-${p}`])

// presetRokkit already includes extractorSvelte, transformerDirectives, transformerVariantGroup
//
// `theme` mirrors app/uno.config.js (frontend-svelte-guidelines §1.8 per-surface
// config parity) so text-sm→13px, text-base→15px, … map to the design's 8-stop scale
// instead of UnoCSS's defaults. Marketing display sizes beyond text-4xl (56px) stay
// bespoke — the scale is the UI vocabulary, not the hero type.
export default defineConfig({
  presets: [presetRokkit(config)],
  safelist: ['i-auth-magic', 'i-auth-email', 'i-auth-password', ...productSafelist],
  theme: {
    fontSize: {
      xs: ['11px', '1.4'],
      sm: ['13px', '1.5'],
      base: ['15px', '1.6'],
      lg: ['17px', '1.5'],
      xl: ['22px', '1.2'],
      '2xl': ['28px', '1.2'],
      '3xl': ['40px', '1.2'],
      '4xl': ['56px', '1.05'],
    },
    letterSpacing: { tight: '-0.02em', normal: '0', wide: '0.18em' },
    lineHeight: { tight: '1.2', snug: '1.4', normal: '1.6', loose: '1.75' },
    borderRadius: { sm: '4px', DEFAULT: '6px', lg: '10px', full: '9999px' },
    transitionDuration: { fast: '120ms', DEFAULT: '180ms', slow: '280ms' },
    transitionTimingFunction: { DEFAULT: 'cubic-bezier(0.2, 0.6, 0.2, 1)' },
    boxShadow: {
      sm: '0 1px 2px oklch(var(--color-ink-z9) / 0.04)',
      DEFAULT:
        '0 1px 3px oklch(var(--color-ink-z9) / 0.06), 0 8px 24px oklch(var(--color-ink-z9) / 0.06)',
      lg: '0 24px 60px oklch(var(--color-ink-z9) / 0.18)',
    },
  },
})
