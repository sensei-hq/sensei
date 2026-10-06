<!--
  A pressed/unpressed chip — the level, grain and kind controls every diagram
  and filter bar is built out of.

  ## Why this is a component rather than five copies

  The shape appeared in five screens, and all five rendered their PRESSED chip
  invisible: `background-color` computed to `rgba(0, 0, 0, 0)` behind
  `oklch(0.975 0.008 85)` text, near-white on paper. Measured in the browser on
  the shipped Structure screen, where "file" and "calls" were unreadable while
  selected.

  TWO CAUSES STACKED, and only one of them is obvious.

  The first was ours: a static `bg-transparent` beside a conditional
  `bg-primary`. Both rules were emitted and `bg-transparent` won by source
  order. The two backgrounds are now mutually exclusive conditionals, so exactly
  one ever applies.

  The second is why that alone did not fix it. The preflight reset is
  `button, [type="button"], [type="reset"], [type="submit"] { background-color:
  transparent }`, and `[type="button"]` is an ATTRIBUTE selector — specificity
  (0,1,0), the same as `.bg-primary`. The tie goes to source order, and the
  reset is later. So a bare `bg-primary` can never paint a
  `<button type="button">` anywhere in this app; the sub-nav's `<a>` works only
  because no reset claims it.

  `bg-primary!` is the escape, and it is the narrow one. It keeps the named
  token (no hex, no `<style>` block, no global override) and raises exactly this
  declaration above a reset it ties with. The reset is the real defect and
  belongs in its own change — it silently disables every background utility on
  every typed button in the app, which is a far wider question than one chip.

  ## `aria-pressed`, not `aria-current`

  This is a toggle, not navigation. `aria-current` belongs on the sub-nav links,
  which go somewhere; a chip changes what the current screen shows.
-->
<script lang="ts">
    interface Props {
        /** The label, and what a screen reader reads. */
        label: string;
        pressed: boolean;
        onpress: () => void;
        /** `sm` for a primary axis (level, grain), `xs` for a secondary one
         *  (edge kinds) — the two sizes the diagram bars already use. */
        size?: 'sm' | 'xs';
        testid?: string;
    }

    let { label, pressed, onpress, size = 'sm', testid }: Props = $props();
</script>

<button
    type="button"
    class="rounded border border-paper-edge cursor-pointer"
    class:px-3={size === 'sm'}
    class:py-1={true}
    class:text-sm={size === 'sm'}
    class:px-2={size === 'xs'}
    class:text-xs={size === 'xs'}
    class:bg-primary!={pressed}
    class:text-on-primary={pressed}
    class:bg-transparent={!pressed}
    class:text-ink={!pressed}
    aria-pressed={pressed}
    data-testid={testid}
    onclick={onpress}
>
    {label}
</button>
