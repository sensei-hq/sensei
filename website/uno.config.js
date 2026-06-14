import { defineConfig } from 'unocss'
import { presetRokkit } from '@rokkit/unocss'
import config from './rokkit.config.js'

// Per-product accent utilities are applied via dynamic class names
// (class="kanji text-{p.id}"), which UnoCSS can't see at build time — safelist
// them so text-/bg-/border-<product> always generate.
const PRODUCTS = ['sensei', 'dbd', 'rokkit', 'kavach', 'magpie', 'kata', 'burne']
const productSafelist = PRODUCTS.flatMap((p) => [`text-${p}`, `bg-${p}`, `border-${p}`])

// presetRokkit already includes extractorSvelte, transformerDirectives, transformerVariantGroup
export default defineConfig({
  presets: [presetRokkit(config)],
  safelist: ['i-auth-magic', 'i-auth-email', 'i-auth-password', ...productSafelist],
})
