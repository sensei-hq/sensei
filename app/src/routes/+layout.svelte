<script lang="ts">
  import 'uno.css';
  import '../app.css';
  import { onMount } from 'svelte';
  import { goto } from '$app/navigation';
  import { appState } from '$lib/appstate.svelte.js';

  let { children } = $props();

  /**
   * Read the OS accent color (CSS system keyword `AccentColor`) by briefly
   * measuring its computed value on a hidden element, then derive a full
   * Rokkit-compatible z1–z9 scale (comma-separated RGB triplets) and write
   * them as inline CSS custom properties on the root element.
   *
   * Rokkit stores colors as "R,G,B" triplets consumed via rgb(var(--color-*)).
   * Inline styles on :root override Rokkit's stylesheet declarations.
   */
  function applySystemAccent() {
    // Read AccentColor from OS via a hidden element
    const probe = document.createElement('div');
    probe.style.cssText = 'position:fixed;opacity:0;pointer-events:none;color:AccentColor';
    document.body.appendChild(probe);
    const raw = getComputedStyle(probe).color; // "rgb(r, g, b)"
    document.body.removeChild(probe);

    const m = raw.match(/(\d+),?\s*(\d+),?\s*(\d+)/);
    if (!m) return;
    const accent: [number, number, number] = [+m[1], +m[2], +m[3]];
    // No real accent available (headless, non-macOS) — keep Rokkit's default violet
    if (accent.every(v => v === 0) || accent.every(v => v === 255)) return;

    const dark = window.matchMedia('(prefers-color-scheme: dark)').matches;
    // Base color to mix toward for z1–z5 tints
    const base: [number, number, number] = dark ? [15, 23, 42] : [248, 250, 252];
    // Direction for z7–z9 (text-weight tones)
    const shade: [number, number, number] = dark ? [255, 255, 255] : [0, 0, 0];

    function mix(a: [number,number,number], b: [number,number,number], t: number): string {
      return [0,1,2].map(i => Math.round(a[i] + (b[i] - a[i]) * t)).join(',');
    }

    const r = document.documentElement;
    r.style.setProperty('--color-primary-z1', mix(accent, base, 0.92));
    r.style.setProperty('--color-primary-z2', mix(accent, base, 0.82));
    r.style.setProperty('--color-primary-z3', mix(accent, base, 0.68));
    r.style.setProperty('--color-primary-z4', mix(accent, base, 0.50));
    r.style.setProperty('--color-primary-z5', mix(accent, base, 0.28));
    r.style.setProperty('--color-primary-z6', accent.join(','));
    r.style.setProperty('--color-primary-z7', mix(shade, accent, 0.28));
    r.style.setProperty('--color-primary-z8', mix(shade, accent, 0.50));
    r.style.setProperty('--color-primary-z9', mix(shade, accent, 0.72));
  }

  function applyColorScheme() {
    const dark = window.matchMedia('(prefers-color-scheme: dark)').matches;
    document.body.dataset.mode = dark ? 'dark' : 'light';
    applySystemAccent();
  }

  onMount(() => {
    // Port resolution now happens in +layout.ts load() — runs before any page loader.

    applyColorScheme();
    const mq = window.matchMedia('(prefers-color-scheme: dark)');
    mq.addEventListener('change', applyColorScheme);

    const unlistens: Array<() => void> = [];
    if (typeof window !== 'undefined' && (window as any).__TAURI__) {
      import('@tauri-apps/api/event').then(({ listen }) => {
        listen<void>('open-logs', () => {
          goto('/logs');
        }).then(fn => unlistens.push(fn));

        // Dev View-menu navigation — bypasses routing guards for testing.
        // Sets health=ready in sessionStorage so the guard doesn't intercept.
        listen<string>('dev-navigate', (e) => {
          sessionStorage.setItem('sensei:health', 'ready');
          goto(e.payload, { replaceState: true });
        }).then(fn => unlistens.push(fn));
      });
    }

    return () => {
      mq.removeEventListener('change', applyColorScheme);
      unlistens.forEach(fn => fn());
    };
  });
</script>

{@render children()}
