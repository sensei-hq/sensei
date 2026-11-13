#!/usr/bin/env node
// Builds the review sheet: for every captured gallery fixture, each matrix
// variant's component capture beside the mockup crop that `mockup-map.json`
// names for it (rendered on demand through e2e/mockup/render.mjs).
//
//   node e2e/gallery/sheet.mjs [--out <dir>] [--mockup <Relay.dc.html>] [--variants a,b] [--embed]
//
// index.html links the PNGs beside it and opens in any browser. `--embed` also
// writes sheet.artifact.html: self-contained (images inlined) and shaped for
// publishing as a page — it starts at <title>, no document wrapper.

import { readFileSync, writeFileSync, existsSync, mkdirSync } from 'node:fs';
import { resolve, relative, dirname } from 'node:path';
import { execFileSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';

const here = dirname(fileURLToPath(import.meta.url));
const args = process.argv.slice(2);
const opt = (flag, dflt) => {
  const i = args.indexOf(flag);
  return i >= 0 ? args[i + 1] : dflt;
};

/** The first ancestor of `from` holding `rel`, or null — the coord folder sits
 *  beside the repo, at a depth that differs between a lane worktree and the
 *  main checkout. */
function findUp(from, rel) {
  let dir = resolve(from);
  for (;;) {
    const candidate = resolve(dir, rel);
    if (existsSync(candidate)) return candidate;
    const parent = dirname(dir);
    if (parent === dir) return null;
    dir = parent;
  }
}
const OUT = resolve(opt('--out', process.env.GALLERY_OUT ?? resolve(here, 'out')));
const MOCKUP = resolve(opt('--mockup', process.env.RELAY_MOCKUP ?? findUp(here, 'coord/mockups/relay-lanes/Relay.dc.html') ?? ''));
if (!existsSync(MOCKUP)) {
  console.error(`no mockup at ${MOCKUP || '(unset)'} — pass --mockup or RELAY_MOCKUP`);
  process.exit(2);
}
const manifestPath = resolve(OUT, 'components', 'manifest.json');
if (!existsSync(manifestPath)) {
  console.error(`no capture manifest at ${manifestPath} — run the capture first (bun run gallery:capture)`);
  process.exit(2);
}
const manifest = JSON.parse(readFileSync(manifestPath, 'utf8'));
const map = JSON.parse(readFileSync(resolve(here, 'mockup-map.json'), 'utf8'));
const mockupDir = resolve(OUT, 'mockup');
mkdirSync(mockupDir, { recursive: true });

const VARIANTS = ['wide-light-kanji', 'wide-light-icon', 'wide-dark-kanji', 'wide-dark-icon', 'phone-light-kanji', 'phone-light-icon', 'phone-dark-kanji', 'phone-dark-icon'];
const embed = args.includes('--embed');
const only = opt('--variants', '').split(',').filter(Boolean);
const chosen = only.length ? VARIANTS.filter((v) => only.includes(v)) : VARIANTS;
const rendered = new Map();

/** The mockup crop for one component fixture in one variant, rendered once per
 *  (state, variant, crop); `null` when the map names no mockup region for it. */
function mockupCrop(id, variant) {
  const entry = map[id] ?? map[id.split('/')[0]];
  if (!entry) return null;
  const [width, theme, glyph] = variant.split('-');
  const state = { ...entry.state, theme, glyph, width };
  const key = `${JSON.stringify(state)}|${entry.crop}`;
  if (rendered.has(key)) return rendered.get(key);
  const name = `${entry.state.view ?? 'now'}-${entry.state.load ?? 'loaded'}-${variant}-${id.replace(/[^A-Za-z0-9]+/g, '-')}`;
  execFileSync('node', [resolve(here, '../mockup/render.mjs'), MOCKUP, '--state', JSON.stringify(state), '--out', mockupDir, '--name', name, '--shot', '--crop', entry.crop], {
    stdio: 'pipe',
  });
  const crop = resolve(mockupDir, `${name}.1.png`);
  const file = existsSync(crop) ? crop : null;
  rendered.set(key, file);
  return file;
}

const esc = (s) => String(s).replace(/[&<>"]/g, (c) => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;' })[c]);
const rel = (p) => relative(OUT, p);
const dataUri = (p) => `data:image/png;base64,${readFileSync(p).toString('base64')}`;
const label = (variant) => variant.replace('wide', 'desktop').replaceAll('-', ' · ');

/** One fixture's block: its id and note, then a mockup | component pair per variant. */
function fixtureBlock(id, { note, shots }, src) {
  const [component, fixture] = id.split('/');
  let pairs = '';
  let mapped = 0;
  for (const variant of chosen) {
    const shot = shots[variant];
    if (!shot) continue;
    const mock = mockupCrop(id, variant);
    if (mock) mapped += 1;
    pairs += `<div class="pair"><figure><figcaption>mockup · ${esc(label(variant))}</figcaption>${
      mock ? `<img src="${esc(src(mock))}" alt="mockup region for ${esc(id)}, ${esc(label(variant))}" loading="lazy">` : '<p class="none">no mockup region for this fixture</p>'
    }</figure><figure><figcaption>component · ${esc(label(variant))}</figcaption><img src="${esc(src(shot))}" alt="${esc(id)} as built, ${esc(label(variant))}" loading="lazy"></figure></div>`;
  }
  return `<section class="fixture" id="${esc(id.replace(/[^A-Za-z0-9]+/g, '-'))}">
<header class="fixture-head"><h2><span class="component">${esc(component)}</span><span class="sep">/</span><span class="fixture-name">${esc(fixture ?? '')}</span></h2><p class="note">${esc(note)}</p><p class="meta">${mapped ? `${mapped} of ${chosen.length} variants have a mockup region` : 'no mockup region mapped'}</p></header>
${pairs}</section>`;
}

const ids = Object.keys(manifest);
const components = [...new Set(ids.map((id) => id.split('/')[0]))];
const toc = components
  .map((c) => `<li><a href="#${esc(c)}">${esc(c)}</a> <span class="count">${ids.filter((id) => id.startsWith(`${c}/`)).length}</span></li>`)
  .join('');
const generated = new Date().toISOString().slice(0, 16).replace('T', ' ');

const STYLE = `
<style>
/* Layout: one reading column; each fixture a labelled block of mockup | component pairs that stack on a phone. */
:root {
  --paper: #f6f3ec; --paper-soft: #efebe2; --edge: #d9d2c4;
  --ink: #2a2522; --ink-soft: #5c554e; --ink-mute: #7d766e;
  --accent: #b4502c; --mark: #2e7a5f;
  --display: Fraunces, "Iowan Old Style", Georgia, serif;
  --body: Inter, "Helvetica Neue", Arial, sans-serif;
  --mono: "JetBrains Mono", "SF Mono", Menlo, Consolas, monospace;
}
@media (prefers-color-scheme: dark) {
  :root:not([data-theme="light"]) {
    --paper: #1c1917; --paper-soft: #252120; --edge: #3e3834;
    --ink: #ede7df; --ink-soft: #c3bbb0; --ink-mute: #958d83;
    --accent: #e08a62; --mark: #6cbf9b; color-scheme: dark;
  }
}
:root[data-theme="dark"] {
  --paper: #1c1917; --paper-soft: #252120; --edge: #3e3834;
  --ink: #ede7df; --ink-soft: #c3bbb0; --ink-mute: #958d83;
  --accent: #e08a62; --mark: #6cbf9b; color-scheme: dark;
}
body { margin: 0; background: var(--paper); color: var(--ink); font: 15px/1.5 var(--body);
  padding-block: 0 48px; padding-left: max(16px, env(safe-area-inset-left, 0px)); padding-right: max(16px, env(safe-area-inset-right, 0px)); }
.wrap { max-width: 1200px; margin: 0 auto; display: flex; flex-direction: column; gap: 28px; }
h1 { font: 500 32px/1.15 var(--display); margin: 0; text-wrap: balance; }
h2 { font: 500 20px/1.2 var(--display); margin: 0; }
.lede { max-width: 68ch; color: var(--ink-soft); margin: 0; }
.top { padding-top: 36px; display: flex; flex-direction: column; gap: 12px; border-bottom: 1px solid var(--edge); padding-bottom: 24px; }
.facts { display: flex; flex-wrap: wrap; gap: 6px 22px; font: 12px/1.4 var(--mono); color: var(--ink-mute); letter-spacing: .02em; }
.facts b { color: var(--ink); font-weight: 500; }
.how { display: grid; grid-template-columns: repeat(auto-fit, minmax(240px, 1fr)); gap: 14px 24px; max-width: 980px; }
.how h3 { font: 600 11px/1.3 var(--body); letter-spacing: .12em; text-transform: uppercase; color: var(--ink-mute); margin: 0 0 4px; }
.how p { margin: 0; color: var(--ink-soft); font-size: 14px; }
.toc { display: flex; flex-wrap: wrap; gap: 6px 10px; list-style: none; margin: 0; padding: 0; }
.toc li { display: inline-flex; align-items: center; gap: 6px; }
.toc a { color: var(--ink); text-decoration: none; border: 1px solid var(--edge); border-radius: 999px; padding: 3px 10px; font-size: 13px; }
.toc a:hover, .toc a:focus-visible { border-color: var(--accent); outline: none; }
.count { font: 12px/1 var(--mono); color: var(--ink-mute); }
.fixture { display: flex; flex-direction: column; gap: 14px; padding-top: 22px; border-top: 1px solid var(--edge); }
.fixture-head { display: flex; flex-wrap: wrap; align-items: baseline; gap: 4px 14px; }
.component { color: var(--accent); }
.sep { color: var(--ink-mute); padding: 0 6px; font-weight: 400; }
.fixture-name { font-family: var(--mono); font-size: 16px; font-weight: 500; }
.note { margin: 0; color: var(--ink-soft); }
.meta { margin: 0; flex-basis: 100%; font: 12px/1.4 var(--mono); color: var(--ink-mute); }
.pair { display: grid; grid-template-columns: 1fr 1fr; gap: 14px; }
figure { margin: 0; display: flex; flex-direction: column; gap: 6px; min-width: 0; }
figcaption { font: 11px/1.3 var(--mono); letter-spacing: .08em; text-transform: uppercase; color: var(--ink-mute); }
figure img { max-width: 100%; height: auto; display: block; border: 1px solid var(--edge); border-radius: 8px; background: var(--paper-soft); }
.none { margin: 0; color: var(--ink-mute); font-style: italic; border: 1px dashed var(--edge); border-radius: 8px; padding: 14px; font-size: 13px; }
@media (max-width: 640px) { .pair { grid-template-columns: 1fr; } h1 { font-size: 26px; } }
@media (prefers-reduced-motion: no-preference) { .toc a { transition: border-color .15s; } }
</style>`;

const HEAD = `<title>Relay Component Bench</title>
<meta name="claude:viewport-fit-sides" content="cover">
<link rel="stylesheet" href="https://fonts.googleapis.com/css2?family=Fraunces:opsz,wght@9..144,500&family=Inter:wght@400;500;600&family=JetBrains+Mono:wght@400;500&display=swap">
${STYLE}`;

const body = (src) => `<div class="wrap">
<header class="top">
  <h1>Relay Component Bench</h1>
  <p class="lede">Every relay component the app has today, rendered from a fixture, beside the region of the r1 mockup it stands in for. Same theme, width and glyph mode on both sides. The right column is what ships now; the left is the target. Differences are the work.</p>
  <div class="facts"><span>fixtures <b>${ids.length}</b></span><span>components <b>${components.length}</b></span><span>variants <b>${chosen.map(label).join(' · ')}</b></span><span>generated <b>${generated} UTC</b></span><span>mockup <b>Relay.dc.html (2026-11-10)</b></span></div>
  <div class="how">
    <div><h3>Mockup side</h3><p>The artboard's own render logic run for the state the component lives in, then the one region cropped. Not a screenshot of the canvas tool: a reproduction, so any state can be shown.</p></div>
    <div><h3>Component side</h3><p>The dev gallery at <span style="font-family:var(--mono)">/gallery</span>, where each component renders its fixture on plain paper. Nothing reads the daemon.</p></div>
    <div><h3>How to give feedback</h3><p>Name the fixture (component / fixture) and the variant. "Matches", "close, fix X", or "wrong shape" is enough; the issue for that component carries the fix.</p></div>
  </div>
  <ul class="toc">${toc}</ul>
</header>
${ids.map((id) => fixtureBlock(id, manifest[id], src)).join('\n')}
</div>`;

const sheet = resolve(OUT, 'index.html');
writeFileSync(sheet, `<!doctype html><html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width, initial-scale=1">${HEAD}</head><body>${body(rel)}</body></html>`);
console.log(`review sheet: ${sheet} (${ids.length} fixtures, ${chosen.length} variants)`);
if (embed) {
  const artifact = resolve(OUT, 'sheet.artifact.html');
  writeFileSync(artifact, `${HEAD}\n${body(dataUri)}`);
  console.log(`self-contained sheet for publishing: ${artifact}`);
}
