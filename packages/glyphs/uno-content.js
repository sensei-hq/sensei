/**
 * What UnoCSS scans for class names, shared by the desktop app and the dōjō
 * (#133). Its default skips plain `.js` and `.ts`, and the vocabulary's Solar
 * classes live in this package's index.js (and the app's nav data in .ts) —
 * without this, a production build ships no CSS for them and every vocabulary
 * icon renders blank.
 */
export const contentInclude = [/\.(svelte|[jt]sx?|mdx?|html)($|\?)/];
