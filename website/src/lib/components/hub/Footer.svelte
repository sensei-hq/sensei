<script lang="ts">
  import { base } from '$app/paths';

  const cols = [
    {
      title: 'Products',
      items: [
        { label: 'Sensei', href: `${base}/sensei`, external: false },
        { label: 'Torii', href: `${base}/torii-seiki`, external: false },
        { label: 'Seiki', href: `${base}/torii-seiki#seiki`, external: false },
      ],
    },
    {
      title: 'Libraries',
      items: [
        { label: 'Gateway', href: 'https://gateway.sensei-hq.com', external: true },
        { label: 'DBD', href: 'https://dbd.sensei-hq.com', external: true },
        { label: 'Rokkit', href: 'https://rokkit.sensei-hq.com', external: true },
        { label: 'Kavach', href: 'https://kavach.sensei-hq.com', external: true },
      ],
    },
    {
      title: 'Studio',
      items: [
        { label: 'Approach', href: '#approach', external: false },
        { label: 'Open source', href: '#open', external: false },
        { label: 'Contact', href: '#contact', external: false },
      ],
    },
    {
      title: 'Connect',
      items: [
        { label: 'GitHub', href: 'https://github.com/sensei-hq', external: true },
        { label: 'Twitter', href: 'https://twitter.com/sensei_hq', external: true },
        { label: 'hi@sensei-hq.com', href: 'mailto:hi@sensei-hq.com', external: false },
      ],
    },
  ];
</script>

<footer class="border-t border-paper-edge">
  <div class="footer-inner mx-auto px-7 py-8">
    <div class="footer-grid gap-7">
      <!-- Brand -->
      <div>
        <div class="flex items-center gap-3 mb-3">
          <!-- Sensei HQ brushed-ensō mark (i-brand:sensei-hq), tinted to the brand vermillion -->
          <span class="i-brand:sensei-hq text-accent block flex-shrink-0" style="width:26px;height:26px" aria-hidden="true"></span>
          <span class="flex items-baseline gap-2">
            <span class="display text-ink text-base">Sensei</span>
            <span class="mono text-ink-mute text-xs" style="letter-spacing:0.08em">HQ</span>
          </span>
        </div>
        <p class="text-ink-mute m-0 text-sm" style="line-height:1.6;max-width:280px">
          A small workshop of developer tools. Built quietly, in Tennessee.
        </p>
      </div>

      <!-- Link columns -->
      {#each cols as col (col.title)}
        <div>
          <span class="mono text-ink-mute block mb-4 text-xs" style="letter-spacing:0.12em;text-transform:uppercase">{col.title}</span>
          <div class="flex flex-col gap-2">
            {#each col.items as item (item.label)}
              <a href={item.href}
                 target={item.external ? '_blank' : undefined}
                 rel={item.external ? 'noopener' : undefined}
                 class="text-ink-soft text-sm footer-link no-underline">{item.label}</a>
            {/each}
          </div>
        </div>
      {/each}
    </div>

    <div class="border-t border-paper-edge mt-7 pt-5 flex items-center justify-between flex-wrap gap-3">
      <span class="mono text-ink-faint text-xs">© 2026 Sensei HQ · All rights reserved</span>
      <span class="mono text-ink-faint text-xs">道 · the way</span>
    </div>
  </div>
</footer>

<style>
  .footer-inner {
    max-width: 1120px;
  }
  .footer-grid {
    display: grid;
    grid-template-columns: 1.4fr 1fr 1fr 1fr 1fr;
    align-items: start;
  }
  .footer-link {
    transition: color 0.15s;
  }
  .footer-link:hover {
    color: var(--ink);
  }
  @media (max-width: 768px) {
    .footer-grid {
      grid-template-columns: 1fr 1fr;
    }
  }
</style>
