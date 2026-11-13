#!/usr/bin/env node
// Builds the contact sheet: for every captured gallery fixture, each matrix
// variant's component capture beside the mockup crop that `mockup-map.json`
// names for it (rendered on demand through e2e/mockup/render.mjs). The sheet
// is one HTML file next to the captures, opened in a browser for review.
//
//   node e2e/gallery/sheet.mjs [--out <dir>] [--mockup <Relay.dc.html>]

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
const rendered = new Map();

/** The mockup crop for one component fixture in one variant, rendered once per
 *  (state, variant); `null` when the map names no mockup region for it. */
function mockupCrop(id, variant) {
  const entry = map[id] ?? map[id.split('/')[0]];
  if (!entry) return null;
  const [width, theme, glyph] = variant.split('-');
  const state = { ...entry.state, theme, glyph, width };
  const key = `${JSON.stringify(state)}|${entry.crop}`;
  if (rendered.has(key)) return rendered.get(key);
  const name = `${(entry.state.view ?? 'now')}-${entry.state.load ?? 'loaded'}-${variant}-${id.replace(/[^A-Za-z0-9]+/g, '-')}`;
  execFileSync('node', [resolve(here, '../mockup/render.mjs'), MOCKUP, '--state', JSON.stringify(state), '--out', mockupDir, '--name', name, '--shot', '--crop', entry.crop], { stdio: 'pipe' });
  const crop = resolve(mockupDir, `${name}.1.png`);
  const file = existsSync(crop) ? crop : null;
  rendered.set(key, file);
  return file;
}

const esc = (s) => String(s).replace(/[&<>"]/g, (c) => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;' })[c]);
// `--embed` writes a second, self-contained sheet (images as data URIs) that can
// be handed over as one file; `--variants a,b` limits both sheets to those.
const embed = args.includes('--embed');
const only = opt('--variants', '')
  .split(',')
  .filter(Boolean);
const chosen = only.length ? VARIANTS.filter((v) => only.includes(v)) : VARIANTS;
const rel = (p) => relative(OUT, p);
const dataUri = (p) => `data:image/png;base64,${readFileSync(p).toString('base64')}`;
let rows = '';
let rowsEmbedded = '';
for (const [id, captures] of Object.entries(manifest)) {
  rows += `<section class="fixture"><h2>${esc(id)}</h2>`;
  rowsEmbedded += `<section class="fixture"><h2>${esc(id)}</h2>`;
  for (const variant of chosen) {
    const shot = captures[variant];
    if (!shot) continue;
    const mock = mockupCrop(id, variant);
    const pair = (src) =>
      `<div class="pair"><div class="cell"><div class="label">mockup · ${esc(variant)}</div>${
        mock ? `<img src="${esc(src(mock))}" alt="mockup ${esc(id)} ${esc(variant)}">` : '<div class="none">no mockup region mapped</div>'
      }</div><div class="cell"><div class="label">component · ${esc(variant)}</div><img src="${esc(src(shot))}" alt="component ${esc(id)} ${esc(variant)}"></div></div>`;
    rows += pair(rel);
    if (embed) rowsEmbedded += pair(dataUri);
  }
  rows += '</section>';
  rowsEmbedded += '</section>';
}
const page = (body) => `<!doctype html><meta charset="utf-8"><title>Relay components · mockup vs component</title>
<style>
body{margin:0;padding:24px 32px;font:14px/1.45 Inter,system-ui,sans-serif;background:#f3f0e8;color:#2a2522}
h1{font:500 24px Fraunces,serif;margin:0 0 6px}h2{font:500 18px Fraunces,serif;margin:28px 0 10px;padding-top:16px;border-top:1px solid #dcd6ca}
.pair{display:grid;grid-template-columns:1fr 1fr;gap:16px;margin:0 0 14px}.cell{background:#faf8f3;border:1px solid #dcd6ca;border-radius:10px;padding:10px}
.label{font:11px/1 JetBrains Mono,monospace;letter-spacing:.08em;text-transform:uppercase;color:#6b655e;margin-bottom:8px}
img{max-width:100%;height:auto;display:block;border:1px solid #ebe6db;border-radius:6px}.none{color:#6b655e;font-style:italic}
</style>
<h1>Relay components · mockup vs component</h1>
<p>Each row: the mockup region the component replaces (rendered from <code>Relay.dc.html</code> in the same theme, width and glyph mode), beside the component rendered from its fixture on the dev gallery. Generated ${new Date().toISOString()}.</p>
${body}`;
const sheet = resolve(OUT, 'index.html');
writeFileSync(sheet, page(rows));
console.log(`contact sheet: ${sheet} (${Object.keys(manifest).length} fixtures, ${chosen.length} variants)`);
if (embed) {
  const embedded = resolve(OUT, 'index.embedded.html');
  writeFileSync(embedded, page(rowsEmbedded));
  console.log(`self-contained sheet: ${embedded}`);
}
