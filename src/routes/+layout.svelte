<script lang="ts">
  import 'uno.css';
  import '../app.css';
  import { vibe } from '@rokkit/states';
  import { themable } from '@rokkit/actions';
  import { FloatingNavigation } from '@rokkit/ui';
  import { ThemeSwitcherToggle } from '@rokkit/app';
  import { base } from '$app/paths';

  let { children } = $props();

  const GITHUB = 'https://github.com/mizukisu/sensei-dev';

  const navItems = [
    { label: 'Home',         value: 'home',         href: '#',             icon: 'i-solar-home-2-bold-duotone' },
    { label: 'Setup',        value: 'setup',        href: '#setup',        icon: 'i-solar-download-square-bold-duotone' },
    { label: 'Features',     value: 'features',     href: '#features',     icon: 'i-solar-star-bold-duotone' },
    { label: 'Architecture', value: 'architecture', href: '#architecture', icon: 'i-solar-graph-bold-duotone' },
    { label: 'Reference',    value: 'usage',        href: '#usage',        icon: 'i-solar-document-text-bold-duotone' },
    { label: 'Sponsor',      value: 'sponsor',      href: '#sponsor',      icon: 'i-solar-heart-bold-duotone' },
  ];
</script>

<svelte:head>
  <title>Sensei</title>
  <meta name="description" content="AI Development Intelligence — capture how your team uses AI and feed that intelligence back into your workflow." />
</svelte:head>
<svelte:body use:themable={{ theme: vibe, storageKey: 'sensei-theme' }} />

<div class="min-h-screen overflow-y-auto bg-surface-z1 text-surface-z8">

  <!-- Header -->
  <nav class="sticky top-0 z-50 flex items-center justify-between border-b border-surface-z0 bg-surface-z1/95 backdrop-blur px-8 h-14">
    <a href="{base}/" class="flex items-center gap-2 font-bold text-base select-none no-underline text-surface-z8">
      <img src="{base}/sensei-icon.png" alt="" class="w-6 h-6 rounded" />
      <span>sensei</span>
    </a>
    <div class="flex items-center gap-4 text-sm">
      <a href="{base}/#setup" class="text-surface-z5 hover:text-surface-z8 transition-colors hidden sm:block">Setup</a>
      <a href="{base}/#features" class="text-surface-z5 hover:text-surface-z8 transition-colors hidden sm:block">Features</a>
      <a href="{base}/#architecture" class="text-surface-z5 hover:text-surface-z8 transition-colors hidden md:block">Architecture</a>
      <a href="{base}/#usage" class="text-surface-z5 hover:text-surface-z8 transition-colors hidden sm:block">Reference</a>
      <ThemeSwitcherToggle size="xs" />
      <a href={GITHUB} target="_blank" rel="noopener"
         class="flex items-center gap-1.5 rounded-lg border border-surface-z3 bg-surface-z2 px-3 py-1.5 text-xs font-medium hover:bg-surface-z3 transition-colors">
        <span class="i-solar-github-bold-duotone"></span> GitHub
      </a>
    </div>
  </nav>

  {@render children()}

  <!-- Footer -->
  <div class="border-t border-surface-z0 py-8 text-center text-xs text-surface-z4">
    <a href="{base}/" class="hover:text-surface-z6 transition-colors font-bold text-surface-z6">⬡ sensei</a>
    <span class="mx-2">·</span>
    <a href="{base}/#setup" class="hover:text-surface-z6 transition-colors">Setup</a>
    <span class="mx-2">·</span>
    <a href="{base}/#usage" class="hover:text-surface-z6 transition-colors">Reference</a>
    <span class="mx-2">·</span>
    <a href={GITHUB} target="_blank" rel="noopener" class="hover:text-surface-z6 transition-colors">GitHub</a>
  </div>

  <!-- Floating nav -->
  <FloatingNavigation items={navItems} position="right" size="sm" label="Navigation" />

</div>
