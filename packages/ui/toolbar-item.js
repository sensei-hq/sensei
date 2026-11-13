// packages/ui/toolbar-item.js — the class vocabulary of one toolbar item
// (COMPONENTS.md §2): a 36×36 icon button, radius 8, 1px edge. Pressed is the
// mute surface on the strong edge; a pressed view tints its icon accent, while
// a pressed toggle keeps the ink icon (a toggle is a switch, not a place).
//
// Every class string is literal on purpose: UnoCSS scans source text, so a
// class assembled from a template never reaches the generated CSS.

/** @typedef {'view' | 'toggle'} ToolbarKind */

/**
 * The button's own shape, idle or pressed.
 * @param {boolean} pressed
 * @returns {string}
 */
export function toolbarItemClass(pressed) {
  return `relative inline-flex items-center justify-center w-9 h-9 rounded-[8px] border border-solid cursor-pointer ${
    pressed ? 'bg-paper-mute border-paper-edge-strong' : 'bg-paper border-paper-edge'
  }`;
}

/**
 * The icon's size and colour: 18px, accent only when a view is the pressed one.
 * @param {ToolbarKind} kind
 * @param {boolean} pressed
 * @returns {string}
 */
export function toolbarIconClass(kind, pressed) {
  return `shrink-0 text-[18px] ${kind === 'view' && pressed ? 'text-accent' : 'text-ink'}`;
}

/**
 * The count badge: 16px tall, min 16 wide, 0/4 padding, ink on paper, 10px
 * tabular, hung at the top-right corner.
 * @returns {string}
 */
export function toolbarBadgeClass() {
  return 'absolute -top-1.5 -right-1.5 min-w-4 h-4 px-1 rounded-full bg-ink text-paper text-[10px] leading-4 text-center tabular-nums';
}

/**
 * The same badge drawn by the button's `::after` from its `data-count`, for a
 * toolbar that renders the button itself (rokkit's shell toolbar: the item's
 * attrs land on the button, nothing else can). Literal, like the class above,
 * so UnoCSS finds every `after:` utility in the source.
 * @returns {string}
 */
export function toolbarBadgeAfterClass() {
  return 'after:content-[attr(data-count)] after:absolute after:-top-1.5 after:-right-1.5 after:min-w-4 after:h-4 after:px-1 after:rounded-full after:bg-ink after:text-paper after:text-[10px] after:leading-4 after:text-center after:tabular-nums after:font-sans';
}

/**
 * The accessible name and title: "Queue · 7" when it carries a count.
 * @param {string} label
 * @param {number} [count]
 * @returns {string}
 */
export function toolbarLabel(label, count = 0) {
  return count > 0 ? `${label} · ${count}` : label;
}
