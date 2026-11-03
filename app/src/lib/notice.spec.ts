import { describe, it, expect } from 'vitest';
import { KANJI_ICONS } from '@sensei/glyphs';
import {
  NOTICE_TONES,
  noticeTone,
  toneForReasonKind,
  noticeMark,
  noticeFromReason,
  noticeFromApiError,
  noticeForRouteError,
  failureNotice,
  loadFailedNotice,
  stateErrorNotice,
  BANNER_TONES,
} from '@sensei/ui';

// packages/ui/notice.js — what a Notice says and how it looks, decided once so
// every screen in sensei and the dōjō reads the same situation the same way.

describe('noticeTone', () => {
  it.each([
    ['fault', '難', 'text-danger', 'alert'],
    ['refusal', '止', 'text-warning', 'alert'],
    ['action', '手', 'text-accent', 'status'],
    ['normal', '待', 'text-ink-soft', 'status'],
  ] as const)('%s → %s, %s, role=%s', (tone, kanji, text, role) => {
    expect(noticeTone(tone)).toEqual({ kanji, text, role });
  });

  it('reads an unknown tone as a fault, never as a quiet normal', () => {
    expect(noticeTone('surprise')).toBe(NOTICE_TONES.fault);
  });

  it('gives every tone a kanji the vocabulary has an icon for (#133)', () => {
    for (const { kanji } of Object.values(NOTICE_TONES)) expect(KANJI_ICONS[kanji], kanji).toBeTruthy();
  });
});

describe('toneForReasonKind', () => {
  it.each(['normal', 'refusal', 'fault'] as const)('passes %s straight through', (kind) => {
    expect(toneForReasonKind(kind)).toBe(kind);
  });

  it.each([null, undefined, '', 'new_kind'])('reads %s as a fault', (kind) => {
    expect(toneForReasonKind(kind)).toBe('fault');
  });
});

describe('noticeMark', () => {
  it('shows the kanji in kanji mode', () => {
    expect(noticeMark('refusal', 'kanji')).toEqual({ mode: 'kanji', char: '止' });
  });

  it("shows the kanji's vocabulary icon in icon mode", () => {
    expect(noticeMark('normal', 'icon')).toEqual({ mode: 'icon', icon: KANJI_ICONS['待'] });
  });

  it('honours a mark override in both modes', () => {
    expect(noticeMark('fault', 'kanji', { char: '連' })).toEqual({ mode: 'kanji', char: '連' });
    expect(noticeMark('fault', 'icon', { char: '連' })).toEqual({ mode: 'icon', icon: KANJI_ICONS['連'] });
  });

  it('prefers an explicit icon in icon mode only', () => {
    const icon = 'i-solar-wifi-router-bold-duotone';
    expect(noticeMark('fault', 'icon', { icon })).toEqual({ mode: 'icon', icon });
    expect(noticeMark('fault', 'kanji', { icon })).toEqual({ mode: 'kanji', char: '難' });
  });

  it('keeps a kanji with no icon as a kanji in icon mode — never an empty mark', () => {
    expect(noticeMark('fault', 'icon', { char: '龘' })).toEqual({ mode: 'kanji', char: '龘' });
  });
});

describe('noticeFromReason', () => {
  const row = {
    domain: 'repository_sharing',
    code: 'not_subscribed',
    kind: 'refusal',
    summary: 'Needs the Org plan',
    detail: "Private repositories of an organization sync once it's on the Org plan.",
    remedy: 'Ask an admin to attach the Org plan in Plan & billing.',
    actor: 'organization',
  };

  it("uses the registry's own words, so a reason reads the same on every screen", () => {
    expect(noticeFromReason(row)).toEqual({
      tone: 'refusal',
      title: 'Needs the Org plan',
      detail: row.detail,
      remedy: row.remedy,
      actor: "Only the organization's admins can change this.",
      code: 'repository_sharing/not_subscribed',
    });
  });

  it('says nothing about who can act when the reader can', () => {
    expect(noticeFromReason({ ...row, actor: 'user' }).actor).toBeUndefined();
  });

  it('drops absent fields rather than rendering empty lines', () => {
    const n = noticeFromReason({ code: 'not_due', kind: 'normal', summary: 'Not due yet', detail: null, remedy: null, actor: null });
    expect(n).toEqual({ tone: 'normal', title: 'Not due yet', detail: undefined, remedy: undefined, actor: undefined, code: 'not_due' });
  });
});

describe('noticeFromApiError', () => {
  it('status 0: the far end was never reached — say what to check', () => {
    expect(noticeFromApiError({ status: 0, message: 'Failed to fetch' }, { title: "Couldn't load tickets", service: 'the sensei daemon' }))
      .toEqual({
        tone: 'fault',
        title: "Couldn't load tickets",
        detail: 'Failed to fetch',
        remedy: 'Check that the sensei daemon is running and reachable, then retry.',
        code: 'unreachable',
      });
  });

  it('5xx: the server failed — a retry may clear it', () => {
    const n = noticeFromApiError({ status: 503, message: 'Service Unavailable' }, { title: "Couldn't save" });
    expect(n.code).toBe('http_503');
    expect(n.remedy).toMatch(/Retry in a moment/);
  });

  it('401/403/404 are refusals, not faults: nothing broke, and each names the next step', () => {
    const missing = noticeFromApiError({ status: 404, message: 'No such workspace' }, { title: "Couldn't open it" });
    expect(missing).toMatchObject({ tone: 'refusal', detail: 'No such workspace', code: 'http_404' });
    expect(missing.remedy).toBe('It may have moved or been removed. Go back and pick it again.');
    expect(noticeFromApiError({ status: 403, message: 'admins only' }, { title: 'x' })).toMatchObject({
      tone: 'refusal',
      remedy: 'Ask an admin for access.',
    });
    expect(noticeFromApiError({ status: 401, message: '' }, { title: 'x' })).toMatchObject({
      tone: 'refusal',
      remedy: 'Sign in again, then retry.',
    });
  });

  it("another 4xx: the server's message is the whole story — no invented remedy", () => {
    const n = noticeFromApiError({ status: 409, message: 'Already exists' }, { title: "Couldn't save" });
    expect(n).toMatchObject({ tone: 'fault', detail: 'Already exists', code: 'http_409' });
    expect(n.remedy).toBeUndefined();
  });

  it('defaults the far end to "the server"', () => {
    expect(noticeFromApiError({ status: 0, message: '' }, { title: 'x' }).remedy).toMatch(/^Check that the server/);
  });
});

describe('noticeForRouteError', () => {
  it("404: the page isn't here — go home, nothing broke", () => {
    expect(noticeForRouteError(404, 'Not Found')).toEqual({
      tone: 'refusal',
      eyebrow: 'not found',
      title: "This page isn't",
      emphasis: 'here.',
      detail: 'The link may be old, or the item was removed.',
      code: 'http_404',
    });
  });

  it('403: not allowed — says so rather than calling it a fault', () => {
    const n = noticeForRouteError(403, 'Forbidden');
    expect(n.tone).toBe('refusal');
    expect(n.eyebrow).toBe('not allowed');
    expect(n.detail).toBe('Forbidden');
  });

  it("anything else: the page broke — the message is the reason, a reload may clear it", () => {
    expect(noticeForRouteError(500, 'Internal Error')).toEqual({
      tone: 'fault',
      eyebrow: 'something broke',
      title: "This page couldn't",
      emphasis: 'open.',
      detail: 'Internal Error',
      remedy: 'Reload the page. If it keeps failing, the logs will say why.',
      code: 'http_500',
    });
  });

  it('an empty message gets a sentence, never a blank', () => {
    expect(noticeForRouteError(500, '').detail).toBe('No reason was given.');
  });
});

describe('failureNotice — a state module\'s failure, said the one way', () => {
  it('no error, no notice', () => {
    expect(failureNotice("Couldn't load tickets", null)).toBeNull();
    expect(failureNotice("Couldn't load tickets", '')).toBeNull();
  });

  it('the screen names what failed; the error is the reason', () => {
    expect(failureNotice("Couldn't load tickets", 'The sensei daemon didn’t respond.')).toEqual({
      tone: 'fault', title: "Couldn't load tickets", detail: 'The sensei daemon didn’t respond.',
    });
  });

  it('takes a remedy and code when the screen knows them', () => {
    expect(failureNotice('x', 'y', { remedy: 'Retry.', code: 'http_503' })).toEqual({
      tone: 'fault', title: 'x', detail: 'y', remedy: 'Retry.', code: 'http_503',
    });
  });
});

describe('loadFailedNotice — a read that failed, named by what it was', () => {
  it('names what failed, then why, and always says what to do', () => {
    expect(loadFailedNotice('the triage queue', 'forbidden')).toEqual({
      tone: 'fault',
      title: "Couldn't load the triage queue",
      detail: 'forbidden',
      remedy: 'Retry. If it keeps failing, reload the page.',
    });
  });

  it('with no reason given, still says something', () => {
    expect(loadFailedNotice('your metrics', '').detail).toBe('No answer came back.');
  });

  it('takes the screen\'s own remedy when it has a better one', () => {
    expect(loadFailedNotice('x', 'y', { remedy: 'Check the daemon.' }).remedy).toBe('Check the daemon.');
  });
});

describe('stateErrorNotice — what a failed screen says', () => {
  it('names the failure generically, with the reason and a remedy — even when there is a reason', () => {
    expect(stateErrorNotice('daemon unreachable', { remedy: 'Check the daemon.' })).toEqual({
      tone: 'fault',
      eyebrow: "couldn't load",
      title: "Couldn't load this",
      detail: 'daemon unreachable',
      remedy: 'Check the daemon.',
    });
  });

  it("a screen's own notice wins, keeping the eyebrow and the default remedy when it has none", () => {
    expect(stateErrorNotice(null, { notice: { title: "Couldn't load tickets", tone: 'refusal' }, remedy: 'Retry.' })).toEqual({
      tone: 'refusal',
      eyebrow: "couldn't load",
      title: "Couldn't load tickets",
      remedy: 'Retry.',
    });
  });

  it('with no remedy anywhere, still names the next step', () => {
    expect(stateErrorNotice('x').remedy).toBe('Retry. If it keeps failing, reload the page.');
  });
});

describe('BANNER_TONES — a strip per Notice tone, text on its tint from text-on-*-soft', () => {
  it('has the Notice tones, each on named tokens', () => {
    expect(Object.keys(BANNER_TONES)).toEqual(['fault', 'refusal', 'action', 'normal']);
    for (const t of Object.values(BANNER_TONES)) {
      expect(t.bg).toMatch(/^bg-[a-z-]+$/);
      expect(t.edge).toMatch(/^border-[a-z-]+$/);
      expect(t.mark).toMatch(/^text-[a-z-]+$/);
    }
    expect(BANNER_TONES.fault).toEqual({ bg: 'bg-danger-soft', edge: 'border-danger-soft', mark: 'text-on-danger-soft' });
    expect(BANNER_TONES.normal).toEqual({ bg: 'bg-paper-soft', edge: 'border-paper-edge', mark: 'text-ink-soft' });
  });
});
