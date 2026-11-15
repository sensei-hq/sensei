// @vitest-environment jsdom
import { describe, it, expect, afterEach, vi } from 'vitest';
import { flushSync } from 'svelte';
import { KANJI_ICONS } from '@sensei/glyphs';
import { mountComponent } from '$lib/test-mount.js';
import NoticeHarness from './Notice.harness.svelte';

// The shared Notice (packages/ui/Notice.svelte): every tone in every size, in
// both glyph modes, with and without actions, a script and a code.

let cleanup: Array<() => void> = [];
afterEach(() => { cleanup.forEach((fn) => fn()); cleanup = []; });

function mount(props: Record<string, unknown>) {
  const m = mountComponent(NoticeHarness, props);
  cleanup.push(m.destroy);
  const root = m.container.querySelector('[data-component="notice"]') as HTMLElement;
  const part = (name: string) => root.querySelector(`[data-part="${name}"]`) as HTMLElement | null;
  const act = (name: string) => root.querySelector(`[data-action="${name}"]`) as HTMLElement | null;
  return { ...m, root, part, act };
}
const text = (el: Element | null) => (el?.textContent ?? '').replace(/\s+/g, ' ').trim();

const TONES = [
  ['fault', '難', 'text-danger', 'alert'],
  ['refusal', '止', 'text-warning', 'alert'],
  ['action', '手', 'text-accent', 'status'],
  ['normal', '待', 'text-ink-soft', 'status'],
] as const;
const SIZES = ['screen', 'panel', 'inline'] as const;

describe('Notice · tone', () => {
  for (const size of SIZES) {
    for (const [tone, kanji, cls, role] of TONES) {
      it(`${size} · ${tone}: ${kanji}, ${cls}, role=${role}`, () => {
        const n = mount({ size, tone, title: 'Something to say', remedy: 'Do this' });
        expect(n.root.dataset.size).toBe(size);
        expect(n.root.dataset.tone).toBe(tone);
        expect(n.root.getAttribute('role')).toBe(role);
        const mark = n.part('mark')!;
        expect(mark.dataset.glyph).toBe('kanji');
        expect(mark.textContent).toBe(kanji);
        expect(mark.className).toMatch(new RegExp(`\\b${cls}\\b`));
        expect(mark.getAttribute('aria-hidden')).toBe('true');
      });
    }
  }

  it('defaults to a fault in the panel size', () => {
    const n = mount({ title: 'Broke' });
    expect(n.root.dataset.tone).toBe('fault');
    expect(n.root.dataset.size).toBe('panel');
  });
});

describe('Notice · glyph mode (#133)', () => {
  for (const size of SIZES) {
    for (const [tone, kanji] of TONES) {
      it(`${size} · ${tone}: icon mode shows the vocabulary icon for ${kanji}, not the kanji`, () => {
        const n = mount({ size, tone, title: 't', remedy: 'r', glyphs: 'icon' });
        const mark = n.part('mark')!;
        expect(mark.dataset.glyph).toBe('icon');
        expect(mark.className).toContain(KANJI_ICONS[kanji]);
        expect(mark.textContent).toBe('');
      });
    }
  }

  it('defaults to kanji when nothing provides a mode', async () => {
    const { Notice } = await import('@sensei/ui');
    const m = mountComponent(Notice as never, { title: 'No provider' } as never);
    cleanup.push(m.destroy);
    expect(m.container.querySelector('[data-part="mark"]')!.textContent).toBe('難');
  });

  it('a mode prop overrides the provided mode', () => {
    expect(mount({ title: 't', glyphs: 'icon', mode: 'kanji' }).part('mark')!.textContent).toBe('難');
    expect(mount({ title: 't', glyphs: 'kanji', mode: 'icon' }).part('mark')!.dataset.glyph).toBe('icon');
  });

  it('a custom mark follows the mode too', () => {
    expect(mount({ title: 't', mark: '連' }).part('mark')!.textContent).toBe('連');
    expect(mount({ title: 't', mark: '連', glyphs: 'icon' }).part('mark')!.className).toContain(KANJI_ICONS['連']);
  });

  it('an explicit icon is used in icon mode', () => {
    const n = mount({ title: 't', glyphs: 'icon', icon: 'i-solar-wifi-router-bold-duotone' });
    expect(n.part('mark')!.className).toContain('i-solar-wifi-router-bold-duotone');
  });
});

describe('Notice · screen', () => {
  const props = {
    size: 'screen', eyebrow: "couldn't load", title: "Sensei can't reach", emphasis: 'the daemon.',
    detail: 'Nothing you entered was lost.', remedy: 'Open Health to see which part is down.',
    code: 'daemon_unreachable', primary: { label: 'Retry' }, secondary: { label: 'Open Health', href: '/health' },
  };

  it('reads as the bootstrap: eyebrow, headline with a toned word, sentence, card', () => {
    const n = mount(props);
    expect(text(n.part('eyebrow'))).toBe("couldn't load");
    expect(n.part('title')!.tagName).toBe('H2');
    expect(text(n.part('title'))).toBe("Sensei can't reach the daemon.");
    expect(n.part('emphasis')!.className).toMatch(/\btext-danger\b/);
    expect(text(n.part('detail'))).toBe('Nothing you entered was lost.');
    expect(text(n.part('heading'))).toBe('What to do');
    expect(text(n.part('remedy'))).toBe('Open Health to see which part is down.');
    expect(text(n.part('code'))).toBe('daemon_unreachable');
  });

  it('names the card with remedyTitle', () => {
    expect(text(mount({ ...props, remedyTitle: 'Check the daemon, then retry' }).part('heading')))
      .toBe('Check the daemon, then retry');
  });

  it('has no card when there is nothing to do or show', () => {
    expect(mount({ size: 'screen', title: 'Not due yet', tone: 'normal' }).part('card')).toBeNull();
  });

  it('matches its snapshot', () => {
    expect(mount(props).root.outerHTML).toMatchSnapshot();
  });
});

describe('Notice · panel', () => {
  const props = {
    tone: 'refusal', title: 'Signed in as the wrong GitHub account',
    detail: 'GitHub answered as @jerry-seneca, but this identity is @jt-oss. Nothing was saved.',
    remedy: 'Open the sign-in link in a private window.', code: 'account_mismatch',
    primary: { label: 'Copy link for a private window' }, secondary: { label: 'Try again' },
  };

  it('the title heads the card; the body says why and what to do', () => {
    const n = mount(props);
    expect(n.part('heading')!.tagName).toBe('H3');
    expect(text(n.part('heading'))).toBe(props.title);
    expect(text(n.part('detail'))).toBe(props.detail);
    expect(text(n.part('remedy'))).toBe(`What to do: ${props.remedy}`);
    expect(text(n.part('code'))).toBe('account_mismatch');
  });

  it('says who can fix it when that is not the reader', () => {
    const n = mount({ ...props, actor: "Only the organization's admins can change this." });
    expect(text(n.part('actor'))).toBe("Only the organization's admins can change this.");
  });

  it('has no body, no code and no footer when given only a title', () => {
    const n = mount({ title: 'Broke' });
    expect(n.part('body')).toBeNull();
    expect(n.part('code')).toBeNull();
    expect(n.part('actions')).toBeNull();
  });

  it('matches its snapshot', () => {
    expect(mount(props).root.outerHTML).toMatchSnapshot();
  });
});

describe('Notice · inline', () => {
  const props = {
    size: 'inline', tone: 'refusal', title: 'Turned off by an org admin.',
    detail: "senecaglobal's admin keeps this repository out of the dōjō.", remedy: 'Ask them to turn it on.',
    code: 'admin_disabled',
  };

  it('starts closed: the title and a Why? toggle', () => {
    const n = mount(props);
    expect(text(n.part('title'))).toBe(props.title);
    expect(n.part('more')).toBeNull();
    expect(text(n.act('toggle'))).toBe('Why?');
    expect(n.act('toggle')!.getAttribute('aria-expanded')).toBe('false');
  });

  it('Why? opens the detail, remedy and code; Hide closes them', () => {
    const n = mount(props);
    n.act('toggle')!.click();
    flushSync();
    expect(n.act('toggle')!.getAttribute('aria-expanded')).toBe('true');
    expect(text(n.act('toggle'))).toBe('Hide');
    expect(text(n.part('detail'))).toBe(props.detail);
    expect(text(n.part('remedy'))).toBe('What to do: Ask them to turn it on.');
    expect(text(n.part('code'))).toBe('admin_disabled');
    n.act('toggle')!.click();
    flushSync();
    expect(n.part('more')).toBeNull();
  });

  it('can start open', () => {
    expect(mount({ ...props, expanded: true }).part('more')).not.toBeNull();
  });

  it('has no toggle when there is nothing more to say', () => {
    expect(mount({ size: 'inline', title: "Couldn't load linked tasks." }).act('toggle')).toBeNull();
  });

  it('shows the primary action on the line, as a link', () => {
    const onclick = vi.fn();
    const n = mount({ size: 'inline', title: "Couldn't load linked tasks.", primary: { label: 'Retry', onclick } });
    expect(n.act('primary')!.dataset.style).toBe('link');
    n.act('primary')!.click();
    expect(onclick).toHaveBeenCalledOnce();
  });

  it('boxed by default; bare drops the box for a line inside a bordered row', () => {
    expect(mount(props).root.className).toMatch(/\bborder\b/);
    expect(mount({ ...props, bare: true }).root.className).not.toMatch(/\bborder\b/);
  });

  it('matches its snapshot', () => {
    expect(mount({ ...props, expanded: true }).root.outerHTML).toMatchSnapshot();
  });
});

describe('Notice · actions', () => {
  it('primary is solid, secondary outline; both call their handlers', () => {
    const one = vi.fn();
    const two = vi.fn();
    const n = mount({ title: 't', primary: { label: 'Retry', onclick: one }, secondary: { label: 'Cancel', onclick: two } });
    // rokkit's Button: the theme owns how primary and outline look.
    expect(n.act('primary')!.dataset.variant).toBe('primary');
    expect(n.act('secondary')!.dataset.style).toBe('outline');
    n.act('primary')!.click();
    n.act('secondary')!.click();
    expect(one).toHaveBeenCalledOnce();
    expect(two).toHaveBeenCalledOnce();
  });

  it('names its data-action hook when asked (Retry keeps data-action="retry")', () => {
    expect(mount({ title: 't', primary: { label: 'Retry', name: 'retry' } }).act('retry')).not.toBeNull();
  });

  it('an external link opens in the browser, not the webview', () => {
    const a = mount({ title: 't', secondary: { label: 'Learn more', href: 'https://brew.sh', external: true } }).act('secondary')!;
    expect(a.getAttribute('target')).toBe('_blank');
    expect(a.getAttribute('rel')).toContain('noopener');
  });

  it('an href renders a link, not a button', () => {
    const a = mount({ title: 't', primary: { label: 'See plans', href: '/settings/plan' } }).act('primary')!;
    expect(a.tagName).toBe('A');
    expect(a.getAttribute('href')).toBe('/settings/plan');
  });

  it('a busy action is disabled so it cannot be pressed twice', () => {
    const onclick = vi.fn();
    const b = mount({ title: 't', primary: { label: 'Retry', onclick, busy: true } }).act('primary') as HTMLButtonElement;
    expect(b.disabled).toBe(true);
    b.click();
    expect(onclick).not.toHaveBeenCalled();
  });
});

describe('Notice · script', () => {
  it('shows the script and copies it', async () => {
    const writeText = vi.fn(async () => {});
    const n = mount({ tone: 'action', title: 'One last step', script: 'brew bundle', writeText });
    expect(n.part('script')!.textContent).toContain('brew bundle');
    n.act('copy')!.click();
    await vi.waitFor(() => expect(n.act('copy')!.dataset.state).toBe('copied'));
    expect(writeText).toHaveBeenCalledWith('brew bundle');
    expect(text(n.act('copy'))).toBe('Copied');
  });

  it('says so when the clipboard refuses, so the reader selects it by hand', async () => {
    const writeText = vi.fn(async () => { throw new Error('denied'); });
    const n = mount({ title: 't', script: 'x', writeText });
    n.act('copy')!.click();
    await vi.waitFor(() => expect(n.act('copy')!.dataset.state).toBe('failed'));
  });

  it('shows the script inside an opened inline notice too', () => {
    const n = mount({ size: 'inline', title: 't', script: 'sensei doctor', expanded: true });
    expect(n.part('script')!.textContent).toContain('sensei doctor');
  });
});

describe('Notice · attributes', () => {
  it('takes an id and tabindex, so a failure can take focus and describe a field', () => {
    const n = mount({ title: 't', id: 'tracker-alert', tabindex: -1 });
    expect(n.root.id).toBe('tracker-alert');
    expect(n.root.getAttribute('tabindex')).toBe('-1');
  });

  it('passes data-* hooks to the root, as Screen does', () => {
    expect(mount({ title: 't', 'data-screen-state': 'error' }).root.dataset.screenState).toBe('error');
  });
});
