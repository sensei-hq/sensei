// Inline-style → Tailwind codemod. Loaded by run_script via readFile + eval;
// the last expression is the exported `convert(src)` function.
//
// Conservative by design:
//   · only exact literal values (a token var, a keyword, 0/1) are moved
//   · anything dynamic (template literal, ternary, variable) stays inline
//   · an element whose className is an expression is skipped entirely
//   · a property with no faithful utility stays inline
// So a converted file renders identically; only authorship changes.

(function () {
  const SP = (v) => { const m = /^var\(--space-(\d+)\)$/.exec(v); return m ? m[1] : null; };
  const KEYWORD = {
    display: { flex: 'flex', grid: 'grid', block: 'block', 'inline-flex': 'inline-flex', 'inline-block': 'inline-block', none: 'hidden', contents: 'contents' },
    flexDirection: { row: 'flex-row', column: 'flex-col', 'row-reverse': 'flex-row-reverse', 'column-reverse': 'flex-col-reverse' },
    alignItems: { center: 'items-center', 'flex-start': 'items-start', 'flex-end': 'items-end', baseline: 'items-baseline', stretch: 'items-stretch', start: 'items-start', end: 'items-end' },
    alignSelf: { center: 'self-center', 'flex-start': 'self-start', 'flex-end': 'self-end', stretch: 'self-stretch' },
    justifyContent: { center: 'justify-center', 'flex-start': 'justify-start', 'flex-end': 'justify-end', 'space-between': 'justify-between', 'space-around': 'justify-around', 'space-evenly': 'justify-evenly' },
    flexWrap: { wrap: 'flex-wrap', nowrap: 'flex-nowrap', 'wrap-reverse': 'flex-wrap-reverse' },
    overflow: { hidden: 'overflow-hidden', auto: 'overflow-auto', visible: 'overflow-visible', scroll: 'overflow-scroll' },
    overflowX: { hidden: 'overflow-x-hidden', auto: 'overflow-x-auto' },
    overflowY: { hidden: 'overflow-y-hidden', auto: 'overflow-y-auto' },
    position: { relative: 'relative', absolute: 'absolute', sticky: 'sticky', fixed: 'fixed', static: 'static' },
    textAlign: { left: 'text-left', center: 'text-center', right: 'text-right' },
    textTransform: { uppercase: 'uppercase', lowercase: 'lowercase', capitalize: 'capitalize', none: 'normal-case' },
    textDecoration: { none: 'no-underline', underline: 'underline' },
    whiteSpace: { nowrap: 'whitespace-nowrap', normal: 'whitespace-normal', pre: 'whitespace-pre' },
    textOverflow: { ellipsis: 'text-ellipsis', clip: 'text-clip' },
    cursor: { pointer: 'cursor-pointer', default: 'cursor-default', 'not-allowed': 'cursor-not-allowed' },
    fontStyle: { italic: 'italic', normal: 'not-italic' },
    boxSizing: {}, // never convert
  };
  const COLOR = ['paper', 'paper-soft', 'paper-mute', 'paper-edge', 'paper-2', 'paper-3',
    'ink', 'ink-soft', 'ink-mute', 'ink-faint', 'ink-2', 'ink-3', 'ink-4',
    'accent', 'accent-soft', 'accent-edge', 'success', 'success-soft', 'success-edge',
    'warning', 'warning-soft', 'warning-edge', 'danger', 'danger-soft', 'danger-edge',
    'primary', 'on-primary', 'on-primary-soft', 'on-primary-mute', 'on-primary-faint'];
  const tokenColor = (v) => { const m = /^var\(--([a-z0-9-]+)\)$/.exec(v); return m && COLOR.includes(m[1]) ? m[1] : null; };

  // property → class(es), or null to leave inline
  function classesFor(key, raw) {
    let v = raw.trim();
    const quoted = /^(['"])([\s\S]*)\1$/.exec(v);
    const isStr = !!quoted;
    if (isStr) v = quoted[2].trim();
    // bail on anything not a plain literal
    if (!isStr && !/^-?[\d.]+$/.test(v)) return null;

    if (KEYWORD[key]) return isStr ? (KEYWORD[key][v] ? [KEYWORD[key][v]] : null) : null;

    switch (key) {
      case 'flex':       return v === '1' ? ['flex-1'] : v === 'none' ? ['flex-none'] : null;
      case 'flexShrink': return v === '0' ? ['shrink-0'] : v === '1' ? ['shrink'] : null;
      case 'flexGrow':   return v === '0' ? ['grow-0'] : v === '1' ? ['grow'] : null;
      case 'minWidth':   return v === '0' ? ['min-w-0'] : v === '100%' ? ['min-w-full'] : null;
      case 'minHeight':  return v === '0' ? ['min-h-0'] : v === '100%' ? ['min-h-full'] : null;
      case 'width':      return v === '100%' ? ['w-full'] : v === 'auto' ? ['w-auto'] : v === 'fit-content' ? ['w-fit'] : null;
      case 'height':     return v === '100%' ? ['h-full'] : v === 'auto' ? ['h-auto'] : v === 'fit-content' ? ['h-fit'] : null;
      case 'maxWidth':   return v === '100%' ? ['max-w-full'] : null;
      case 'fontWeight': return ({ '300': ['font-light'], '400': ['font-normal'], '500': ['font-medium'], '600': ['font-semibold'] })[v] || null;
      case 'fontSize': { const m = /^var\(--text-([a-z0-9]+)\)$/.exec(v); return m ? ['text-' + m[1]] : null; }
      case 'lineHeight': { const m = /^var\(--leading-([a-z]+)\)$/.exec(v); return m ? ['leading-' + m[1]] : null; }
      case 'letterSpacing': { const m = /^var\(--tracking-([a-z]+)\)$/.exec(v); return m ? ['tracking-' + m[1]] : null; }
      case 'borderRadius': {
        const m = /^var\(--radius(?:-([a-z]+))?\)$/.exec(v);
        if (m) return ['rounded' + (m[1] ? '-' + m[1] : '')];
        return v === '50%' || v === '9999px' ? ['rounded-full'] : null;
      }
      case 'boxShadow': { const m = /^var\(--shadow(?:-([a-z]+))?\)$/.exec(v); return m ? ['shadow' + (m[1] ? '-' + m[1] : '')] : v === 'none' ? ['shadow-none'] : null; }
      case 'background':
      case 'backgroundColor': {
        const c = tokenColor(v); if (c) return ['bg-' + c];
        return v === 'transparent' ? ['bg-transparent'] : null;
      }
      case 'color': {
        const c = tokenColor(v); if (c) return ['text-' + c];
        return v === 'inherit' ? ['text-inherit'] : null;
      }
      case 'border':
        return v === 'var(--hairline)' ? ['border-1px', 'border-paper-edge'] : v === 'none' ? ['border-0'] : null;
      case 'borderTop':    return v === 'var(--hairline)' ? ['border-t'] : v === 'none' ? ['border-t-0'] : null;
      case 'borderBottom': return v === 'var(--hairline)' ? ['border-b'] : v === 'none' ? ['border-b-0'] : null;
      case 'borderLeft':   return v === 'var(--hairline)' ? ['border-l'] : v === 'none' ? ['border-l-0'] : null;
      case 'borderRight':  return v === 'var(--hairline)' ? ['border-r'] : v === 'none' ? ['border-r-0'] : null;
      case 'borderColor': { const c = tokenColor(v); return c ? ['border-' + c] : null; }
    }

    // spacing families — one or two token values ("var(--space-3) var(--space-4)")
    const box = { padding: 'p', margin: 'm' };
    const side = { paddingTop: 'pt', paddingBottom: 'pb', paddingLeft: 'pl', paddingRight: 'pr',
                   marginTop: 'mt', marginBottom: 'mb', marginLeft: 'ml', marginRight: 'mr' };
    const gaps = { gap: 'gap', rowGap: 'gap-y', columnGap: 'gap-x' };
    if (gaps[key] || side[key] || box[key]) {
      if (v === '0') return [(gaps[key] || side[key] || box[key]) + '-0'];
      const parts = v.split(/\s+/);
      if (side[key] || gaps[key]) {
        if (parts.length !== 1) return null;
        const n = SP(parts[0]); return n ? [(side[key] || gaps[key]) + '-' + n] : null;
      }
      // padding / margin shorthand
      if (key === 'margin' && v === '0 auto') return ['mx-auto'];
      if (parts.length === 1) { const n = SP(parts[0]); return n ? [box[key] + '-' + n] : null; }
      if (parts.length === 2) {
        const a = parts[0] === '0' ? '0' : SP(parts[0]), b = parts[1] === '0' ? '0' : SP(parts[1]);
        if (a == null || b == null) return null;
        return [box[key] + 'y-' + a, box[key] + 'x-' + b];
      }
      return null;
    }
    return null;
  }

  // ── walk style={{ … }} blocks, rewrite in place ──────────────────
  function splitTopLevel(body) {
    const parts = []; let d = 0, q = null, cur = '';
    for (const c of body) {
      if (q) { cur += c; if (c === q) q = null; continue; }
      if (c === '"' || c === "'" || c === '`') { q = c; cur += c; continue; }
      if ('{[('.includes(c)) d++;
      if ('}])'.includes(c)) d--;
      if (c === ',' && d === 0) { parts.push(cur); cur = ''; continue; }
      cur += c;
    }
    if (cur.trim()) parts.push(cur);
    return parts;
  }

  return function convert(src) {
    let moved = 0, skipped = 0, out = src, i = 0;
    while ((i = out.indexOf('style={{', i)) !== -1) {
      // out[i+6] and out[i+7] are the two opening braces; walk the inner one.
      let d = 0, j = i + 7;
      for (; j < out.length; j++) {
        const c = out[j];
        if (c === '{') d++;
        else if (c === '}') { d--; if (d === 0) break; }
      }
      if (out[j + 1] !== '}') { i = j + 1; continue; }   // not an object-literal attr
      const attrEnd = j + 2;                              // index after "}}"
      const body = out.slice(i + 8, j);

      // locate the owning tag: nearest "<Name" before, and its closing ">"
      const tagStart = (() => {
        for (let k = i; k >= 0; k--) if (out[k] === '<' && /[A-Za-z]/.test(out[k + 1] || '')) return k;
        return -1;
      })();
      if (tagStart < 0) { i = attrEnd; continue; }
      // ONLY intrinsic DOM elements. A custom component (<Card>, <K2PlanBar>,
      // <window.Foo>) may forward `style` but not `className`, in which case a
      // moved property would vanish silently — as K2PlanBar's flex:1 did.
      const tagName = /^<([A-Za-z][\w.-]*)/.exec(out.slice(tagStart, tagStart + 40));
      if (!tagName || !/^[a-z][a-z0-9-]*$/.test(tagName[1])) { skipped++; i = attrEnd; continue; }
      const tagEnd = (() => {
        let dd = 0, q = null;
        for (let k = attrEnd; k < out.length; k++) {
          const c = out[k];
          if (q) { if (c === q) q = null; continue; }
          if (c === '"' || c === "'" || c === '`') { q = c; continue; }
          if (c === '{') dd++;
          else if (c === '}') dd--;
          else if (c === '>' && dd === 0) return k;
        }
        return -1;
      })();
      if (tagEnd < 0) { i = attrEnd; continue; }

      // className anywhere in the tag must be absent or a plain string literal
      const tagText = out.slice(tagStart, tagEnd);
      if (/className=\{/.test(tagText)) { skipped++; i = attrEnd; continue; }
      const hasCn = /className=(["'])([\s\S]*?)\1/.test(tagText);

      const keep = [], add = [];
      for (const part of splitTopLevel(body)) {
        const m = /^\s*([A-Za-z][A-Za-z0-9]*)\s*:\s*([\s\S]+?)\s*$/.exec(part);
        if (!m) { keep.push(part); continue; }
        const cls = classesFor(m[1], m[2]);
        if (cls) { add.push(...cls); moved++; } else keep.push(part);
      }
      if (!add.length) { i = attrEnd; continue; }

      const newStyle = keep.length ? 'style={{' + keep.join(',') + ' }}' : '';
      // rebuild the whole tag in one go so offsets stay sane
      let tag = out.slice(tagStart, i) + newStyle + out.slice(attrEnd, tagEnd);
      if (hasCn) {
        tag = tag.replace(/className=(["'])([\s\S]*?)\1/, (s, q2, val) =>
          'className=' + q2 + (val.trim() ? val.trim() + ' ' : '') + add.join(' ') + q2);
      } else {
        const nameEnd = /^<[A-Za-z][\w.]*/.exec(tag)[0].length;
        tag = tag.slice(0, nameEnd) + ' className="' + add.join(' ') + '"' + tag.slice(nameEnd);
      }
      tag = tag.replace(/\s+style=\{\{\s*\}\}/g, '').replace(/[ \t]{2,}/g, ' ');
      out = out.slice(0, tagStart) + tag + out.slice(tagEnd);
      i = tagStart + tag.length;
    }
    return { src: out, moved, skipped };
  };
})
