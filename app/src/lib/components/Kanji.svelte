<script lang="ts">
  // `inherit`: add no class of its own — for a mark whose size or colour the
  // caller states in `class` (one converted from a bare span, #133).
  type Size = 'xs' | 'sm' | 'base' | 'lg' | 'xl' | '2xl' | '3xl' | '4xl' | 'screen' | 'inherit';
  type Tone = 'accent' | 'muted' | 'watermark' | 'success' | 'warning' | 'inherit';

  import { useGlyphMode, type GlyphMode } from '$lib/glyphs';
  import { iconForKanji } from '$lib/glyph-vocabulary';

  let {
    char,
    size = 'base',
    tone = 'accent',
    icon,
    mode: modeProp,
    class: extra = '',
  }: {
    char: string;
    size?: Size;
    tone?: Tone;
    /** Icon-mode mark. Omitted (or null): the vocabulary's icon for `char`.
     *  Every mark follows the mode (#133). */
    icon?: string | null;
    mode?: GlyphMode;
    /** Classes the mark carries in both modes. */
    class?: string;
  } = $props();

  // The provided mode (#133) unless the caller says otherwise.
  const provided = useGlyphMode();
  const mode = $derived(modeProp ?? provided());
  const resolved = $derived(icon ?? iconForKanji(char));

  const sizeClass = $derived(
    ({
      'xs':  'text-xs',
      'sm':  'text-sm',
      'base':'text-base',
      'lg':  'text-lg',
      'xl':  'text-xl',
      '2xl': 'text-2xl',
      '3xl': 'text-3xl',
      '4xl': 'text-4xl',
      // Signature screen-header glyph — 40px, matching the mockup ScreenHeader/
      // PageHeader (a fixed display glyph, not body type). See mockup-drift-audit F5.
      'screen': 'text-3xl',
      // Leading too: an inherited size is the caller's whole typographic box.
      'inherit': '',
    })[size]
  );

  const toneClass = $derived(
    ({
      'accent':    'text-accent',
      'muted':     'text-ink-soft',
      'success':   'text-success',
      'warning':   'text-warning',
      'watermark': 'text-accent opacity-55',
      'inherit':   '',
    })[tone]
  );
</script>

{#if mode === 'icon' && resolved}
  <!-- #133: the icon that stands for this kanji, at the same size and tone. -->
  <span data-component="glyph-icon" class={[resolved, sizeClass, toneClass, extra]} aria-hidden="true"></span>
{:else}
  <span
    data-component="kanji"
    class={['kanji', size !== 'inherit' && 'leading-none', sizeClass, toneClass, extra]}
  >{char}</span>
{/if}
