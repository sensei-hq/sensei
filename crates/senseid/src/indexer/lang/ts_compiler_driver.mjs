// sensei's TypeScript / JavaScript / Svelte compiler driver
// (docs/plan/2026-10-13-indexer-full-call-placement.md).
//
//   node ts_compiler_driver.mjs <project_root> <output.scip>
//   node --input-type=module -e "<this file>" <project_root> <output.scip>
//
// Uses the PROJECT'S OWN `typescript` (so the compiler is the one the project
// builds with), reads its tsconfig with JS allowed, turns every `.svelte`
// component into TypeScript through svelte2tsx (positions mapped back through
// its source map), and asks the type checker what every identifier refers to.
// Writes a SCIP index — the format rust-analyzer writes — so the Rust side has
// one reader.
//
// Symbols:
//   local <n>                    a binding inside a body — never a node
//   ts <file>:<line>:<name>      declared in this project; a definition
//                                occurrence marks where (roles = 1)
//   npm <package> <path>         a library: node_modules, Svelte's runes, and
//                                the language's own declarations as package
//                                `ecmascript` (the walk's name for them)
import { createRequire } from 'node:module';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';

// The last two arguments: run as a file or as `-e` text, argv differs before them.
const [rootArg, outPath] = process.argv.length >= 3 ? process.argv.slice(-2) : [];
if (!rootArg || !outPath) {
  console.error('usage: ts_compiler_driver.mjs <project_root> <output.scip>');
  process.exit(2);
}
const root = fs.realpathSync(rootArg);

/** A module from the project, else from sensei's engines directory. */
function load(name) {
  for (const base of [root, path.join(os.homedir(), '.sensei', 'engines')]) {
    try {
      return createRequire(path.join(base, 'package.json'))(name);
    } catch {}
  }
  return null;
}
const ts = load('typescript');
if (!ts) {
  console.error(`no typescript resolvable from ${root}`);
  process.exit(3);
}

const SKIP = new Set(['node_modules', '.svelte-kit', '.git', 'build', 'dist', 'target', '.turbo', 'coverage']);
function walk(dir, out = []) {
  for (const entry of fs.readdirSync(dir, { withFileTypes: true })) {
    if (SKIP.has(entry.name)) continue;
    const full = path.join(dir, entry.name);
    if (entry.isDirectory()) walk(full, out);
    else out.push(full);
  }
  return out;
}

// ── the program ───────────────────────────────────────────────────────────────
const configPath = ts.findConfigFile(root, ts.sys.fileExists, 'tsconfig.json');
let options = { allowJs: true, noEmit: true };
let rootNames = [];
if (configPath) {
  const read = ts.readConfigFile(configPath, ts.sys.readFile);
  const parsed = ts.parseJsonConfigFileContent(read.config, ts.sys, path.dirname(configPath), {}, configPath);
  options = { ...parsed.options, allowJs: true, noEmit: true };
  rootNames = parsed.fileNames;
}
const sources = walk(root);
const scripts = sources.filter((f) => /\.(m|c)?[jt]sx?$/.test(f) && !f.endsWith('.d.ts'));
rootNames = [...new Set([...rootNames, ...scripts])];

// Svelte components, as TypeScript, with a map back to the component.
const s2t = load('svelte2tsx');
const traceMapping = load('@jridgewell/trace-mapping');
const virtual = new Map(); // `<file>.svelte.ts` -> { code, trace, original, lines }
const componentFiles = sources.filter((f) => f.endsWith('.svelte'));
if (componentFiles.length && (!s2t || !traceMapping)) {
  console.error('svelte2tsx / @jridgewell/trace-mapping not installed — .svelte files are not compiled');
}
if (s2t && traceMapping) {
  for (const file of componentFiles) {
    const text = fs.readFileSync(file, 'utf8');
    try {
      const out = s2t.svelte2tsx(text, { filename: file, isTsFile: true, mode: 'ts' });
      virtual.set(`${file}.ts`, {
        code: out.code,
        trace: new traceMapping.TraceMap(out.map),
        original: file,
        lines: text.split('\n'),
      });
    } catch (e) {
      console.error(`svelte2tsx could not read ${file}: ${e.message}`);
    }
  }
  const shims = createRequire(path.join(root, 'package.json'));
  for (const base of [root, path.join(os.homedir(), '.sensei', 'engines')]) {
    try {
      const dir = path.dirname(createRequire(path.join(base, 'package.json')).resolve('svelte2tsx/package.json'));
      for (const shim of ['svelte-shims-v4.d.ts', 'svelte-jsx-v4.d.ts']) {
        const p = path.join(dir, shim);
        if (fs.existsSync(p)) rootNames.push(p);
      }
      break;
    } catch {}
  }
  void shims;
}
rootNames.push(...virtual.keys());

const host = ts.createCompilerHost(options, true);
const { fileExists, readFile, getSourceFile } = host;
host.fileExists = (f) => virtual.has(f) || fileExists.call(host, f);
host.readFile = (f) => (virtual.has(f) ? virtual.get(f).code : readFile.call(host, f));
host.getSourceFile = (f, lang, onError, create) =>
  virtual.has(f)
    ? ts.createSourceFile(f, virtual.get(f).code, lang, true, ts.ScriptKind.TS)
    : getSourceFile.call(host, f, lang, onError, create);
host.resolveModuleNameLiterals = (literals, containing, redirected, opts) =>
  literals.map((literal) => {
    const spec = literal.text;
    if (spec.endsWith('.svelte')) {
      const base = containing.endsWith('.svelte.ts') ? path.dirname(containing) : path.dirname(containing);
      const candidate = `${path.resolve(base, spec)}.ts`;
      if (virtual.has(candidate)) {
        return { resolvedModule: { resolvedFileName: candidate, extension: ts.Extension.Ts, isExternalLibraryImport: false } };
      }
    }
    return ts.resolveModuleName(spec, containing, opts, host, undefined, redirected);
  });

const program = ts.createProgram({ rootNames, options, host });
const checker = program.getTypeChecker();

// ── naming ────────────────────────────────────────────────────────────────────
const locals = new Map();
let localCount = 0;

/** The package a library file belongs to; `@types/x` declares package `x`. */
function packageOf(file) {
  const at = file.lastIndexOf('/node_modules/');
  const rest = file.slice(at + '/node_modules/'.length).split('/');
  const name = rest[0].startsWith('@') ? `${rest[0]}/${rest[1]}` : rest[0];
  if (!name.startsWith('@types/')) return name;
  const typed = name.slice('@types/'.length);
  return typed.includes('__') ? `@${typed.replace('__', '/')}` : typed;
}

function isLibraryFile(sf) {
  return program.isSourceFileDefaultLibrary(sf) || sf.fileName.includes('/node_modules/');
}

/** Inside a function body — a binding no other file can name. */
function isLocal(decl) {
  if (ts.isParameter(decl) || ts.isTypeParameterDeclaration(decl)) return true;
  for (let p = decl.parent; p; p = p.parent) {
    if (ts.isSourceFile(p) || ts.isModuleBlock(p)) return false;
    if (ts.isClassLike(p) || ts.isInterfaceDeclaration(p)) return false;
    // svelte2tsx wraps a component's script in `$$render`: its body is the
    // component's top level, not a function's.
    if (isComponentBody(p)) return false;
    if (ts.isFunctionLike(p)) return true;
  }
  return false;
}

function isComponentBody(node) {
  return (
    ts.isFunctionDeclaration(node) &&
    node.name?.text === '$$render' &&
    ts.isSourceFile(node.parent) &&
    virtual.has(node.parent.fileName)
  );
}

/** Where a node starts in its ORIGINAL file: 0-based line, UTF-8 byte column. */
function originalPosition(sf, node) {
  const start = node.getStart(sf);
  const width = node.getEnd() - start;
  const lc = sf.getLineAndCharacterOfPosition(start);
  const v = virtual.get(sf.fileName);
  if (!v) {
    const text = sf.text.split('\n')[lc.line] ?? '';
    return { file: sf.fileName, line: lc.line, ...bytes(text, lc.character, width) };
  }
  const at = traceMapping.originalPositionFor(v.trace, { line: lc.line + 1, column: lc.character });
  if (at.line == null) return null;
  const text = v.lines[at.line - 1] ?? '';
  // svelte2tsx's own scaffolding maps near, not onto, what the component
  // wrote: a position is the component's only where it spells the same name.
  if (text.slice(at.column, at.column + width) !== node.getText(sf)) return null;
  return { file: v.original, line: at.line - 1, ...bytes(text, at.column, width) };
}

function bytes(text, utf16Start, utf16Width) {
  const start = Buffer.byteLength(text.slice(0, utf16Start));
  const end = Buffer.byteLength(text.slice(0, utf16Start + utf16Width));
  return { start, end };
}

function symbolFor(sym, decl, sf) {
  const declSf = decl.getSourceFile();
  if (isLibraryFile(declSf)) {
    const pkg = program.isSourceFileDefaultLibrary(declSf) ? 'ecmascript' : packageOf(declSf.fileName);
    const qualified = checker.getFullyQualifiedName(sym).replace(/^".*?"\./, '');
    return `npm ${pkg} ${qualified}`;
  }
  if (isLocal(decl)) {
    if (!locals.has(decl)) locals.set(decl, ++localCount);
    return `local ${locals.get(decl)}`;
  }
  const name = decl.name ?? decl;
  const at = originalPosition(declSf, name);
  if (!at) return null;
  return `ts ${path.relative(root, at.file)}:${at.line + 1}:${name.getText?.(declSf) ?? sym.getName()}`;
}

// ── the index ─────────────────────────────────────────────────────────────────
const documents = new Map(); // relative path -> occurrences

for (const sf of program.getSourceFiles()) {
  const v = virtual.get(sf.fileName);
  const original = v ? v.original : sf.fileName;
  if (!original.startsWith(`${root}/`) || original.includes('/node_modules/')) continue;
  if (!v && isLibraryFile(sf)) continue;
  const visit = (node) => {
    if (ts.isIdentifier(node) || ts.isPrivateIdentifier(node)) record(sf, node);
    ts.forEachChild(node, visit);
  };
  visit(sf);
}

function record(sf, id) {
  let sym = checker.getSymbolAtLocation(id);
  if (!sym) return;
  if (sym.flags & ts.SymbolFlags.Alias) {
    try {
      sym = checker.getAliasedSymbol(sym);
    } catch {}
  }
  const decl = sym.valueDeclaration ?? sym.declarations?.[0];
  if (!decl) return;
  const at = originalPosition(sf, id);
  if (!at) return;
  const symbol = symbolFor(sym, decl, sf);
  if (!symbol) return;
  const declaresIt = decl.name === id;
  const rel = path.relative(root, at.file);
  if (!documents.has(rel)) documents.set(rel, []);
  documents.get(rel).push({ range: [at.line, at.start, at.end], symbol, roles: declaresIt ? 1 : 0 });
}

// ── SCIP, by hand: Index.documents = 2; Document.relative_path = 1,
//    occurrences = 2; Occurrence.range = 1 (packed int32), symbol = 2,
//    symbol_roles = 3. ───────────────────────────────────────────────────────────
function varint(n) {
  const out = [];
  let v = n >>> 0;
  while (v > 127) {
    out.push((v & 127) | 128);
    v >>>= 7;
  }
  out.push(v);
  return out;
}
const field = (n, wire) => varint((n << 3) | wire);
function lengthDelimited(n, bytes) {
  return [...field(n, 2), ...varint(bytes.length), ...bytes];
}
const utf8 = (s) => [...Buffer.from(s, 'utf8')];

const chunks = [];
for (const [rel, occurrences] of documents) {
  const doc = [...lengthDelimited(1, utf8(rel))];
  for (const o of occurrences) {
    const range = o.range.flatMap((v) => varint(v));
    const occ = [...lengthDelimited(1, range), ...lengthDelimited(2, utf8(o.symbol))];
    if (o.roles) occ.push(...field(3, 0), ...varint(o.roles));
    doc.push(...lengthDelimited(2, occ));
  }
  chunks.push(Buffer.from(lengthDelimited(2, doc)));
}
fs.writeFileSync(outPath, Buffer.concat(chunks));
console.error(`indexed ${documents.size} files, ${virtual.size} svelte components`);
