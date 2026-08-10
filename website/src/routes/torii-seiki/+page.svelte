<script lang="ts">
  import { Button } from '@rokkit/ui';
  import { base } from '$app/paths';
  import {
    CLIENTS,
    PLANES,
    CAPABILITIES,
    PRIVACY_ROWS,
    GATEWAY_ROWS,
    GATEWAY_URL,
    TS_NAV_LINKS,
  } from '$lib/torii-seiki-data.js';
</script>

<svelte:head>
  <title>Torii · Seiki — the gate and the governance plane · Sensei HQ</title>
  <meta
    name="description"
    content="Torii is the desktop client your team walks through to reach your models; Seiki is the self-hosted governance plane behind it — routing, budgets, audit. Both built on Gateway." />
</svelte:head>

<!-- ── Nav ───────────────────────────────────────────────────────────── -->
<div class="ts-nav-wrap sticky top-0 z-50">
  <nav class="ts-inner mx-auto px-7 py-4 flex items-center justify-between">
    <a href="{base}/" class="flex items-center gap-3 no-underline">
      <span class="i-brand:sensei-hq text-accent block flex-shrink-0" style="width:26px;height:26px" aria-hidden="true"></span>
      <span class="flex items-baseline gap-2">
        <span class="display text-ink text-lg" style="letter-spacing:-0.01em">Torii · Seiki</span>
        <span class="mono text-ink-mute text-xs" style="letter-spacing:0.08em">SENSEI HQ</span>
      </span>
    </a>
    <div class="flex items-center gap-8">
      {#each TS_NAV_LINKS as [href, label] (href)}
        <a {href} class="ts-nav-link text-ink-soft text-sm no-underline">{label}</a>
      {/each}
      <Button href="#contact" variant="primary" size="sm" label="Request access" class="ml-1" />
    </div>
  </nav>
</div>

<!-- ── Hero ──────────────────────────────────────────────────────────── -->
<header id="top" class="ts-inner mx-auto px-7 pt-12 pb-24">
  <div class="ts-hero-grid gap-16">
    <div>
      <div class="flex items-baseline gap-3 mb-6">
        <span class="kanji text-accent" style="font-size:44px;line-height:1">門</span>
        <span class="mono text-ink-mute text-xs" style="letter-spacing:0.12em;text-transform:uppercase">Torii · Seiki — the gate and what stands behind it</span>
      </div>
      <h1 class="display text-ink m-0" style="font-size:56px;font-weight:300;line-height:1.08;letter-spacing:-0.025em;max-width:640px">
        The gate, and the sanctuary behind it.
      </h1>
      <p class="text-ink-soft mt-6 m-0" style="font-size:17px;line-height:1.6;max-width:540px">
        Torii is the gate your team walks through to reach your models. Seiki is the
        governance plane behind it — routing, budgets, audit. Both stand on Gateway,
        our Rust library for fallback chains and budget control.
      </p>
      <div class="flex items-center flex-wrap gap-3 mt-8">
        <Button href="mailto:hi@sensei-hq.com" variant="primary" size="lg">
          <span class="kanji text-on-primary text-base" style="line-height:1">入</span>
          Request access
        </Button>
        <a href="#clients" class="text-ink-soft text-sm no-underline">Meet Torii and Seiki ↓</a>
      </div>
      <div class="flex flex-wrap gap-4 mt-12">
        {#each ['Self-hosted', 'Bring your own keys', 'Built on Gateway'] as m (m)}
          <span class="mono text-ink-mute text-xs">{m}</span>
        {/each}
      </div>
    </div>

    <aside class="border border-paper-edge rounded-lg bg-paper-soft" style="overflow:hidden">
      <div class="px-6 py-4 border-b border-paper-edge flex items-center justify-between">
        <span class="mono text-ink-mute text-xs" style="letter-spacing:0.12em;text-transform:uppercase">The pair</span>
        <span class="mono text-ink-faint text-xs">02</span>
      </div>
      <div>
        {#each CLIENTS as c (c.id)}
          <a href="#{c.id}" class="ts-pair-row gap-4 px-6 py-4 no-underline text-{c.id}">
            <span class="kanji text-{c.id} text-center" style="font-size:26px;line-height:1;width:30px">{c.kanji}</span>
            <span>
              <span class="display text-ink block text-base">{c.name}</span>
              <span class="text-ink-mute text-xs">{c.category}</span>
            </span>
            <span class="ts-dot bg-{c.id}"></span>
          </a>
        {/each}
      </div>
      <div class="px-6 py-4 border-t border-paper-edge">
        <p class="text-ink-mute m-0 text-sm" style="line-height:1.6">
          Pass through the gate and the rules are already in force. That is the point of the pair.
        </p>
      </div>
    </aside>
  </div>
</header>

<!-- ── Two planes ────────────────────────────────────────────────────── -->
<section id="planes" class="border-t border-paper-edge py-24">
  <div class="ts-inner mx-auto px-7">
    <div class="mb-12">
      <div class="flex items-baseline gap-3 mb-4">
        <span class="kanji text-accent text-2xl" style="line-height:1">分</span>
        <span class="mono text-ink-mute text-xs" style="letter-spacing:0.12em;text-transform:uppercase">Bun · to divide</span>
      </div>
      <h2 class="display text-ink m-0 text-3xl" style="font-weight:300;line-height:1.15;letter-spacing:-0.022em;max-width:720px">
        Every answer knows where it ran.
      </h2>
      <p class="text-ink-soft mt-4 m-0 text-lg" style="line-height:1.6;max-width:560px">
        The split execution plane is not a setting buried in preferences — it is written on the response, every time.
      </p>
    </div>
    <div class="ts-two-col gap-6">
      {#each PLANES as p (p.label)}
        <div class="border border-paper-edge rounded-lg bg-paper-soft p-7">
          <div class="flex items-center gap-3 mb-4">
            <span class="kanji text-accent text-xl" style="line-height:1">{p.kanji}</span>
            <span class="mono text-ink border border-paper-edge rounded-full text-xs px-2.5 py-1">{p.label}</span>
          </div>
          <p class="text-ink-soft m-0 text-base" style="line-height:1.65">{p.text}</p>
        </div>
      {/each}
    </div>
  </div>
</section>

<!-- ── Clients ───────────────────────────────────────────────────────── -->
<section id="clients" class="border-t border-paper-edge py-24">
  <div class="ts-inner mx-auto px-7">
    <div class="mb-12">
      <div class="flex items-baseline gap-3 mb-4">
        <span class="kanji text-accent text-2xl" style="line-height:1">二</span>
        <span class="mono text-ink-mute text-xs" style="letter-spacing:0.12em;text-transform:uppercase">Ni · two</span>
      </div>
      <h2 class="display text-ink m-0 text-3xl" style="font-weight:300;line-height:1.15;letter-spacing:-0.022em;max-width:720px">
        Two tools, one temperament.
      </h2>
      <p class="text-ink-soft mt-4 m-0 text-lg" style="line-height:1.6;max-width:560px">
        Torii is what your team opens in the morning. Seiki is where you go when you need to know why — and to change it.
      </p>
    </div>
    <div class="flex flex-col gap-8">
      {#each CLIENTS as c, i (c.id)}
        <div id={c.id} class="ts-client border border-paper-edge rounded-lg text-{c.id}"
             class:flip={i % 2 === 1} style="overflow:hidden">
          <div class="ts-client-body p-10">
            <div class="mb-6 flex items-center justify-between">
              <span class="mono text-ink-faint text-sm">{String(i + 1).padStart(2, '0')}</span>
              <span class="mono rounded-sm px-2 py-0 text-warning bg-warning-soft text-xs" style="white-space:nowrap">{c.status}</span>
            </div>
            <div class="flex items-baseline flex-wrap gap-3 mb-2">
              <span class="display text-ink text-2xl" style="font-weight:400;letter-spacing:-0.02em">{c.name}</span>
              <span class="mono text-ink-mute text-xs" style="letter-spacing:0.1em;text-transform:uppercase">{c.category}</span>
            </div>
            <p class="display text-ink m-0 text-xl" style="font-weight:300;letter-spacing:-0.015em;line-height:1.3">{c.tagline}</p>
            <p class="text-ink-soft mt-3 text-base" style="line-height:1.65;max-width:460px">{c.blurb}</p>
            <div class="flex flex-wrap gap-2 mt-6 mb-8">
              {#each c.surfaces as s (s)}
                <span class="text-ink-soft text-sm border border-paper-edge rounded-full px-3 py-1" style="white-space:nowrap">{s}</span>
              {/each}
            </div>
            <div class="flex flex-wrap gap-2">
              {#each c.meta as m (m)}
                <span class="mono text-ink-mute border border-paper-edge rounded-sm text-xs px-2 py-0.5" style="white-space:nowrap">{m}</span>
              {/each}
            </div>
          </div>
          <div class="ts-client-panel">
            <span class="kanji text-{c.id}" style="font-size:220px;line-height:1;opacity:0.92">{c.kanji}</span>
            <span class="mono text-ink-faint" style="position:absolute;bottom:18px;right:20px;font-size:11px;letter-spacing:0.14em">{c.gloss}</span>
          </div>
        </div>
      {/each}
    </div>
  </div>
</section>

<!-- ── Capabilities ──────────────────────────────────────────────────── -->
<section id="capabilities" class="border-t border-paper-edge py-24">
  <div class="ts-inner mx-auto px-7">
    <div class="mb-12">
      <div class="flex items-baseline gap-3 mb-4">
        <span class="kanji text-accent text-2xl" style="line-height:1">具</span>
        <span class="mono text-ink-mute text-xs" style="letter-spacing:0.12em;text-transform:uppercase">Gu · the instruments</span>
      </div>
      <h2 class="display text-ink m-0 text-3xl" style="font-weight:300;line-height:1.15;letter-spacing:-0.022em;max-width:720px">
        What the pair is made of.
      </h2>
    </div>
    <div class="ts-cap-grid gap-x-12 gap-y-12">
      {#each CAPABILITIES as c (c.title)}
        <div>
          <span class="kanji text-accent block mb-4" style="font-size:32px;line-height:1">{c.kanji}</span>
          <h3 class="display text-ink m-0 text-lg" style="font-weight:400;letter-spacing:-0.01em">{c.title}</h3>
          <p class="text-ink-soft mt-2 m-0 text-sm" style="line-height:1.65">{c.text}</p>
        </div>
      {/each}
    </div>
  </div>
</section>

<!-- ── Privacy ───────────────────────────────────────────────────────── -->
<section id="privacy" class="border-t border-paper-edge py-24">
  <div class="ts-inner mx-auto px-7">
    <div class="ts-split gap-16">
      <div>
        <div class="flex items-baseline gap-3 mb-4">
          <span class="kanji text-accent text-2xl" style="line-height:1">蔵</span>
          <span class="mono text-ink-mute text-xs" style="letter-spacing:0.12em;text-transform:uppercase">Zō · to keep</span>
        </div>
        <h2 class="display text-ink m-0 text-3xl" style="font-weight:300;line-height:1.15;letter-spacing:-0.022em;max-width:460px">
          Your prompts stay in your building.
        </h2>
        <p class="text-ink-soft mt-4 m-0 text-base" style="line-height:1.7;max-width:460px">
          Both clients run on your own infrastructure. There is no Sensei HQ account, no
          telemetry back to us, and no copy of your traffic anywhere we can read it.
          You hold the provider keys; you choose the region; you can export or delete
          the whole audit log whenever you like.
        </p>
      </div>
      <div class="border border-paper-edge rounded-lg bg-paper-soft p-7">
        <span class="mono text-ink-mute block mb-4 text-xs" style="letter-spacing:0.12em;text-transform:uppercase">What leaves your network</span>
        <div class="flex flex-col">
          {#each PRIVACY_ROWS as row, i (row.term)}
            <div class="ts-detail-row py-3 flex items-baseline justify-between gap-6" class:last={i === PRIVACY_ROWS.length - 1}>
              <span class="text-ink text-sm">{row.term}</span>
              <span class="text-ink-mute text-sm" style="text-align:right;max-width:240px">{row.detail}</span>
            </div>
          {/each}
        </div>
      </div>
    </div>
  </div>
</section>

<!-- ── Gateway ───────────────────────────────────────────────────────── -->
<section id="gateway" class="border-t border-paper-edge py-24">
  <div class="ts-inner mx-auto px-7">
    <div class="ts-split gap-16" style="align-items:center">
      <div>
        <div class="flex items-baseline gap-3 mb-4">
          <span class="kanji text-gateway text-2xl" style="line-height:1">基</span>
          <span class="mono text-ink-mute text-xs" style="letter-spacing:0.12em;text-transform:uppercase">Ki · the foundation</span>
        </div>
        <h2 class="display text-ink m-0 text-3xl" style="font-weight:300;line-height:1.15;letter-spacing:-0.022em;max-width:460px">
          Both are built on Gateway.
        </h2>
        <p class="text-ink-soft mt-4 m-0 text-base" style="line-height:1.7;max-width:460px">
          Gateway is our Rust library for talking to model providers: fallback chains,
          budget control, and one interface across every vendor you use. Torii and Seiki
          are what it looks like with a face on it — but the library stands alone, and
          you can build your own client on it.
        </p>
        <Button href={GATEWAY_URL} target="_blank" rel="noopener" style="outline" size="md" class="mt-6" label="gateway.sensei-hq.com →" />
      </div>
      <div class="border border-paper-edge rounded-lg bg-paper-soft p-7">
        <span class="mono text-ink-mute block mb-4 text-xs" style="letter-spacing:0.12em;text-transform:uppercase">What Gateway handles</span>
        <div class="flex flex-col">
          {#each GATEWAY_ROWS as row, i (row.term)}
            <div class="ts-detail-row py-3 flex items-baseline justify-between gap-6" class:last={i === GATEWAY_ROWS.length - 1}>
              <span class="text-ink text-sm">{row.term}</span>
              <span class="text-ink-mute text-sm" style="text-align:right;max-width:240px">{row.detail}</span>
            </div>
          {/each}
        </div>
      </div>
    </div>
  </div>
</section>

<!-- ── Contact ───────────────────────────────────────────────────────── -->
<section id="contact" class="border-t border-paper-edge py-24">
  <div class="mx-auto px-7 text-center" style="max-width:720px">
    <span class="kanji text-accent block mb-5 text-3xl" style="line-height:1">入</span>
    <h2 class="display text-ink m-0 text-3xl" style="font-weight:300;line-height:1.15;letter-spacing:-0.022em">
      Torii and Seiki are in private beta.
    </h2>
    <p class="text-ink-soft mt-4 m-0 text-lg" style="line-height:1.6">
      Tell us how your organization works and we will tell you honestly whether the gate fits yet.
    </p>
    <div class="flex items-center justify-center flex-wrap gap-3 mt-8">
      <Button href="mailto:hi@sensei-hq.com" variant="primary" size="lg">
        <span class="kanji text-on-primary text-base" style="line-height:1">文</span>
        Request access
      </Button>
      <Button href="{base}/" style="outline" size="lg" label="See the whole workshop" />
    </div>
    <a href="mailto:hi@sensei-hq.com" class="mono text-ink-mute mt-4 inline-block text-sm" style="letter-spacing:0.02em;white-space:nowrap">hi@sensei-hq.com</a>
  </div>
</section>

<!-- ── Footer ────────────────────────────────────────────────────────── -->
<footer class="border-t border-paper-edge">
  <div class="ts-inner mx-auto px-7 py-16 flex items-center justify-between flex-wrap gap-4">
    <a href="{base}/" class="flex items-center gap-3 no-underline">
      <span class="i-brand:sensei-hq text-accent block flex-shrink-0" style="width:24px;height:24px" aria-hidden="true"></span>
      <span class="flex items-baseline gap-2">
        <span class="display text-ink text-base">Sensei</span>
        <span class="mono text-ink-mute text-xs" style="letter-spacing:0.08em">HQ</span>
      </span>
    </a>
    <span class="mono text-ink-faint text-xs">門 · Torii · Seiki · built on Gateway</span>
  </div>
</footer>

<style>
  .ts-inner {
    max-width: 1120px;
  }
  .ts-nav-wrap {
    background: color-mix(in oklch, var(--paper) 80%, transparent);
    backdrop-filter: blur(14px) saturate(150%);
    -webkit-backdrop-filter: blur(14px) saturate(150%);
    -webkit-mask-image: linear-gradient(to bottom, #000 72%, transparent);
    mask-image: linear-gradient(to bottom, #000 72%, transparent);
    padding-bottom: 6px;
  }
  .ts-nav-link {
    transition: color 0.15s;
  }
  .ts-nav-link:hover {
    color: var(--ink);
  }
  .ts-hero-grid {
    display: grid;
    grid-template-columns: 1.55fr 1fr;
    align-items: start;
  }
  .ts-pair-row {
    display: grid;
    grid-template-columns: auto 1fr auto;
    align-items: center;
    border-top: 1px solid var(--paper-edge);
    transition: background 0.15s;
  }
  .ts-pair-row:first-child {
    border-top: none;
  }
  .ts-pair-row:hover {
    background: var(--paper-mute);
  }
  .ts-dot {
    display: inline-block;
    width: 8px;
    height: 8px;
    border-radius: 50%;
    flex-shrink: 0;
  }
  .ts-two-col {
    display: grid;
    grid-template-columns: 1fr 1fr;
  }
  .ts-cap-grid {
    display: grid;
    grid-template-columns: repeat(3, 1fr);
  }
  .ts-split {
    display: grid;
    grid-template-columns: 1fr 1fr;
    align-items: start;
  }
  .ts-client {
    display: grid;
    grid-template-columns: 1.3fr 1fr;
    transition: border-color 0.18s;
  }
  .ts-client.flip {
    grid-template-columns: 1fr 1.3fr;
  }
  .ts-client.flip .ts-client-body {
    order: 2;
  }
  .ts-client.flip .ts-client-panel {
    order: 1;
    border-left: none;
    border-right: 1px solid var(--paper-edge);
  }
  .ts-client-panel {
    display: flex;
    align-items: center;
    justify-content: center;
    background: var(--paper-mute);
    position: relative;
    min-height: 340px;
    overflow: hidden;
    border-left: 1px solid var(--paper-edge);
  }
  .ts-detail-row {
    border-bottom: 1px solid var(--paper-edge);
  }
  .ts-detail-row.last {
    border-bottom: none;
  }
  @media (max-width: 900px) {
    .ts-hero-grid,
    .ts-two-col,
    .ts-split,
    .ts-cap-grid,
    .ts-client,
    .ts-client.flip {
      grid-template-columns: 1fr;
    }
    .ts-client.flip .ts-client-panel {
      order: 2;
      border-right: none;
    }
    .ts-client-panel {
      border-left: none;
      border-top: 1px solid var(--paper-edge);
      min-height: 220px;
    }
  }
</style>
