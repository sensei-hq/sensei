#!/usr/bin/env node
// Render a `.dc.html` design-canvas mockup to static HTML for one state, and
// optionally screenshot it. The canvas runtime (support.js / DCLogic) is not in
// the tree; the mockup's own `renderVals()` is self-contained, so this runs it
// in a Node vm context with a stub base class and expands the template
// (`sc-for`, `sc-if`, `{{path}}`) the way the canvas would, minus interaction.
// The script it runs is the designer's local mockup file, named on the command
// line — a trusted input, never a remote one.
//
//   node e2e/mockup/render.mjs <mockup.dc.html> --state '{"view":"crew","theme":"dark","width":"phone"}' \
//        --out <dir> [--name <base>] [--shot] [--crop '<css selector>' ...]
//
// `--shot` writes <name>.png of the screen frame (the element whose style came
// from `frameStyle`) through Chromium; each `--crop` adds <name>.<n>.png of that
// element. Evidence files belong outside the repo (coord/reports/<lane>/…): the
// app's rule is no checked-in PNGs.

import { readFileSync, writeFileSync, mkdirSync } from 'node:fs';
import { resolve, basename } from 'node:path';
import vm from 'node:vm';
import { JSDOM } from 'jsdom';

const args = process.argv.slice(2);
const file = args.find((a) => !a.startsWith('--') && !args[args.indexOf(a) - 1]?.startsWith('--'));
if (!file) {
  console.error('usage: render.mjs <mockup.dc.html> [--state json] [--out dir] [--name base] [--shot] [--crop sel]...');
  process.exit(2);
}
const opt = (flag, dflt) => {
  const i = args.indexOf(flag);
  return i >= 0 ? args[i + 1] : dflt;
};
const crops = args.flatMap((a, i) => (a === '--crop' ? [args[i + 1]] : []));
const state = JSON.parse(opt('--state', '{}'));
const outDir = resolve(opt('--out', './mockup-out'));
const name = opt('--name', `${basename(file, '.dc.html')}-${Object.values(state).join('-') || 'default'}`);
const shot = args.includes('--shot');

const source = readFileSync(file, 'utf8');
const dom = new JSDOM(source);
const { document } = dom.window;

// ── 1. Run the mockup's component for this state ──────────────────────────────
const script = document.querySelector('script[type="text/x-dc"]');
if (!script) throw new Error('no <script type="text/x-dc"> in the mockup');
const context = vm.createContext({
  DCLogic: class {
    constructor(st) {
      this.state = st;
    }
    setState() {
      /* static render: handlers never fire */
    }
  },
  Object,
  String,
  Array,
  Math,
  JSON,
});
const Component = vm.runInContext(`${script.textContent}\nComponent;`, context, { filename: basename(file) });
const vals = new Component(state).renderVals();
script.remove();

// ── 2. Expand the template ────────────────────────────────────────────────────
const lookup = (path, scopes) => {
  const [head, ...rest] = path.split('.');
  for (let i = scopes.length - 1; i >= 0; i--) {
    const s = scopes[i];
    if (s && typeof s === 'object' && head in s) {
      return rest.reduce((v, k) => (v == null ? v : v[k]), s[head]);
    }
  }
  if (head === 'true') return true;
  if (head === 'false') return false;
  return undefined;
};
const PLACEHOLDER = /\{\{\s*([\w.$]+)\s*\}\}/g;
const substitute = (text, scopes) =>
  text.replace(PLACEHOLDER, (_, p) => {
    const v = lookup(p, scopes);
    return v == null ? '' : String(v);
  });
const BOOL_PRESENCE = new Set(['checked', 'selected', 'disabled', 'hidden', 'open']);

function expandAttributes(el, scopes) {
  for (const attr of [...el.attributes]) {
    const { name: n, value } = attr;
    if (n.startsWith('hint-')) {
      el.removeAttribute(n);
      continue;
    }
    const whole = value.match(/^\{\{\s*([\w.$]+)\s*\}\}$/);
    if (whole) {
      const v = lookup(whole[1], scopes);
      if (typeof v === 'function') {
        el.removeAttribute(n);
        continue;
      }
      if (n === 'style') el.setAttribute('data-dc', whole[1]);
      if (BOOL_PRESENCE.has(n)) {
        if (v) el.setAttribute(n, '');
        else el.removeAttribute(n);
        continue;
      }
      if (n.startsWith('aria-') && typeof v === 'boolean') {
        el.setAttribute(n, v ? 'true' : 'false');
        continue;
      }
      el.setAttribute(n, v == null ? '' : String(v));
      continue;
    }
    if (n.startsWith('on')) {
      el.removeAttribute(n);
      continue;
    }
    el.setAttribute(n, substitute(value, scopes));
  }
}

function expand(node, scopes) {
  if (node.nodeType === 3) {
    node.textContent = substitute(node.textContent, scopes);
    return;
  }
  if (node.nodeType !== 1) return;
  const tag = node.tagName.toLowerCase();
  // A nested block replaces itself in place while its parent is still being
  // expanded, so a parent block expands its children first and only then moves
  // whatever is left under it — never the child node it started with.
  if (tag === 'sc-if') {
    const expr = node.getAttribute('value')?.match(/\{\{\s*([\w.$]+)\s*\}\}/)?.[1];
    const on = expr ? lookup(expr, scopes) : false;
    if (on) {
      for (const child of [...node.childNodes]) expand(child, scopes);
      node.replaceWith(...node.childNodes);
    } else {
      node.remove();
    }
    return;
  }
  if (tag === 'sc-for') {
    const expr = node.getAttribute('list')?.match(/\{\{\s*([\w.$]+)\s*\}\}/)?.[1];
    const as = node.getAttribute('as') || 'it';
    const list = (expr && lookup(expr, scopes)) || [];
    const frag = document.createDocumentFragment();
    for (const item of list) {
      const pass = node.cloneNode(true);
      for (const child of [...pass.childNodes]) expand(child, [...scopes, { [as]: item }]);
      frag.append(...pass.childNodes);
    }
    node.replaceWith(frag);
    return;
  }
  expandAttributes(node, scopes);
  for (const child of [...node.childNodes]) expand(child, scopes);
}

expand(document.body, [vals]);

// `<helmet>` carries the head; `<x-dc>` is the canvas wrapper.
const helmet = document.querySelector('helmet');
if (helmet) {
  for (const child of [...helmet.childNodes]) document.head.appendChild(child);
  helmet.remove();
}
for (const wrap of document.querySelectorAll('x-dc')) wrap.replaceWith(...wrap.childNodes);
const frame = document.querySelector('[data-dc="frameStyle"]');
if (frame) frame.id = 'frame';

mkdirSync(outDir, { recursive: true });
const htmlPath = resolve(outDir, `${name}.html`);
writeFileSync(htmlPath, `<!doctype html>\n${document.documentElement.outerHTML}`);
console.log(`rendered ${htmlPath}`);

// ── 3. Screenshots ────────────────────────────────────────────────────────────
if (shot) {
  const { chromium } = await import('@playwright/test');
  const browser = await chromium.launch();
  const page = await browser.newPage({ viewport: { width: 1600, height: 1400 }, deviceScaleFactor: 2 });
  await page.goto(`file://${htmlPath}`);
  await page.waitForLoadState('networkidle').catch(() => {});
  await page.evaluate(() => document.fonts?.ready);
  await page.locator('#frame').screenshot({ path: resolve(outDir, `${name}.png`) });
  console.log(`shot ${resolve(outDir, `${name}.png`)}`);
  let n = 0;
  for (const sel of crops) {
    n += 1;
    const loc = page.locator(sel).first();
    if ((await loc.count()) === 0) {
      console.log(`crop ${n}: no element for ${sel}`);
      continue;
    }
    await loc.screenshot({ path: resolve(outDir, `${name}.${n}.png`) });
    console.log(`crop ${n} (${sel}) → ${name}.${n}.png`);
  }
  await browser.close();
}
