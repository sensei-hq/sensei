import { defineConfig } from 'unocss'
import { presetRokkit } from '@rokkit/unocss'
import config from './rokkit.config.js'

// presetRokkit already includes extractorSvelte, transformerDirectives, transformerVariantGroup
export default defineConfig({
  presets: [presetRokkit(config)],
})
