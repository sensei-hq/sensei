/**
 * What UnoCSS scans for class names. Its default skips plain `.ts`, and the
 * rails' Solar icon classes live in `.ts` data files (#133) — without them here
 * a production build ships no CSS for the icons. Kept apart from uno.config.js
 * so a test can load it without the preset.
 */
export const contentInclude = [/\.(svelte|[jt]sx?|mdx?|html)($|\?)/];
