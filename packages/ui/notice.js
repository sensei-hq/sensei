// packages/ui/notice.js — what a Notice says and how it looks, decided once.
//
// The Notice is the bootstrap's "needs your hand" screen made reusable: an
// eyebrow, a display headline with one toned word, a plain sentence, then the
// Remedy card (a mark and a title, what to do, an optional script, the actions).
// Every error, refusal and wait in sensei and the dōjō renders through it, so the
// same situation reads the same everywhere.
//
// ## Four tones
//
// Three are `sensei.reason_kind` (docs/architecture/reason-codes.md) — a reason
// row maps straight onto a Notice without a second vocabulary. The fourth,
// `action`, is the bootstrap's own: nothing is broken, it just needs the reader.
//
//   fault    難  danger   something broke; always offers a retry or a remedy
//   refusal  止  warning  someone decided this; says who can change it
//   action   手  accent   fine once you act (run a script, renew, sign in)
//   normal   待  muted    nothing is wrong and it resolves itself
//
// A fault or refusal is announced (`role="alert"`); action and normal are not
// emergencies and are `role="status"`, so a screen reader is not shouted at
// about a schedule that simply is not due.

import { iconForKanji } from '@sensei/glyphs';

/** @typedef {'fault' | 'refusal' | 'action' | 'normal'} NoticeTone */
/** @typedef {'screen' | 'panel' | 'inline'} NoticeSize */
/** @typedef {'kanji' | 'icon'} GlyphMode */
/** @typedef {{ kanji: string, text: string, role: 'alert' | 'status' }} ToneView */
/** @typedef {{ mode: 'kanji', char: string } | { mode: 'icon', icon: string }} NoticeMark */

/** @type {Record<NoticeTone, ToneView>} */
export const NOTICE_TONES = {
  fault: { kanji: '難', text: 'text-danger', role: 'alert' },
  refusal: { kanji: '止', text: 'text-warning', role: 'alert' },
  action: { kanji: '手', text: 'text-accent', role: 'status' },
  normal: { kanji: '待', text: 'text-ink-soft', role: 'status' },
};

/**
 * Pure: a tone's mark, colour and live-region role. An unknown tone reads as a
 * FAULT, not as normal: a Notice exists to say something went wrong, and
 * quietly muting a value we did not expect is how a failure becomes invisible.
 * @param {string} tone
 * @returns {ToneView}
 */
export function noticeTone(tone) {
  return NOTICE_TONES[/** @type {NoticeTone} */ (tone)] ?? NOTICE_TONES.fault;
}

/**
 * Pure: `sensei.reason_kind` → a Notice tone. The three kinds are three of the
 * tones by name; anything else is a fault (see `noticeTone`).
 * @param {string | null | undefined} kind
 * @returns {NoticeTone}
 */
export function toneForReasonKind(kind) {
  return kind === 'normal' || kind === 'refusal' || kind === 'fault' ? kind : 'fault';
}

/**
 * Pure: what the mark shows in this glyph mode. Kanji mode shows the kanji;
 * icon mode shows `icon` if given, else the vocabulary's icon for the kanji.
 * A kanji with no icon stays a kanji even in icon mode — never an empty slot.
 * @param {string} tone
 * @param {GlyphMode} mode
 * @param {{ char?: string | null, icon?: string | null }} [override]
 * @returns {NoticeMark}
 */
export function noticeMark(tone, mode, override = {}) {
  const char = override.char || noticeTone(tone).kanji;
  if (mode === 'icon') {
    const icon = override.icon || iconForKanji(char);
    if (icon) return { mode: 'icon', icon };
  }
  return { mode: 'kanji', char };
}

/** Who can act, in words. `null` when the reader can (it goes without saying). */
const ACTOR_LINES = {
  organization: "Only the organization's admins can change this.",
};

/**
 * Pure: one `sensei.reason_codes` row (as the API joins it) → Notice props.
 * The registry's text is the copy — a screen never keeps its own wording of a
 * reason, so the same code reads the same on every screen that meets it.
 * @param {{ domain?: string | null, code: string, kind?: string | null, summary: string,
 *           detail?: string | null, remedy?: string | null, actor?: string | null }} row
 */
export function noticeFromReason(row) {
  return {
    tone: toneForReasonKind(row.kind),
    title: row.summary,
    detail: row.detail ?? undefined,
    remedy: row.remedy ?? undefined,
    actor: (row.actor && ACTOR_LINES[/** @type {keyof typeof ACTOR_LINES} */ (row.actor)]) || undefined,
    code: row.domain ? `${row.domain}/${row.code}` : row.code,
  };
}

/**
 * Pure: a failed API read → Notice props. `title` names WHAT failed ("Couldn't
 * load tickets") because only the screen knows; the error supplies why and what
 * to do. Status 0 is "never reached the server" — the daemon or the network,
 * which is what the reader can check; a 5xx is the server's fault and a retry
 * may clear it; anything else is shown as the server said it.
 * @param {{ status: number, message: string }} error
 * @param {{ title: string, service?: string }} what  `service` names the far end
 *        ("the daemon", "the dōjō") in the remedy for an unreachable server.
 */
export function noticeFromApiError(error, what) {
  const service = what.service ?? 'the server';
  const remedy =
    error.status === 0
      ? `Check that ${service} is running and reachable, then retry.`
      : error.status >= 500
        ? 'This is on the server side. Retry in a moment; if it keeps failing, the logs will say why.'
        : undefined;
  return {
    tone: /** @type {NoticeTone} */ ('fault'),
    title: what.title,
    detail: error.message || undefined,
    remedy,
    code: error.status === 0 ? 'unreachable' : `http_${error.status}`,
  };
}

/**
 * Pure: a route that failed to load (SvelteKit's `+error.svelte`) → screen-size
 * Notice props. A missing page and a forbidden one are refusals — nothing broke,
 * and a red "fault" would say otherwise; everything else is a fault the reader
 * may clear by reloading.
 * @param {number} status
 * @param {string | null | undefined} message
 */
export function noticeForRouteError(status, message) {
  const code = `http_${status}`;
  if (status === 404) {
    return {
      tone: /** @type {NoticeTone} */ ('refusal'),
      eyebrow: 'not found',
      title: "This page isn't",
      emphasis: 'here.',
      detail: 'The link may be old, or the item was removed.',
      code,
    };
  }
  if (status === 401 || status === 403) {
    return {
      tone: /** @type {NoticeTone} */ ('refusal'),
      eyebrow: 'not allowed',
      title: "You can't open",
      emphasis: 'this page.',
      detail: message || 'Your account does not have access to it.',
      code,
    };
  }
  return {
    tone: /** @type {NoticeTone} */ ('fault'),
    eyebrow: 'something broke',
    title: "This page couldn't",
    emphasis: 'open.',
    detail: message || 'No reason was given.',
    remedy: 'Reload the page. If it keeps failing, the logs will say why.',
    code,
  };
}

/**
 * Pure: a state module's failure → Notice props, or `null` when nothing failed.
 * The screen names WHAT failed ("Couldn't load tickets"); the error it already
 * holds is the reason — never the headline.
 * @param {string} title
 * @param {string | null | undefined} error
 * @param {{ remedy?: string, code?: string }} [extra]
 */
export function failureNotice(title, error, extra = {}) {
  if (!error) return null;
  return { tone: /** @type {NoticeTone} */ ('fault'), title, detail: error, ...extra };
}
