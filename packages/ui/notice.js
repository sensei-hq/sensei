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

/** What every failed read says to do when the screen has nothing better. */
export const DEFAULT_REMEDY = 'Retry. If it keeps failing, reload the page.';

/** A refusal names its own next step: nothing broke, the reader may not, or the
 *  thing is not there. A 404 is a refusal only for an entity lookup. */
const REFUSAL_REMEDY = {
  401: 'Sign in again, then retry.',
  403: 'Ask an admin for access.',
};
const MISSING_REMEDY = 'It may have moved or been removed. Go back and pick it again.';

/**
 * Pure: a failed API read → Notice props. `title` names WHAT failed ("Couldn't
 * load tickets") because only the screen knows; the error supplies why and what
 * to do — always something to do:
 * - 401/403: a refusal (sign in, ask an admin).
 * - 404 on an entity lookup (`entity: true`): a refusal — it moved or was removed.
 *   A 404 on any other read is a route the far end does not know: a version
 *   mismatch, a fault, and the fix is to update (M2).
 * - 0: the far end was never reached — the daemon or the network.
 * - 5xx: the server's fault; a retry may clear it.
 * - any other 4xx: the message says what is wrong; fix it and try again.
 * @param {{ status: number, message: string }} error
 * @param {{ title: string, service?: string, entity?: boolean }} what  `service`
 *        names the far end ("the sensei daemon", "the dōjō").
 */
export function noticeFromApiError(error, what) {
  const service = what.service ?? 'the server';
  const s = error.status;
  const refusal = s === 401 || s === 403 || (s === 404 && what.entity === true);
  const remedy =
    s === 401 || s === 403
      ? REFUSAL_REMEDY[s]
      : s === 404
        ? what.entity
          ? MISSING_REMEDY
          : `${capitalise(service)} doesn't know this request. Update Sensei so the app and ${service} match, then retry.`
        : s === 0
          ? `Check that ${service} is running and reachable, then retry.`
          : s >= 500
            ? 'This is on the server side. Retry in a moment; if it keeps failing, the logs will say why.'
            : 'Fix what it says, then try again.';
  return {
    tone: /** @type {NoticeTone} */ (refusal ? 'refusal' : 'fault'),
    title: what.title,
    detail: error.message || undefined,
    remedy,
    code: s === 0 ? 'unreachable' : `http_${s}`,
  };
}

/** @param {string} s */
function capitalise(s) {
  return s.charAt(0).toUpperCase() + s.slice(1);
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
      remedy: 'Go back, or start again from home.',
      code,
    };
  }
  if (status === 401) {
    return {
      tone: /** @type {NoticeTone} */ ('refusal'),
      eyebrow: 'signed out',
      title: 'Sign in to',
      emphasis: 'open this page.',
      detail: 'Your session ended, or you have not signed in here yet.',
      remedy: 'Sign in again.',
      code,
    };
  }
  if (status === 403) {
    return {
      tone: /** @type {NoticeTone} */ ('refusal'),
      eyebrow: 'not allowed',
      title: "You can't open",
      emphasis: 'this page.',
      detail: message || 'Your account does not have access to it.',
      remedy: REFUSAL_REMEDY[403],
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
 * holds is the reason — never the headline. It always names a next step: the
 * screen's `remedy`, else DEFAULT_REMEDY (H2).
 * @param {string} title
 * @param {string | null | undefined} error
 * @param {{ remedy?: string, code?: string }} [extra]
 */
export function failureNotice(title, error, extra = {}) {
  if (!error) return null;
  return { tone: /** @type {NoticeTone} */ ('fault'), title, detail: error, remedy: DEFAULT_REMEDY, ...extra };
}


/**
 * Pure: a read that failed, named by what it was ("the triage queue") → Notice
 * props. Always a next step. With the failed read itself (`{status, message}`)
 * it is said through noticeFromApiError, so a 401/403 reads as a refusal; with
 * only a message, a fault with the screen's remedy or DEFAULT_REMEDY.
 * @param {string} what
 * @param {string | { status: number, message: string } | null | undefined} error
 * @param {{ remedy?: string, code?: string, service?: string }} [extra]
 */
export function loadFailedNotice(what, error, extra = {}) {
  const title = `Couldn't load ${what}`;
  if (error && typeof error === 'object') {
    const n = noticeFromApiError(error, { title, service: extra.service });
    return { ...n, detail: n.detail ?? 'No answer came back.', remedy: extra.remedy ?? n.remedy, ...(extra.code ? { code: extra.code } : {}) };
  }
  const { service: _service, ...rest } = extra;
  return {
    tone: /** @type {NoticeTone} */ ('fault'),
    title,
    detail: error || 'No answer came back.',
    remedy: DEFAULT_REMEDY,
    ...rest,
  };
}

/**
 * Pure: what StateView's error branch says. A screen's own notice (from its
 * state module) wins; otherwise the generic title with the reason. Either way it
 * names the next step — the app's default remedy (`setStateDefaults`), else
 * DEFAULT_REMEDY.
 * @param {string | null | undefined} error
 * @param {{ notice?: { tone?: NoticeTone, eyebrow?: string, title: string, detail?: string, remedy?: string, code?: string }, remedy?: string,
 *           failure?: { status: number, message: string } | null, service?: string }} [opts]  `failure` is the read
 *        that failed, status and all: it is said through noticeFromApiError, so a 403 reads as a refusal.
 */
export function stateErrorNotice(error, opts = {}) {
  const remedy = opts.remedy ?? DEFAULT_REMEDY;
  if (opts.notice) {
    return { tone: /** @type {NoticeTone} */ ('fault'), eyebrow: "couldn't load", ...opts.notice, remedy: opts.notice.remedy ?? remedy };
  }
  if (opts.failure) {
    const n = noticeFromApiError(opts.failure, { title: "Couldn't load this", service: opts.service });
    return { tone: n.tone, eyebrow: "couldn't load", title: n.title, ...(n.detail ? { detail: n.detail } : {}), remedy: n.remedy ?? remedy, code: n.code };
  }
  return {
    tone: /** @type {NoticeTone} */ ('fault'),
    eyebrow: "couldn't load",
    title: "Couldn't load this",
    ...(error ? { detail: error } : {}),
    remedy,
  };
}

/** A strip (Banner) per Notice tone: its tint, its edge, its mark's colour and its words'.
 *  Text on a tint is `text-on-{role}-soft` (rokkit-defaults). */
export const BANNER_TONES = {
  fault: { bg: 'bg-danger-soft', edge: 'border-danger-soft', mark: 'text-on-danger-soft', text: 'text-on-danger-soft' },
  refusal: { bg: 'bg-warning-soft', edge: 'border-warning-soft', mark: 'text-on-warning-soft', text: 'text-on-warning-soft' },
  action: { bg: 'bg-accent-soft', edge: 'border-accent-soft', mark: 'text-on-accent-soft', text: 'text-on-accent-soft' },
  normal: { bg: 'bg-paper-soft', edge: 'border-paper-edge', mark: 'text-ink-soft', text: 'text-ink' },
};
