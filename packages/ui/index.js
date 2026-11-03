// packages/ui — Svelte components shared by the desktop app and the dōjō (#293).
// Reached through each app's `@sensei/ui` alias, as @sensei/glyphs is.
export { default as Screen } from './Screen.svelte';
export { default as Notice } from './Notice.svelte';
export { default as Eyebrow } from './Eyebrow.svelte';
export {
  NOTICE_TONES,
  noticeTone,
  toneForReasonKind,
  noticeMark,
  noticeFromReason,
  noticeFromApiError,
  noticeForRouteError,
  failureNotice,
} from './notice.js';
export { setGlyphMode, useGlyphMode } from './glyph-mode.js';
