// @vitest-environment jsdom
import { describe, it, expect, afterEach, vi } from 'vitest';
import type { ComponentProps } from 'svelte';
import { mountComponent } from '$lib/test-mount.js';
import { ToolbarItem, toolbarBadgeClass, toolbarIconClass, toolbarItemClass, toolbarLabel } from '@sensei/ui';

// ToolbarItem (COMPONENTS.md §2): the 36×36 icon button a screen's toolbar is
// made of — a view tab, or a toggle. Fixtures: idle, pressed, pressed with a
// badge, a two-digit badge, and a toggle. The component takes its icon as a
// class (the caller owns the icon style); the fixture uses the app's Solar
// Bold Duotone vocabulary, which is the only icon style the app ships.
// Theme is not a snapshot dimension (tokens flip, the DOM does not) and the
// button never reflows, so 36×36 is asserted instead of a layout prop.

let cleanup: Array<() => void> = [];
afterEach(() => {
  cleanup.forEach((fn) => fn());
  cleanup = [];
});

type ToolbarItemProps = ComponentProps<typeof ToolbarItem>;

function mount(props: ToolbarItemProps) {
  const m = mountComponent(ToolbarItem, props);
  cleanup.push(m.destroy);
  const root = m.container.querySelector('[data-toolbar-item]') as HTMLButtonElement;
  return { ...m, root };
}

const NOW: ToolbarItemProps = { id: 'now', label: 'Now', icon: 'i-solar-widget-4-bold-duotone', kind: 'view', pressed: false };
const QUEUE: ToolbarItemProps = { id: 'queue', label: 'Queue', icon: 'i-solar-clipboard-list-bold-duotone', kind: 'view', pressed: true, count: 7 };
const MESSAGE: ToolbarItemProps = { id: 'compose', label: 'Message the crew', icon: 'i-solar-chat-round-line-bold-duotone', kind: 'toggle', pressed: false };

/** Every token the toolbar vocabulary may reference (rokkit's zen-sumi
 *  preflight plus the app's own overrides). A name that is not emitted falls
 *  back to nothing, which is why it is pinned here. */
const KNOWN_TOKENS = new Set([
  'paper',
  'paper-mute',
  'paper-edge',
  'paper-edge-strong',
  'ink',
  'accent',
]);

describe('the toolbar item class vocabulary', () => {
  it('names only tokens the app emits, and no literal colour', () => {
    const all = [
      toolbarItemClass(false),
      toolbarItemClass(true),
      toolbarIconClass('view', false),
      toolbarIconClass('view', true),
      toolbarIconClass('toggle', true),
      toolbarBadgeClass(),
    ].join(' ');
    for (const [, name] of all.matchAll(/var\(--([a-z-]+)\)/g)) expect(KNOWN_TOKENS).toContain(name);
    expect(all).not.toMatch(/#[0-9a-f]{3,8}\b/i);
    expect(all).not.toMatch(/\b(rgb|hsl|oklch)\(/);
  });

  it('is 36×36, radius 8, 1px edge; pressed is the mute surface on the strong edge', () => {
    const idle = toolbarItemClass(false);
    expect(idle).toContain('w-9');
    expect(idle).toContain('h-9');
    expect(idle).toContain('rounded-[8px]');
    expect(idle).toContain('bg-paper');
    expect(idle).toContain('border-paper-edge');

    const pressed = toolbarItemClass(true);
    expect(pressed).toContain('bg-paper-mute');
    expect(pressed).toContain('border-paper-edge-strong');
  });

  it('a pressed view tints its icon accent; a pressed toggle keeps the ink icon', () => {
    expect(toolbarIconClass('view', false)).toContain('text-ink');
    expect(toolbarIconClass('view', true)).toContain('text-accent');
    expect(toolbarIconClass('toggle', true)).toContain('text-ink');
    expect(toolbarIconClass('toggle', true)).not.toContain('text-accent');
    for (const kind of ['view', 'toggle'] as const) {
      expect(toolbarIconClass(kind, false)).toContain('text-[18px]');
    }
  });

  it('the badge is 16px tall, min 16 wide, 0/4 padding, ink on paper, 10px tabular', () => {
    const cls = toolbarBadgeClass();
    expect(cls).toContain('h-4');
    expect(cls).toContain('min-w-4');
    expect(cls).toContain('px-1');
    expect(cls).toContain('rounded-full');
    expect(cls).toContain('bg-ink');
    expect(cls).toContain('text-paper');
    expect(cls).toContain('text-[10px]');
    expect(cls).toContain('tabular-nums');
    expect(cls).toContain('-top-1.5');
    expect(cls).toContain('-right-1.5');
  });

  it('labels the button with its count, as the mockup does', () => {
    expect(toolbarLabel('Queue')).toBe('Queue');
    expect(toolbarLabel('Queue', 0)).toBe('Queue');
    expect(toolbarLabel('Queue', 7)).toBe('Queue · 7');
  });
});

describe('ToolbarItem', () => {
  it('is a 36×36 icon button with the idle surface', () => {
    const { root } = mount(NOW);
    expect(root.tagName).toBe('BUTTON');
    expect(root.type).toBe('button');
    expect(root.dataset.toolbarItem).toBe('now');
    expect(root.dataset.kind).toBe('view');
    expect(root.dataset.pressed).toBe('false');
    expect(root.className).toBe(toolbarItemClass(false));
    expect(root.className).toContain('w-9');
    expect(root.className).toContain('h-9');
  });

  it('names itself "label · count" in both title and aria-label', () => {
    expect(mount(NOW).root.getAttribute('aria-label')).toBe('Now');
    expect(mount(NOW).root.getAttribute('title')).toBe('Now');
    const { root } = mount(QUEUE);
    expect(root.getAttribute('aria-label')).toBe(toolbarLabel('Queue', 7));
    expect(root.getAttribute('title')).toBe('Queue · 7');
  });

  it('is a tab when it is a view, and says whether it is the pressed one', () => {
    const idle = mount(NOW).root;
    expect(idle.getAttribute('role')).toBe('tab');
    expect(idle.getAttribute('aria-selected')).toBe('false');
    expect(idle.hasAttribute('aria-pressed')).toBe(false);

    const pressed = mount(QUEUE).root;
    expect(pressed.getAttribute('aria-selected')).toBe('true');
    expect(pressed.dataset.pressed).toBe('true');
    expect(pressed.className).toBe(toolbarItemClass(true));
  });

  it('is a toggle when it is not a view, and presses with aria-pressed', () => {
    const idle = mount(MESSAGE).root;
    expect(idle.hasAttribute('role')).toBe(false);
    expect(idle.getAttribute('aria-pressed')).toBe('false');
    expect(idle.hasAttribute('aria-selected')).toBe(false);

    const pressed = mount({ ...MESSAGE, pressed: true }).root;
    expect(pressed.getAttribute('aria-pressed')).toBe('true');
  });

  it('tints a pressed view icon accent, and leaves a pressed toggle ink', () => {
    const view = mount(QUEUE).root;
    const viewIcon = view.querySelector('[aria-hidden="true"]') as HTMLElement;
    expect(viewIcon.className).toContain('i-solar-clipboard-list-bold-duotone');
    expect(viewIcon.className).toContain(toolbarIconClass('view', true));

    const toggle = mount({ ...MESSAGE, pressed: true }).root;
    const toggleIcon = toggle.querySelector('[aria-hidden="true"]') as HTMLElement;
    expect(toggleIcon.className).toContain(toolbarIconClass('toggle', true));
    expect(toggleIcon.className).toContain('text-ink');
  });

  it('draws the count as a badge only when there is one', () => {
    expect(mount(NOW).root.querySelector('[data-toolbar-count]')).toBeNull();
    expect(mount({ ...QUEUE, count: 0 }).root.querySelector('[data-toolbar-count]')).toBeNull();

    const badge = mount(QUEUE).root.querySelector('[data-toolbar-count]') as HTMLElement;
    expect(badge.textContent).toBe('7');
    expect(badge.className).toBe(toolbarBadgeClass());

    const two = mount({ ...QUEUE, count: 12 }).root.querySelector('[data-toolbar-count]') as HTMLElement;
    expect(two.textContent).toBe('12');
  });

  it('forwards the click, not a state of its own', () => {
    const onclick = vi.fn();
    const { root } = mount({ ...QUEUE, onclick });
    root.click();
    expect(onclick).toHaveBeenCalledTimes(1);
  });

  it('renders the same markup in dark mode — the tokens flip, not the DOM', () => {
    const light = mount(QUEUE).root.outerHTML;
    document.documentElement.dataset.mode = 'dark';
    try {
      expect(mount(QUEUE).root.outerHTML).toBe(light);
    } finally {
      delete document.documentElement.dataset.mode;
    }
  });

  it('pins every fixture', () => {
    const all = [NOW, QUEUE, { ...QUEUE, count: 12 }, MESSAGE, { ...MESSAGE, pressed: true }]
      .map((props) => mount(props).root.outerHTML)
      .join('\n');
    expect(all).toMatchSnapshot();
  });
});
