// @vitest-environment jsdom
// The Crew view's parts (#424; COMPONENTS §3e RunCard, MemberGroup, MemberCard, AddMember;
// CONTRACT §1h): each draws the crew selectors' data and calls back.
import { describe, it, expect, afterEach, vi } from 'vitest';
import { tick } from 'svelte';
import { mountComponent } from '$lib/test-mount.js';
import type { RelayAddMemberView, RelayMemberGroupView, RelayMemberView, RelayRunCardView } from '@sensei/state';
import RunCard from './RunCard.svelte';
import MemberCard from './MemberCard.svelte';
import AddMember from './AddMember.svelte';

let cleanup: Array<() => void> = [];
afterEach(() => {
  cleanup.forEach((fn) => fn());
  cleanup = [];
});
function mount<P extends Record<string, unknown>>(component: unknown, props: P) {
  const m = mountComponent(component as never, props as never);
  cleanup.push(m.destroy);
  return m.container;
}
/** What a person reads: the text, without the decorative marks. */
function text(el: Element | null | undefined): string {
  if (!el) return '';
  const copy = el.cloneNode(true) as Element;
  copy.querySelectorAll('[aria-hidden="true"]').forEach((n) => n.remove());
  return (copy.textContent ?? '').replace(/\s+/g, ' ').trim();
}

const member = (over: Partial<RelayMemberView> = {}): RelayMemberView => ({
  id: 'l1',
  codename: 'cobalt-lyra',
  lane: 'Diagrams · L1',
  laneCode: 'L1',
  role: 'contributor',
  status: 'active',
  statusText: 'Active',
  family: 'Claude Code 2.1',
  model: 'claude-opus-5-5',
  alert: null,
  editor: null,
  ...over,
});

const card = (): RelayRunCardView => ({
  title: 'Run 7f3a',
  state: { label: 'running', tone: 'success' },
  meta: 'sensei · started 06:00 · coordinator atlas-rigel · completes, and is archived, when its last assigned task is done',
  facts: [
    { label: 'Members', value: 7 },
    { label: 'Tasks', value: 12 },
    { label: 'Done', value: 5 },
  ],
  draft: null,
});

const groups = (): RelayMemberGroupView[] => [
  { label: 'Coordinator', members: [member({ id: 'l0', codename: 'atlas-rigel', lane: 'Planning & review · L0', laneCode: 'L0', role: 'coordinator' })] },
  { label: 'Contributors · 1', members: [member()] },
];

describe('RunCard', () => {
  it('the title with its state, the meta line, the facts; the members by group — and no End button', () => {
    const c = mount(RunCard, { card: card(), groups: groups(), onmessage: vi.fn() });
    const root = c.querySelector('[data-run-card]')!;
    expect(text(root.querySelector('[data-run-title]'))).toBe('Run 7f3a running');
    expect(root.querySelector('[data-run-title] [data-pill]')?.getAttribute('data-tone')).toBe('success');
    expect(text(root.querySelector('[data-run-meta]'))).toContain('completes, and is archived');
    expect([...root.querySelectorAll('[data-run-fact]')].map(text)).toEqual(['Members 7', 'Tasks 12', 'Done 5']);
    expect([...root.querySelectorAll('[data-member-group]')].map((g) => text(g.querySelector('[data-member-group-label]')))).toEqual([
      'Coordinator',
      'Contributors · 1',
    ]);
    expect(text(root)).not.toMatch(/\bEnd\b/);
    expect(root.querySelector('[data-run-start]')).toBeNull();
    expect(root.querySelector('[data-run-cancel]')).toBeNull();
  });

  it('the facts wrap on a phone instead of running off the card (#420)', () => {
    const c = mount(RunCard, { card: card(), groups: groups(), onmessage: vi.fn() });
    expect(c.querySelector('[data-run-card] dl')!.classList).toContain('flex-wrap');
  });

  const draft = (over: Partial<NonNullable<RelayRunCardView['draft']>> = {}): RelayRunCardView => ({
    ...card(),
    title: 'Run ab12',
    state: { label: 'draft', tone: 'neutral' },
    facts: [{ label: 'Members', value: 2 }],
    draft: { runId: 'run-ab12', canStart: true, note: null, ...over },
  });

  it('a draft: Start and Cancel, each calling back with the run', async () => {
    const onstart = vi.fn();
    const oncancel = vi.fn();
    const c = mount(RunCard, { card: draft(), groups: groups(), onmessage: vi.fn(), onstart, oncancel });
    const start = c.querySelector<HTMLButtonElement>('[data-run-start]')!;
    expect(text(start)).toBe('Start run');
    expect(start.disabled).toBe(false);
    start.click();
    c.querySelector<HTMLButtonElement>('[data-run-cancel]')!.click();
    await tick();
    expect(onstart).toHaveBeenCalledWith('run-ab12');
    expect(oncancel).toHaveBeenCalledWith('run-ab12');
  });

  it('a draft that cannot start yet: Start disabled, the reason beside it', () => {
    const c = mount(RunCard, {
      card: draft({ canStart: false, note: 'Start it once a member is answering.' }),
      groups: groups(),
      onmessage: vi.fn(),
      onstart: vi.fn(),
      oncancel: vi.fn(),
    });
    expect(c.querySelector<HTMLButtonElement>('[data-run-start]')!.disabled).toBe(true);
    expect(text(c.querySelector('[data-run-note]'))).toBe('Start it once a member is answering.');
  });

  it('while it starts: both buttons wait; a refusal shows on the card', () => {
    const busy = mount(RunCard, { card: draft(), groups: groups(), onmessage: vi.fn(), onstart: vi.fn(), oncancel: vi.fn(), busy: true });
    expect(busy.querySelector<HTMLButtonElement>('[data-run-start]')!.disabled).toBe(true);
    expect(busy.querySelector<HTMLButtonElement>('[data-run-cancel]')!.disabled).toBe(true);
    const refused = mount(RunCard, { card: draft(), groups: groups(), onmessage: vi.fn(), onstart: vi.fn(), oncancel: vi.fn(), error: 'the crew has no live lane' });
    expect(text(refused.querySelector('[data-run-error]'))).toContain('the crew has no live lane');
  });
});

describe('MemberCard', () => {
  it('codename in its presence tone, objective · code, the family and model tags, a Message button', async () => {
    const onmessage = vi.fn();
    const c = mount(MemberCard, { member: member(), onmessage });
    const root = c.querySelector('[data-member="L1"]')!;
    const name = root.querySelector('[data-member-codename]')!;
    expect(text(name)).toBe('cobalt-lyra');
    expect(name.getAttribute('title')).toBe('Active');
    expect(name.className).toMatch(/text-success/);
    expect(text(root.querySelector('[data-member-lane]'))).toBe('Diagrams · L1');
    const tags = [...root.querySelectorAll('[data-member-tags] [data-pill]')] as HTMLElement[];
    expect(tags.map(text)).toEqual(['Claude Code 2.1', 'claude-opus-5-5']);
    // rokkit's Pill: the family is the plain chip; the model is the outline chip, in mono.
    expect(tags[0].hasAttribute('data-variant')).toBe(false);
    expect(tags[1].getAttribute('data-variant')).toBe('outline');
    expect(tags[1].className).toMatch(/\bfont-mono\b/);
    expect(tags.every((t) => t.getAttribute('data-size') === 'sm')).toBe(true);
    const message = root.querySelector('button[data-member-message]') as HTMLButtonElement;
    expect(message.getAttribute('aria-label')).toBe('Message cobalt-lyra');
    message.click();
    await tick();
    expect(onmessage).toHaveBeenCalledWith('l1');
  });

  it('not responding: the warning tone, its border, and the line under it', () => {
    const c = mount(MemberCard, { member: member({ status: 'not_responding', statusText: 'Not responding', alert: 'No answer since 10:42 · lease lost.' }), onmessage: vi.fn() });
    const root = c.querySelector('[data-member="L1"]')!;
    expect(root.getAttribute('data-status')).toBe('not_responding');
    expect(root.querySelector('[data-member-codename]')!.className).toMatch(/text-warning/);
    expect(text(root.querySelector('[data-member-alert]'))).toBe('No answer since 10:42 · lease lost.');
  });

  it('in an editor: a host tag after the model, its title saying what the editor is', () => {
    const editor = { label: 'in Cursor', title: 'Cursor is the editor; the assistant inside it does the work, and its transcripts count for claude-opus-5-5.' };
    const c = mount(MemberCard, { member: member({ editor }), onmessage: vi.fn() });
    const tags = [...c.querySelectorAll('[data-member-tags] [data-pill]')] as HTMLElement[];
    expect(tags.map(text)).toEqual(['Claude Code 2.1', 'claude-opus-5-5', 'in Cursor']);
    const host = c.querySelector('[data-member-host] [data-pill]') as HTMLElement;
    expect(host.getAttribute('title')).toBe(editor.title);
    expect(host.getAttribute('data-variant')).toBe('outline');
    expect(host.getAttribute('data-size')).toBe('sm');
  });

  it('no editor reported: no host tag', () => {
    const c = mount(MemberCard, { member: member(), onmessage: vi.fn() });
    expect(c.querySelector('[data-member-host]')).toBeNull();
  });

  it('idle reads in the muted tone', () => {
    const c = mount(MemberCard, { member: member({ status: 'idle', statusText: 'Idle' }), onmessage: vi.fn() });
    expect(c.querySelector('[data-member-codename]')!.className).toMatch(/text-ink-mute/);
  });
});

describe('AddMember', () => {
  const setup = (over: Partial<RelayAddMemberView> = {}): RelayAddMemberView => ({
    mode: 'setup',
    title: 'Start a run: set up your crew',
    lead: 'Relay runs one crew at a time.',
    role: 'coordinator',
    roleLocked: false,
    command: { step: 'Step 1 of 2.', text: 'register with sensei as coordinator', note: 'Paste it into an assistant session.' },
    listening: 'Listening · 0 of 2 to start a run',
    catalogue: null,
    catalogueError: null,
    launch: null,
    ...over,
  });

  it('the title, the role choice, the step and the line to paste with Copy, the listening line', async () => {
    const oncopy = vi.fn();
    const onrole = vi.fn();
    const c = mount(AddMember, { view: setup(), oncopy, onrole });
    const root = c.querySelector('[data-add-member]')!;
    expect(text(root.querySelector('[data-add-member-title]'))).toBe('Start a run: set up your crew');
    const roles = [...root.querySelectorAll('[data-role-option]')];
    expect(roles.map(text)).toEqual(['Coordinator', 'Contributor']);
    expect(roles[0].getAttribute('aria-pressed')).toBe('true');
    (roles[1] as HTMLButtonElement).click();
    expect(onrole).toHaveBeenCalledWith('contributor');
    expect(text(root.querySelector('[data-paste-line] [data-paste-text]'))).toBe('register with sensei as coordinator');
    expect(text(root.querySelector('[data-command-step]'))).toBe('Step 1 of 2.');
    (root.querySelector('[data-paste-copy]') as HTMLButtonElement).click();
    await tick();
    expect(oncopy).toHaveBeenCalledWith('register with sensei as coordinator');
    expect(text(root.querySelector('[data-listening]'))).toBe('Listening · 0 of 2 to start a run');
  });

  it('a line naming its crew shows whole: it wraps, never cut short (#419)', () => {
    const line = 'register with sensei as contributor in crew 019a5e2c-4b1d-7c00-8d2e-00000000c7f3';
    const c = mount(AddMember, {
      view: setup({ role: 'contributor', roleLocked: true, command: { ...setup().command, text: line } }),
      oncopy: vi.fn(),
      onrole: vi.fn(),
    });
    const code = c.querySelector('[data-paste-text]')!;
    expect(text(code)).toBe(line);
    expect(code.className).not.toMatch(/\btruncate\b/);
    expect(code.className).toContain("[overflow-wrap:anywhere]");
  });

  it('the coordinator taken: Contributor only, Coordinator disabled', () => {
    const c = mount(AddMember, { view: setup({ role: 'contributor', roleLocked: true }), oncopy: vi.fn(), onrole: vi.fn() });
    const [coordinator, contributor] = [...c.querySelectorAll('[data-role-option]')] as HTMLButtonElement[];
    expect(coordinator.disabled).toBe(true);
    expect(contributor.getAttribute('aria-pressed')).toBe('true');
  });

  // #455: the catalogue picker — Assistants and Editors this machine has seen.
  const picker = (picked: string | null = null): NonNullable<RelayAddMemberView['catalogue']> => ({
    picked,
    assistants: [
      { id: 'claude', name: 'Claude Code', used: 'last used 2h ago', details: ['3 sessions this week'], pressed: picked === 'claude', launch: 'cd "/w/sensei" && claude' },
    ],
    editors: [
      {
        id: 'cursor',
        name: 'Cursor',
        used: 'last used 5 min ago',
        details: ['claude-sonnet-4-5', "sensei's tools in ~/.cursor/mcp.json"],
        pressed: picked === 'cursor',
        launch: null,
      },
    ],
  });

  it('the catalogue: Assistants and Editors as pressed-state lists; a pick calls back', () => {
    const onpick = vi.fn();
    const c = mount(AddMember, { view: setup({ catalogue: picker('cursor') }), oncopy: vi.fn(), onrole: vi.fn(), onpick });
    const groups = [...c.querySelectorAll('[data-catalogue-group]')];
    expect(groups.map((g) => g.getAttribute('data-catalogue-group'))).toEqual(['assistants', 'editors']);
    const claude = c.querySelector('[data-catalogue-group="assistants"] [data-catalogue-item="claude"]') as HTMLButtonElement;
    expect(text(claude)).toBe('Claude Code last used 2h ago 3 sessions this week');
    expect(claude.getAttribute('aria-pressed')).toBe('false');
    const cursor = c.querySelector('[data-catalogue-group="editors"] [data-catalogue-item="cursor"]')!;
    expect(text(cursor)).toBe("Cursor last used 5 min ago claude-sonnet-4-5 sensei's tools in ~/.cursor/mcp.json");
    expect(cursor.getAttribute('aria-pressed')).toBe('true');
    claude.click();
    expect(onpick).toHaveBeenCalledWith('claude');
  });

  it('the note under the line is the view’s, named for the picked tool', () => {
    const note = "Paste it into Cursor's agent chat. sensei's tools are set up in ~/.cursor/mcp.json.";
    const c = mount(AddMember, {
      view: setup({ catalogue: picker('cursor'), command: { ...setup().command, note } }),
      oncopy: vi.fn(),
      onrole: vi.fn(),
    });
    expect(text(c.querySelector('[data-command-note]'))).toBe(note);
  });

  it('a failed catalogue read: an error Notice with Retry, and the paste line stays usable', () => {
    const onretrycatalogue = vi.fn();
    const c = mount(AddMember, {
      view: setup({ catalogueError: 'internal error' }),
      oncopy: vi.fn(),
      onrole: vi.fn(),
      onretrycatalogue,
    });
    expect(c.querySelector('[data-catalogue-group]')).toBeNull();
    const notice = c.querySelector('[data-catalogue-error]')!;
    expect(text(notice)).toContain("Couldn't read the assistants");
    const retry = [...notice.querySelectorAll('button')].find((b) => text(b) === 'Retry') as HTMLButtonElement;
    retry.click();
    expect(onretrycatalogue).toHaveBeenCalled();
    expect(text(c.querySelector('[data-paste-text]'))).toBe('register with sensei as coordinator');
  });

  it('no catalogue on this surface: no picker and no error', () => {
    const c = mount(AddMember, { view: setup(), oncopy: vi.fn(), onrole: vi.fn() });
    expect(c.querySelector('[data-catalogue-group]')).toBeNull();
    expect(c.querySelector('[data-catalogue-error]')).toBeNull();
  });

  it('the picked tool’s launch: its own block under the paste line, with Copy (#513)', async () => {
    const oncopy = vi.fn();
    const c = mount(AddMember, {
      view: setup({ catalogue: picker('claude'), launch: 'cd "/w/sensei" && claude' }),
      oncopy,
      onrole: vi.fn(),
    });
    expect(text(c.querySelector('[data-launch-text]'))).toBe('cd "/w/sensei" && claude');
    (c.querySelector('[data-launch-copy]') as HTMLButtonElement).click();
    await tick();
    expect(oncopy).toHaveBeenCalledWith('cd "/w/sensei" && claude');
    expect(text(c.querySelector('[data-paste-text]'))).toBe('register with sensei as coordinator');
  });

  it('nothing picked, or nothing resolved: no launch block (#513)', () => {
    const c = mount(AddMember, {
      view: setup({ catalogue: picker('cursor'), launch: null }),
      oncopy: vi.fn(),
      onrole: vi.fn(),
    });
    expect(c.querySelector('[data-launch-line]')).toBeNull();
  });
});
