<script lang="ts">
  type Size = 'xs' | 'sm' | 'base' | 'lg' | 'xl' | '2xl' | '3xl' | '4xl';
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
    })[size]
  );

  const toneClass = $derived(
    ({
      'accent':    'text-primary-z6',
      'muted':     'text-ink-z6',
      'success':   'text-success-z6',
      'warning':   'text-warning-z6',
      'watermark': 'text-primary-z6 opacity-55',
    })[tone]
  );
</script>

<span
  data-component="kanji"
  class="kanji leading-none {sizeClass} {toneClass}"
>{char}</span>
