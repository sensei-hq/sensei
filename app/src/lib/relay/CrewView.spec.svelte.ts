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
    expect(root.querySelector('[data-run-title] [data-relay-pill]')?.getAttribute('data-tone')).toBe('success');
    expect(text(root.querySelector('[data-run-meta]'))).toContain('completes, and is archived');
    expect([...root.querySelectorAll('[data-run-fact]')].map(text)).toEqual(['Members 7', 'Tasks 12', 'Done 5']);
    expect([...root.querySelectorAll('[data-member-group]')].map((g) => text(g.querySelector('[data-member-group-label]')))).toEqual([
      'Coordinator',
      'Contributors · 1',
    ]);
    expect(text(root)).not.toMatch(/\bEnd\b/);
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
    expect([...root.querySelectorAll('[data-tag]')].map(text)).toEqual(['Claude Code 2.1', 'claude-opus-5-5']);
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

  it('the coordinator taken: Contributor only, Coordinator disabled', () => {
    const c = mount(AddMember, { view: setup({ role: 'contributor', roleLocked: true }), oncopy: vi.fn(), onrole: vi.fn() });
    const [coordinator, contributor] = [...c.querySelectorAll('[data-role-option]')] as HTMLButtonElement[];
    expect(coordinator.disabled).toBe(true);
    expect(contributor.getAttribute('aria-pressed')).toBe('true');
  });
});
