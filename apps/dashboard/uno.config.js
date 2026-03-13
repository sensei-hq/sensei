import extractorSvelte from '@unocss/extractor-svelte'
import { defineConfig } from 'unocss'
import { presetRokkit } from '@rokkit/unocss'
import config from './rokkit.config.js'

export default defineConfig({
  extractors: [extractorSvelte()],
  presets: [presetRokkit(config)],
})
