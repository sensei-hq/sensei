<script lang="ts">
  // packages/ui/Notice.svelte — one component for every error, refusal and wait,
  // in sensei and the dōjō. The bootstrap's "needs your hand" screen made
  // reusable, built from rokkit's Card and Button on the design tokens: what it
  // says and how it looks are decided in ./notice.js; this file only lays the
  // parts out. See docs/architecture/notice.md.
  //
  //   screen  a whole screen that could not do its job (ScreenState's error):
  //           eyebrow, display headline, sentence, then the card
  //   panel   the card alone, inside a section (the bootstrap's Remedy card)
  //   inline  one line in a row, a list or a form; "Why?" opens the rest
  //
  // The mark follows the glyph mode (#133): kanji mode shows the tone's kanji,
  // icon mode its vocabulary icon. Copy comes from the caller's state module or
  // from `noticeFromReason` / `noticeFromApiError` — never typed into a screen.
  import type { Snippet } from 'svelte';
  import { Button, Card } from '@rokkit/ui';
  import Eyebrow from './Eyebrow.svelte';
  import { useGlyphMode } from './glyph-mode.js';
  import { noticeTone, noticeMark } from './notice.js';

  type Tone = 'fault' | 'refusal' | 'action' | 'normal';
  type Size = 'screen' | 'panel' | 'inline';
  type GlyphMode = 'kanji' | 'icon';
  interface NoticeAction {
    label: string;
    onclick?: () => void;
    /** A link instead of a button — "Open Health", "See plans". */
    href?: string;
    /** A page outside the app: opens in the browser, never the webview. */
    external?: boolean;
    /** In flight: the button is disabled so it cannot be pressed twice. */
    busy?: boolean;
    /** The `data-action` hook (default `primary` / `secondary`) — e.g. `retry`. */
    name?: string;
  }

  let {
    tone = 'fault',
    size = 'panel',
    title,
    emphasis,
    eyebrow,
    detail,
    remedy,
    remedyTitle = 'What to do',
    actor,
    code,
    script,
    primary,
    secondary,
    mark,
    icon,
    mode: modeProp,
    expanded = false,
    bare = false,
    writeText,
    children,
    id,
    tabindex,
    ...rest
  }: {
    tone?: Tone;
    size?: Size;
    /** What happened, in one line. The headline at screen size. */
    title: string;
    /** Screen size: the headline's last words, in the tone colour ("One last *step.*"). */
    emphasis?: string;
    /** Screen size: the situation in two or three words ("couldn't load"). */
    eyebrow?: string;
    /** Why — and whether anything was saved or lost. */
    detail?: string;
    /** The one thing to do next. */
    remedy?: string;
    /** The card's heading at screen size; the remedy's label otherwise. */
    remedyTitle?: string;
    /** Who can fix it, when that is not the reader. */
    actor?: string;
    /** The reason or error code, for logs and support — never the headline. */
    code?: string;
    /** A command to run, shown with a Copy button (the bootstrap's Remedy). */
    script?: string;
    primary?: NoticeAction;
    secondary?: NoticeAction;
    /** A kanji other than the tone's own. */
    mark?: string;
    /** The icon-mode mark, when not the vocabulary's icon for the kanji. */
    icon?: string;
    /** Overrides the provided glyph mode. */
    mode?: GlyphMode;
    /** Inline size: open on first render. */
    expanded?: boolean;
    /** Inline size: no box — for a line already inside a bordered row. */
    bare?: boolean;
    /** Clipboard override for tests; defaults to `navigator.clipboard.writeText`. */
    writeText?: (text: string) => Promise<void>;
    /** Anything else the card body needs (a list of affected rows, say). */
    children?: Snippet;
    /** For `aria-describedby` from the field a failure concerns. */
    id?: string;
    /** `-1` makes a notice focusable, so a failure can take focus. */
    tabindex?: number;
    [attr: `data-${string}`]: unknown;
  } = $props();

  const provided = useGlyphMode();
  const mode = $derived(modeProp ?? provided());
  const view = $derived(noticeTone(tone));
  const glyph = $derived(noticeMark(tone, mode, { char: mark, icon }));

  // Inline: the reader's toggle wins once they have used it.
  let toggled = $state<boolean | null>(null);
  const open = $derived(toggled ?? expanded);
  const hasMore = $derived(Boolean(detail || remedy || actor || code || script || secondary || children));
  const hasBody = $derived(Boolean((size === 'panel' && detail) || remedy || actor || script || children));
  const hasCard = $derived(Boolean(remedy || actor || code || script || primary || secondary || children));
  const hasFooter = $derived(Boolean(primary || secondary));

  type CopyState = 'idle' | 'copied' | 'failed';
  let copyState = $state<CopyState>('idle');
  const copyLabel = $derived(
    copyState === 'copied' ? 'Copied' : copyState === 'failed' ? 'Copy failed — select it below' : 'Copy script',
  );

  async function copyScript() {
    const write =
      writeText ??
      (typeof navigator !== 'undefined' && navigator.clipboard
        ? navigator.clipboard.writeText.bind(navigator.clipboard)
        : null);
    try {
      if (!write || !script) throw new Error('no clipboard');
      await write(script);
      copyState = 'copied';
    } catch {
      copyState = 'failed';
    }
    setTimeout(() => (copyState = 'idle'), 2400);
  }
</script>

{#snippet markEl(extra: string)}
  {#if glyph.mode === 'icon'}
    <span data-part="mark" data-glyph="icon" class={[glyph.icon, view.text, extra, 'shrink-0']} aria-hidden="true"></span>
  {:else}
    <span data-part="mark" data-glyph="kanji" class={['font-kanji leading-none', view.text, extra, 'shrink-0']} aria-hidden="true">{glyph.char}</span>
  {/if}
{/snippet}

<!-- primary: solid · secondary: outline · on an inline line: link. -->
{#snippet action(a: NoticeAction, which: 'primary' | 'secondary', look: 'solid' | 'outline' | 'link')}
  <Button
    label={a.label}
    size="sm"
    variant={look === 'solid' ? 'primary' : 'default'}
    style={look === 'solid' ? 'default' : look}
    href={a.href}
    target={a.href && a.external ? '_blank' : undefined}
    {...(a.href && a.external ? { rel: 'noopener noreferrer' } : {})}
    disabled={a.busy}
    onclick={a.onclick}
    data-action={a.name ?? which}
  />
{/snippet}

{#snippet scriptBlock()}
  <div data-part="script" class="flex flex-col gap-2">
    <pre class="m-0 px-3 py-3 rounded-md font-mono text-xs text-ink bg-paper-mute leading-normal whitespace-pre-wrap break-words max-h-56 overflow-auto select-text cursor-text">{script}</pre>
    <div>
      <Button label={copyLabel} size="sm" style="outline" onclick={copyScript} data-action="copy" data-state={copyState} />
    </div>
  </div>
{/snippet}

{#snippet codeEl()}
  {#if code}<span data-part="code" class="font-mono text-xs text-ink-soft whitespace-nowrap">{code}</span>{/if}
{/snippet}

{#snippet card(heading: string, level: 'h2' | 'h3')}
  <div data-part="card">
    <Card>
      {#snippet header()}
        <div class="flex items-center gap-2.5">
          {@render markEl('text-base')}
          <svelte:element this={level} data-part="heading" class="m-0 flex-1 min-w-0 text-sm font-medium text-ink">{heading}</svelte:element>
          {@render codeEl()}
        </div>
      {/snippet}
      {#if hasBody}
        <div data-part="body" class="flex flex-col gap-1.5 text-sm">
          {#if size === 'panel' && detail}<p data-part="detail" class="m-0 text-ink">{detail}</p>{/if}
          {#if remedy}
            <p data-part="remedy" class="m-0 text-ink-mute">
              {#if size === 'panel'}<strong class="font-medium text-ink">{remedyTitle}:</strong>{' '}{/if}{remedy}
            </p>
          {/if}
          {#if actor}<p data-part="actor" class="m-0 text-xs text-ink-soft">{actor}</p>{/if}
          {@render children?.()}
          {#if script}{@render scriptBlock()}{/if}
        </div>
      {/if}
      {#snippet footer()}
        {#if hasFooter}
          <div data-part="actions" class="flex flex-wrap items-center justify-between gap-2.5">
            {#if primary}{@render action(primary, 'primary', 'solid')}{/if}
            {#if secondary}{@render action(secondary, 'secondary', 'outline')}{/if}
          </div>
        {/if}
      {/snippet}
    </Card>
  </div>
{/snippet}

{#if size === 'screen'}
  <!-- `tabindex` is only ever -1: a failure that takes focus (Integrations). -->
  <!-- svelte-ignore a11y_no_noninteractive_tabindex -->
  <div {...rest} {id} {tabindex} data-component="notice" data-size="screen" data-tone={tone} role={view.role}
       class="w-full max-w-[560px] mx-auto flex flex-col gap-4 py-16 px-6">
    {#if eyebrow}<div data-part="eyebrow"><Eyebrow>{eyebrow}</Eyebrow></div>{/if}
    <h2 data-part="title" class="m-0 font-display text-3xl font-light leading-tight tracking-tight text-ink">
      {title}{#if emphasis}{' '}<span data-part="emphasis" class={view.text}>{emphasis}</span>{/if}
    </h2>
    {#if detail}<p data-part="detail" class="m-0 text-sm text-ink-mute leading-relaxed">{detail}</p>{/if}
    {#if hasCard}{@render card(remedyTitle, 'h3')}{/if}
  </div>
{:else if size === 'panel'}
  <!-- `tabindex` is only ever -1: a failure that takes focus (Integrations). -->
  <!-- svelte-ignore a11y_no_noninteractive_tabindex -->
  <div {...rest} {id} {tabindex} data-component="notice" data-size="panel" data-tone={tone} role={view.role}>
    {@render card(title, 'h3')}
  </div>
{:else}
  <!-- `tabindex` is only ever -1: a failure that takes focus (Integrations). -->
  <!-- svelte-ignore a11y_no_noninteractive_tabindex -->
  <div {...rest} {id} {tabindex} data-component="notice" data-size="inline" data-tone={tone} role={view.role}
       class={['text-sm', !bare && 'rounded-md border border-paper-edge bg-paper']}>
    <div class={['flex items-center gap-2', bare ? 'py-1' : 'px-3 py-2']}>
      {@render markEl('text-sm')}
      <span data-part="title" class="min-w-0 text-ink">{title}</span>
      <span class="ml-auto flex items-center gap-3 shrink-0">
        {#if primary}{@render action(primary, 'primary', 'link')}{/if}
        {#if hasMore}
          <Button label={open ? 'Hide' : 'Why?'} size="sm" style="link" aria-expanded={open}
                  data-action="toggle" onclick={() => (toggled = !open)} />
        {/if}
      </span>
    </div>
    {#if open && hasMore}
      <div data-part="more" class={['flex flex-col gap-1.5 text-sm', bare ? 'pt-1 pl-6' : 'px-3 py-2 pl-9 border-t border-paper-edge bg-paper-soft']}>
        {#if detail}<p data-part="detail" class="m-0 text-ink">{detail}</p>{/if}
        {#if remedy}<p data-part="remedy" class="m-0 text-ink-mute"><strong class="font-medium text-ink">{remedyTitle}:</strong>{' '}{remedy}</p>{/if}
        {#if actor}<p data-part="actor" class="m-0 text-xs text-ink-soft">{actor}</p>{/if}
        {@render children?.()}
        {#if script}{@render scriptBlock()}{/if}
        {#if secondary || code}
          <div class="flex items-center gap-3">
            {#if secondary}{@render action(secondary, 'secondary', 'link')}{/if}
            {@render codeEl()}
          </div>
        {/if}
      </div>
    {/if}
  </div>
{/if}
