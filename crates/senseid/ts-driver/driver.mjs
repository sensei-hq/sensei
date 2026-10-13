// sensei's TypeScript / JavaScript / Svelte compiler driver
// (docs/plan/2026-10-13-indexer-full-call-placement.md).
//
//   node driver.mjs <project_root> <engines_dir> <output.scip>
//   node --input-type=module -e "<this file>" <project_root> <engines_dir> <output.scip>
//
// <engines_dir> is where sensei-bootstrap installs the packages this loads when
// the project does not carry them.
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
import { spawnSync } from 'node:child_process';
import { createRequire } from 'node:module';
import fs from 'node:fs';
import path from 'node:path';

// The last three arguments: run as a file or as `-e` text, argv differs before them.
const [rootArg, enginesDir, outPath] = process.argv.length >= 4 ? process.argv.slice(-3) : [];
if (!rootArg || !enginesDir || !outPath) {
  console.error('usage: driver.mjs <project_root> <engines_dir> <output.scip>');
  process.exit(2);
}
const root = fs.realpathSync(rootArg);

/** A module from the project, else from sensei's engines directory. */
/** @param {string} name */
function load(name) {
  for (const base of [root, enginesDir]) {
    try {
      return createRequire(path.join(base, 'package.json'))(name);
    } catch {}
  }
  return null;
}
/** @type {typeof import('typescript')} */
const ts = load('typescript');
if (!ts) {
  console.error(`no typescript resolvable from ${root}`);
  process.exit(3);
}

const SKIP = new Set(['node_modules', '.svelte-kit', '.git', 'build', 'dist', 'target', '.turbo', 'coverage']);
/** @param {string} dir @param {string[]} [out] @returns {string[]} */
function walk(dir, out = []) {
  for (const entry of fs.readdirSync(dir, { withFileTypes: true })) {
    if (SKIP.has(entry.name)) continue;
    const full = path.join(dir, entry.name);
    if (entry.isDirectory()) walk(full, out);
    else out.push(full);
  }
  return out;
}

// ── a SvelteKit project, as its own tooling sees it ──────────────────────────
// Its tsconfig extends what `svelte-kit sync` generates — `$lib`, `$app/*`,
// `./$types` — which a clean checkout does not have. Run the project's own
// sync first (it writes only `.svelte-kit/`, the kit's own output); a sync that
// fails is the compile failing, not a project compiled without its paths.
const kitDir = (() => {
  try {
    return path.dirname(createRequire(path.join(root, 'package.json')).resolve('@sveltejs/kit/package.json'));
  } catch {
    return null;
  }
})();
const kitConfig = ['svelte.config.js', 'svelte.config.mjs', 'svelte.config.ts'].some((f) =>
  fs.existsSync(path.join(root, f))
);
if (kitDir && kitConfig) {
  const sync = spawnSync(process.execPath, [path.join(kitDir, 'svelte-kit.js'), 'sync'], {
    cwd: root,
    encoding: 'utf8',
  });
  if (sync.status !== 0) {
    console.error(`svelte-kit sync failed in ${root}: ${(sync.stderr || sync.error?.message || '').trim()}`);
    process.exit(4);
  }
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
/** @type {typeof import('svelte2tsx') | null} */
const s2t = load('svelte2tsx');
/** @type {typeof import('@jridgewell/trace-mapping') | null} */
const traceMapping = load('@jridgewell/trace-mapping');
/** @type {Map<string, { code: string, trace: import('@jridgewell/trace-mapping').TraceMap, original: string, lines: string[], scripts: number[][] }>} */
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
        // svelte2tsx states its map as magic-string's; it IS a v3 source map.
        trace: new traceMapping.TraceMap(/** @type {import('@jridgewell/trace-mapping').EncodedSourceMap} */ (out.map)),
        original: file,
        lines: text.split('\n'),
        scripts: scriptLines(text),
      });
    } catch (e) {
      console.error(`svelte2tsx could not read ${file}: ${e.message}`);
    }
  }
  const shims = createRequire(path.join(root, 'package.json'));
  for (const base of [root, enginesDir]) {
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
/** @type {Map<import('typescript').Node, number>} */
const locals = new Map();
let localCount = 0;

/** The package a library file belongs to; `@types/x` declares package `x`. */
/** @param {string} file */
function packageOf(file) {
  const at = file.lastIndexOf('/node_modules/');
  const rest = file.slice(at + '/node_modules/'.length).split('/');
  const name = rest[0].startsWith('@') ? `${rest[0]}/${rest[1]}` : rest[0];
  if (!name.startsWith('@types/')) return name;
  const typed = name.slice('@types/'.length);
  return typed.includes('__') ? `@${typed.replace('__', '/')}` : typed;
}

/** @param {import('typescript').SourceFile} sf */
function isLibraryFile(sf) {
  return program.isSourceFileDefaultLibrary(sf) || sf.fileName.includes('/node_modules/');
}

/** The 0-based line ranges of a component's `<script>` blocks. */
/** @param {string} text */
function scriptLines(text) {
  const ranges = [];
  for (const m of text.matchAll(/<script\b[^>]*>([\s\S]*?)<\/script>/g)) {
    const first = text.slice(0, m.index).split('\n').length - 1;
    ranges.push([first, first + m[0].split('\n').length - 1]);
  }
  return ranges;
}

/** Declared in a component's MARKUP — a snippet, an `{#each}` item, an
 *  `{@const}`: the template's own local. */
/** @param {import('typescript').Node} decl */
function isMarkupBinding(decl) {
  const declSf = decl.getSourceFile();
  const v = virtual.get(declSf.fileName);
  if (!v) return false;
  const at = originalPosition(declSf, /** @type {import('typescript').NamedDeclaration} */ (decl).name ?? decl);
  return !!at && !v.scripts.some(([a, b]) => a <= at.line && at.line <= b);
}

/** Inside a function body — a binding no other file can name. */
/** @param {import('typescript').Node} decl */
function isLocal(decl) {
  if (ts.isParameter(decl) || ts.isTypeParameterDeclaration(decl)) return true;
  // An index signature's entry is a value's, not a declaration.
  if (ts.isIndexSignatureDeclaration(decl)) return true;
  if (isPropsBinding(decl)) return true;
  if (isMarkupBinding(decl)) return true;
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

/** `let { a, b } = $props()` — a Svelte component's parameters. */
/** @param {import('typescript').Node} decl */
function isPropsBinding(decl) {
  if (!ts.isBindingElement(decl)) return false;
  /** @type {import('typescript').Node} */
  let p = decl.parent;
  while (p && (ts.isObjectBindingPattern(p) || ts.isArrayBindingPattern(p) || ts.isBindingElement(p))) p = p.parent;
  const init = p && ts.isVariableDeclaration(p) ? p.initializer : undefined;
  return !!init && ts.isCallExpression(init) && ts.isIdentifier(init.expression) && init.expression.text === '$props';
}

/** The property a name destructured out of a value stands for:
 *  `const { bind } = await import('./net')` is `./net`'s `bind`. */
/** @param {import('typescript').Node} decl */
function destructured(decl) {
  if (!ts.isBindingElement(decl) || !ts.isObjectBindingPattern(decl.parent)) return undefined;
  const owner = decl.parent.parent;
  if (!ts.isVariableDeclaration(owner) || !owner.initializer) return undefined;
  const key = decl.propertyName ?? decl.name;
  if (!ts.isIdentifier(key)) return undefined;
  const type = checker.getTypeAtLocation(owner.initializer);
  return checker.getPropertyOfType(checker.getAwaitedType(type) ?? type, key.text);
}

/** What an object property that only NAMES a value stands for:
 *  `{ loadCatalog }` and `{ load: loadCatalog }` are `loadCatalog`. */
/** @param {import('typescript').Node} decl */
function named(decl) {
  if (ts.isShorthandPropertyAssignment(decl)) return checker.getShorthandAssignmentValueSymbol(decl);
  if (ts.isPropertyAssignment(decl) && ts.isIdentifier(decl.initializer)) {
    return checker.getSymbolAtLocation(decl.initializer);
  }
  return undefined;
}

/** A symbol followed to what it stands for: through imports and re-exports,
 *  and through destructuring. */
/** @param {import('typescript').Symbol | undefined} sym */
function resolved(sym) {
  for (let hops = 0; sym && hops < 16; hops++) {
    if (sym.flags & ts.SymbolFlags.Alias) {
      try {
        sym = checker.getAliasedSymbol(sym);
        continue;
      } catch {
        return sym;
      }
    }
    const decl = sym.valueDeclaration ?? sym.declarations?.[0];
    const property = decl && !isPropsBinding(decl) ? destructured(decl) ?? named(decl) : undefined;
    if (!property || property === sym) return sym;
    sym = property;
  }
  return sym;
}

/** The class `super` reaches: what the enclosing class's `extends` names. */
/** @param {import('typescript').Node} node */
function baseClassOf(node) {
  let cls = node.parent;
  while (cls && !ts.isClassLike(cls)) cls = cls.parent;
  const extended = /** @type {import('typescript').ClassLikeDeclaration | undefined} */ (cls)?.heritageClauses?.find((h) => h.token === ts.SyntaxKind.ExtendsKeyword)?.types[0];
  return extended ? checker.getSymbolAtLocation(extended.expression) : undefined;
}

/** @param {import('typescript').Node} node */
function isComponentBody(node) {
  return (
    ts.isFunctionDeclaration(node) &&
    node.name?.text === '$$render' &&
    ts.isSourceFile(node.parent) &&
    virtual.has(node.parent.fileName)
  );
}

/** Where a node starts in its ORIGINAL file: 0-based line, UTF-8 byte column. */
/** @param {import('typescript').SourceFile} sf @param {import('typescript').Node} node */
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

/** @param {string} text @param {number} utf16Start @param {number} utf16Width */
function bytes(text, utf16Start, utf16Width) {
  const start = Buffer.byteLength(text.slice(0, utf16Start));
  const end = Buffer.byteLength(text.slice(0, utf16Start + utf16Width));
  return { start, end };
}

/** @param {import('typescript').Symbol} sym @param {import('typescript').NamedDeclaration} decl @param {import('typescript').SourceFile} [sf] @returns {string | null} */
function symbolFor(sym, decl, sf) {
  const declSf = decl.getSourceFile();
  if (isLibraryFile(declSf)) {
    const pkg = program.isSourceFileDefaultLibrary(declSf) ? 'ecmascript' : packageOf(declSf.fileName);
    const qualified = checker.getFullyQualifiedName(sym).replace(/^".*?"\./, '');
    return `npm ${pkg} ${qualified}`;
  }
  // `declare module 'x' { … }` in the project declares a LIBRARY's shape — a
  // virtual module, a shim for an untyped package — so what it declares is
  // that library's member (R5), named by its path within the module.
  const ambient = ambientModuleOf(decl);
  if (ambient) return `npm ${packageOfSpecifier(ambient.name.text)} ${pathWithin(decl, ambient)}`;
  if (isLocal(decl)) return localSymbol(decl);
  // A member no declaration of its own names — of an anonymous object type,
  // or a value in an object the walk does not make a container — is a VALUE:
  // reached through the value's named type when it has one, else a local's,
  // as a callback parameter is.
  if (isAnonymousMember(decl)) return typedValue(decl) ?? localSymbol(decl);
  const name = decl.name ?? decl;
  const at = originalPosition(declSf, name);
  if (!at) return null;
  // The name's VALUE, as the walk names a declaration: `'x-key'` is `x-key`.
  const spelled = ('text' in name ? name.text : undefined) ?? name.getText?.(declSf) ?? sym.getName();
  return `ts ${path.relative(root, at.file)}:${at.line + 1}:${spelled}`;
}

// ── the index ─────────────────────────────────────────────────────────────────
/** @type {Map<string, Array<{ range: number[], symbol: string, roles: number }>>} */
const documents = new Map(); // relative path -> occurrences

for (const sf of program.getSourceFiles()) {
  const v = virtual.get(sf.fileName);
  const original = v ? v.original : sf.fileName;
  if (!original.startsWith(`${root}/`) || original.includes('/node_modules/')) continue;
  if (!v && isLibraryFile(sf)) continue;
  const visit = (node) => {
    if (ts.isIdentifier(node) || ts.isPrivateIdentifier(node)) record(sf, node, checker.getSymbolAtLocation(node));
    // `o['x-key']` — a literal key names the property it reads.
    else if (ts.isStringLiteral(node) && ts.isElementAccessExpression(node.parent) && node.parent.argumentExpression === node)
      record(sf, node, checker.getSymbolAtLocation(node));
    // `super(…)` calls the base class: what the class's `extends` names.
    else if (node.kind === ts.SyntaxKind.SuperKeyword) record(sf, node, baseClassOf(node));
    ts.forEachChild(node, visit);
  };
  visit(sf);
}

/** @param {import('typescript').SourceFile} sf @param {import('typescript').Node} id @param {import('typescript').Symbol | undefined} found */
function record(sf, id, found) {
  const sym = resolved(found);
  const decl = sym && (sym.valueDeclaration ?? sym.declarations?.[0]);
  // A name imported from a module the checker cannot find stands for nothing
  // it can type — the receiver-less form of `untyped`.
  const unresolvedImport = !decl && !!found && (found.flags & ts.SymbolFlags.Alias) !== 0;
  const symbol = decl ? symbolFor(sym, decl, sf) : unresolvedImport ? 'untyped' : undeclared(id);
  if (!symbol) return;
  const at = originalPosition(sf, id);
  if (!at) return;
  const declaresIt = !!decl && /** @type {import('typescript').NamedDeclaration} */ (decl).name === id;
  const rel = path.relative(root, at.file);
  if (!documents.has(rel)) documents.set(rel, []);
  documents.get(rel).push({ range: [at.line, at.start, at.end], symbol, roles: declaresIt ? 1 : 0 });
}

/** @param {import('typescript').Node} decl */
function localSymbol(decl) {
  if (!locals.has(decl)) locals.set(decl, ++localCount);
  return `local ${locals.get(decl)}`;
}

/** The `declare module 'x'` a declaration sits in, if any. */
/** @param {import('typescript').Node} decl @returns {import('typescript').ModuleDeclaration | undefined} */
function ambientModuleOf(decl) {
  for (let p = decl.parent; p; p = p.parent) {
    if (ts.isModuleDeclaration(p) && ts.isStringLiteral(p.name)) return p;
  }
  return undefined;
}

/** The package a module specifier names: `@scope/name/…` or `name/…`. */
/** @param {string} spec */
function packageOfSpecifier(spec) {
  const parts = spec.split('/');
  return spec.startsWith('@') ? parts.slice(0, 2).join('/') : parts[0];
}

/** A declaration's name path within `outer`: `kit.configure`, `Item.get`. */
/** @param {import('typescript').Node} decl @param {import('typescript').Node} outer */
function pathWithin(decl, outer) {
  const names = [];
  for (let n = decl; n && n !== outer; n = n.parent) {
    const name = /** @type {import('typescript').NamedDeclaration} */ (n).name;
    if (name && (ts.isIdentifier(name) || ts.isStringLiteral(name) || ts.isPrivateIdentifier(name))) {
      names.unshift(name.text);
    }
  }
  return names.join('.');
}

/** A member of an object type or literal that no declaration of its own
 *  names: one of an anonymous type literal, or an object member the walk does
 *  not mint (only the functions of a module-level object are its members). */
/** @param {import('typescript').Node} decl */
function isAnonymousMember(decl) {
  if (ts.isPropertySignature(decl) || ts.isMethodSignature(decl)) {
    return ts.isTypeLiteralNode(decl.parent) && !ts.isTypeAliasDeclaration(decl.parent.parent);
  }
  const inObject =
    ts.isPropertyAssignment(decl) ||
    ts.isShorthandPropertyAssignment(decl) ||
    ((ts.isMethodDeclaration(decl) || ts.isGetAccessor(decl) || ts.isSetAccessor(decl)) &&
      ts.isObjectLiteralExpression(decl.parent));
  return inObject && !isWalkedObjectMember(decl);
}

/** The walk's own rule: a function under a key of an object bound to a name
 *  at the top of a module is that object's member. */
/** @param {import('typescript').Node} decl */
function isWalkedObjectMember(decl) {
  const value = ts.isPropertyAssignment(decl) ? decl.initializer : decl;
  const fn =
    ts.isMethodDeclaration(value) ||
    ts.isGetAccessor(value) ||
    ts.isSetAccessor(value) ||
    ts.isFunctionExpression(value) ||
    ts.isArrowFunction(value);
  if (!fn) return false;
  let owner = decl.parent.parent;
  while (owner && (ts.isParenthesizedExpression(owner) || ts.isAsExpression(owner) || ts.isSatisfiesExpression(owner))) {
    owner = owner.parent;
  }
  return !!owner && ts.isVariableDeclaration(owner) && ts.isIdentifier(owner.name) && !isLocal(owner);
}

/** A computed value's named type, as a symbol: `{ publish: fn() }` is called
 *  through what `fn()` returns. */
/** @param {import('typescript').Node} decl */
function typedValue(decl) {
  if (!ts.isPropertyAssignment(decl)) return null;
  const type = checker.getTypeAtLocation(decl.initializer);
  const owner = type.aliasSymbol ?? type.symbol;
  const ownerDecl = owner?.declarations?.[0];
  if (!ownerDecl || /** @type {import('typescript').Node} */ (ownerDecl) === decl.initializer) return null;
  const named =
    ts.isClassDeclaration(ownerDecl) ||
    ts.isInterfaceDeclaration(ownerDecl) ||
    ts.isTypeAliasDeclaration(ownerDecl) ||
    ts.isFunctionDeclaration(ownerDecl);
  return named ? symbolFor(owner, ownerDecl) : null;
}

/** A member the checker knows without a declaration of its own: one of a
 *  mapped type (`fireEvent.click`) is named by the type; one of a receiver
 *  typed `any` is marked `untyped` — nothing static names it. */
/** @param {import('typescript').Node} id */
function undeclared(id) {
  const access = id.parent;
  if (!access || !ts.isPropertyAccessExpression(access) || access.name !== id) return null;
  const type = checker.getTypeAtLocation(access.expression);
  if (type.flags & (ts.TypeFlags.Any | ts.TypeFlags.Unknown)) return libraryPath(access) ?? 'untyped';
  // No such property, only an index signature: an entry of the value
  // (`props.low_n` on `Record<string, unknown>`), not a member of its type.
  const declares = checker.getPropertyOfType(type, access.name.text);
  const indexed = checker.getIndexInfosOfType(type).length > 0;
  if (!declares && indexed) return localSymbol(id);
  // An intersection or a union has no name of its own: the member is named
  // by the part that has it (`FireFunction & FireObject` → `FireObject`).
  const parts = type.isUnionOrIntersection() ? type.types : [type];
  const holder = parts.find((t) => checker.getPropertyOfType(t, access.name.text)) ?? type;
  const owner = holder.aliasSymbol ?? holder.symbol;
  const ownerDecl = owner?.declarations?.[0];
  if (!ownerDecl) return null;
  const file = ownerDecl.getSourceFile();
  if (isLibraryFile(file)) {
    const pkg = program.isSourceFileDefaultLibrary(file) ? 'ecmascript' : packageOf(file.fileName);
    return `npm ${pkg} ${checker.getFullyQualifiedName(owner).replace(/^".*?"\./, '')}.${access.name.text}`;
  }
  return symbolFor(owner, ownerDecl);
}

/** A member reached by a property path off a binding imported from a library
 *  — `commands.registerMany`, where `commands` comes from a package typed
 *  `any` — is that library's member, named by its path (R5): naming an
 *  external needs no types. */
/** @param {import('typescript').PropertyAccessExpression} access */
function libraryPath(access) {
  const names = [access.name.text];
  let e = access.expression;
  while (ts.isPropertyAccessExpression(e)) {
    names.unshift(e.name.text);
    e = e.expression;
  }
  if (!ts.isIdentifier(e)) return null;
  let root = checker.getSymbolAtLocation(e);
  if (!root || !(root.flags & ts.SymbolFlags.Alias)) return null;
  try {
    root = checker.getAliasedSymbol(root);
  } catch {
    return null;
  }
  const decl = root.valueDeclaration ?? root.declarations?.[0];
  if (!decl) return null;
  const file = decl.getSourceFile();
  if (isLibraryFile(file)) {
    const pkg = program.isSourceFileDefaultLibrary(file) ? 'ecmascript' : packageOf(file.fileName);
    return `npm ${pkg} ${[checker.getFullyQualifiedName(root).replace(/^".*?"\./, ''), ...names].join('.')}`;
  }
  const ambient = ambientModuleOf(decl);
  if (!ambient) return null;
  return `npm ${packageOfSpecifier(ambient.name.text)} ${[pathWithin(decl, ambient), ...names].join('.')}`;
}

// ── SCIP, by hand: Index.documents = 2; Document.relative_path = 1,
//    occurrences = 2; Occurrence.range = 1 (packed int32), symbol = 2,
//    symbol_roles = 3. ───────────────────────────────────────────────────────────
/** @param {number} n */
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
/** @param {number} n @param {number} wire */
const field = (n, wire) => varint((n << 3) | wire);
/** @param {number} n @param {number[]} bytes */
function lengthDelimited(n, bytes) {
  return [...field(n, 2), ...varint(bytes.length), ...bytes];
}
/** @param {string} s */
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
