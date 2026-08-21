<script lang="ts">
  type Size = 'xs' | 'sm' | 'base' | 'lg' | 'xl' | '2xl' | '3xl' | '4xl' | 'screen';
  type Tone = 'accent' | 'muted' | 'watermark' | 'success' | 'warning';

  let {
    char,
    size = 'base',
    tone = 'accent',
  }: { char: string; size?: Size; tone?: Tone } = $props();

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
      // KanjiHeader (a fixed display glyph, not body type). See mockup-drift-audit F5.
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

<span
  data-component="kanji"
  class="kanji leading-none {sizeClass} {toneClass}"
>{char}</span>
