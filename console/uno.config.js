import { defineConfig } from 'unocss';
import { presetRokkit } from '@rokkit/unocss';
import config from './rokkit.config.js';

// presetRokkit already includes extractorSvelte, transformerDirectives and
// transformerVariantGroup. Safelist the kavach auth icons (rendered via
// dynamic `i-auth-{name}` class names in @kavach/ui's AuthProvider, which
// UnoCSS can't see statically).
export default defineConfig({
	presets: [presetRokkit(config)],
	safelist: ['i-auth-magic', 'i-auth-email', 'i-auth-github', 'i-auth-google']
});
