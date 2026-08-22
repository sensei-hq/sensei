<script lang="ts">
  import type { Snippet } from 'svelte';
  import Eyebrow from './Eyebrow.svelte';
  import Kanji from './Kanji.svelte';

  /** One scale step, not three unrelated heading levels. */
  type Size = 'sm' | 'md' | 'lg';

  // The app's one screen/section header: glyph · eyebrow · title · description,
  // with an optional count and a right slot. Replaces the `variant: h1|h2|h3`
  // heading-level prop with a `size` scale that moves the title, the glyph AND the
  // description together, and absorbs the old KanjiHeader — whose only real
  // differences were a snippet title and no chrome, now covered by `title`
  // accepting a snippet and by `bordered`/`padded`.
  //
  // Deliberately app-local rather than a rokkit component: it encodes this
  // product's header composition, not a generic primitive.
  let {
    title,
    eyebrow,
    kanji,
    icon,
    description,
    clampDescription = false,
    count,
    size = 'md',
    bordered = true,
    padded = true,
    right,
  }: {
    /** Plain string for the common case; a snippet when the title needs markup. */
    title: string | Snippet;
    eyebrow?: string;
    /** A kanji glyph. Mutually exclusive with `icon` — kanji wins if both are set. */
    kanji?: string;
    /** A UnoCSS icon class (e.g. `i-solar-folder-linear`), for headers with no glyph. */
    icon?: string;
    description?: string;
    /**
     * Clamp the description to two lines with an ellipsis. Off by default — a
     * screen header's own copy is short and should wrap in full. Turn it on where
     * the text is caller-supplied and can run long, so one verbose row can't push
     * the content below the fold.
     */
    clampDescription?: boolean;
    /** A tally alongside the title (e.g. how many rows the section lists). */
    count?: number | string;
    size?: Size;
    bordered?: boolean;
    /** Off for a header nested inside a container that already pads. */
    padded?: boolean;
    right?: Snippet;
  } = $props();

  // `lg` and `md` both keep the mockup's 40px signature glyph on purpose — that is
  // the screen-header mark, not a function of heading level (mockup-drift-audit
  // F5). `sm` is the nested/section step and drops to a compact glyph.
  const SIZES = {
    lg: { title: 'text-2xl', kanji: 'screen', icon: 'text-2xl', desc: 'text-sm', gap: 'gap-5', pad: 'pt-5 pb-4 px-6' },
    md: { title: 'text-xl',  kanji: 'screen', icon: 'text-2xl', desc: 'text-sm', gap: 'gap-5', pad: 'pt-5 pb-4 px-6' },
    sm: { title: 'text-lg',  kanji: 'xl',     icon: 'text-lg',  desc: 'text-xs', gap: 'gap-3', pad: 'pt-3 pb-2 px-4' },
  } as const;
  const s = $derived(SIZES[size]);
</script>

<header
  data-component="page-header"
  data-size={size}
  class="flex items-center {s.gap} {padded ? s.pad : ''} {bordered
    ? 'border-b border-paper-edge'
    : ''}"
>
  {#if kanji}
    <Kanji char={kanji} size={s.kanji} />
  {:else if icon}
    <span
      data-component="page-header-icon"
      class="{icon} {s.icon} text-accent shrink-0"
      aria-hidden="true"
    ></span>
  {/if}

  <div class="flex-1 min-w-0">
    {#if eyebrow}
      <div class="mb-1"><Eyebrow>{eyebrow}</Eyebrow></div>
    {/if}
    <div class="flex items-baseline gap-2 min-w-0">
      <h1 class="display {s.title} font-normal m-0 tracking-tight text-ink">
        {#if typeof title === 'string'}{title}{:else}{@render title()}{/if}
      </h1>
      {#if count != null}
        <span data-component="page-header-count" class="mono text-xs text-ink-faint shrink-0"
          >{count}</span
        >
      {/if}
    </div>
    {#if description}
      <p
        data-component="page-header-description"
        class="{s.desc} text-ink-soft leading-normal m-0 mt-1 max-w-[720px] {clampDescription
          ? 'line-clamp-2'
          : ''}"
      >{description}</p>
    {/if}
  </div>

  {#if right}
    <div class="ml-auto shrink-0">{@render right()}</div>
  {/if}
</header>
