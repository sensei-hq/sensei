<script lang="ts">
  type Size = 'xs' | 'sm' | 'base' | 'lg' | 'xl' | '2xl' | '3xl' | '4xl' | 'screen';
  type Tone = 'accent' | 'muted' | 'watermark' | 'success' | 'warning';

  import { useGlyphMode, type GlyphMode } from '$lib/glyphs';

  let {
    char,
    size = 'base',
    tone = 'accent',
    icon,
    mode: modeProp,
  }: { char: string; size?: Size; tone?: Tone; icon?: string; mode?: GlyphMode } = $props();

  // The provided mode (#133) unless the caller says otherwise.
  const provided = useGlyphMode();
  const mode = $derived(modeProp ?? provided());

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
    })[size]
  );

  const toneClass = $derived(
    ({
      'accent':    'text-accent',
      'muted':     'text-ink-mute',
      'success':   'text-success',
      'warning':   'text-warning',
      'watermark': 'text-accent opacity-55',
    })[tone]
  );
</script>

{#if mode === 'icon' && icon}
  <!-- #133: the icon that stands for this kanji, at the same size and tone. -->
  <span data-component="glyph-icon" class="{icon} {sizeClass} {toneClass}" aria-hidden="true"></span>
{:else}
  <span
    data-component="kanji"
    class="kanji leading-none {sizeClass} {toneClass}"
  >{char}</span>
{/if}
