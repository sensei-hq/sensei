import { readFile } from "node:fs/promises";
import type { FileEntry, ParsedFile, ParsedSymbol, ParsedEdge, ParsedImport } from "@sensei/shared";

// Access modifiers that Swift supports
const ACCESS_MODIFIERS = ["public", "open", "internal", "private", "fileprivate"];
const ACCESS_RE = new RegExp(`(?:(${ACCESS_MODIFIERS.join("|")})\\s+)?`, "i");

// Helper: build a pattern that optionally matches access modifier + optional other keywords
function buildRe(keyword: string, extras = ""): RegExp {
  // e.g. /^[ \t]*(?:(public|open|...) )?(?:static |final |override |...)?func NAME/
  return new RegExp(
    `^[ \\t]*${ACCESS_RE.source}(?:(?:static|final|override|class|required|convenience|mutating|nonmutating|weak|lazy|dynamic)\\s+)*${extras}${keyword}\\s+([A-Za-z_][A-Za-z0-9_<>,\\s]*)`,
    "i",
  );
}

const PATTERNS: Array<{ re: RegExp; kind: ParsedSymbol["kind"] }> = [
  { re: buildRe("func"),      kind: "function" },
  { re: buildRe("class"),     kind: "class" },
  { re: buildRe("struct"),    kind: "class" },
  { re: buildRe("protocol"),  kind: "interface" },
  { re: buildRe("enum"),      kind: "enum" },
  { re: buildRe("typealias"), kind: "type" },
  { re: buildRe("extension"), kind: "class" },
];

// Module-level let/var
const LET_VAR_RE = /^[ \t]*(?:(public|open|internal|private|fileprivate)\s+)?(?:static\s+)?(let|var)\s+([A-Za-z_][A-Za-z0-9_]*)/i;

// Import statement
const IMPORT_RE = /^[ \t]*import\s+([A-Za-z_][A-Za-z0-9_.]*)/;

// Call expression: name(  but not keywords
const CALL_RE = /\b([A-Za-z_][A-Za-z0-9_]*)\s*\(/g;

const SWIFT_KEYWORDS = new Set([
  "if", "else", "for", "while", "do", "switch", "case", "default", "guard",
  "return", "throw", "try", "catch", "defer", "in", "where", "let", "var",
  "func", "class", "struct", "enum", "protocol", "extension", "typealias",
  "import", "init", "deinit", "super", "self", "Self", "true", "false", "nil",
  "public", "open", "internal", "private", "fileprivate", "static", "final",
  "override", "mutating", "nonmutating", "required", "convenience", "weak",
  "lazy", "dynamic", "print", "fatalError", "precondition", "assert",
]);

export class SwiftAdapter {
  readonly extensions = [".swift"];

  async parse(file: FileEntry): Promise<ParsedFile> {
    let source: string;
    try {
      source = await readFile(file.absPath, "utf-8");
    } catch {
      return { filePath: file.path, language: "swift", symbols: [], edges: [], imports: [] };
    }

    try {
      const lines = source.split("\n");
      const symbols: ParsedSymbol[] = [];
      const edges: ParsedEdge[] = [];
      const imports: ParsedImport[] = [];

      let i = 0;
      while (i < lines.length) {
        const line = lines[i];

        // Import
        const importMatch = line.match(IMPORT_RE);
        if (importMatch) {
          const mod = importMatch[1].trim();
          imports.push({ targetPath: mod, names: [mod] });
          i++;
          continue;
        }

        // Try declaration patterns
        const decl = this.matchDeclaration(lines, i);
        if (decl) {
          symbols.push(decl.symbol);
          if (decl.symbol.kind === "function" && decl.body) {
            edges.push(...this.extractEdges(decl.symbol.name, decl.body));
          }
          i = decl.nextLine;
          continue;
        }

        // Module-level let/var (only at depth 0 — simple heuristic: line not indented)
        const lvMatch = line.match(LET_VAR_RE);
        if (lvMatch && !line.match(/^[ \t]{4,}/)) {
          const [, accessMod, keyword, varName] = lvMatch;
          const isExported = this.isExported(accessMod ?? null);
          const docstring = this.extractDocstring(lines, i);
          symbols.push({
            name: varName.trim(),
            kind: "const",
            signature: line.replace(/\{.*$/, "").trim(),
            docstring,
            lineStart: i + 1,
            lineEnd: i + 1,
            isExported,
          });
        }

        i++;
      }

      return { filePath: file.path, language: "swift", symbols, edges, imports };
    } catch {
      return { filePath: file.path, language: "swift", symbols: [], edges: [], imports: [] };
    }
  }

  private matchDeclaration(
    lines: string[],
    startIdx: number,
  ): { symbol: ParsedSymbol; body: string | null; nextLine: number } | null {
    const line = lines[startIdx];

    for (const { re, kind } of PATTERNS) {
      const m = line.match(re);
      if (!m) continue;

      // m[1] = optional access modifier, m[2] = name (possibly with generic params)
      const accessMod = m[1] ?? null;
      // Name may include generic params like `MyView<T>` — trim at `<` or `{`
      let rawName = m[2].replace(/\s*\{.*$/, "").replace(/\s*\(.*$/, "").trim();
      // For extension, suffix with "+extension"
      if (kind === "class" && /\bextension\b/i.test(line)) {
        rawName = rawName + "+extension";
      }

      const isExported = this.isExported(accessMod);
      const docstring = this.extractDocstring(lines, startIdx);

      // Signature: declaration line up to `{`
      const sigRaw = line.replace(/\{.*$/, "").trim();
      const signature = sigRaw || null;

      const { lineEnd, body } = this.findBlockEnd(lines, startIdx);

      return {
        symbol: {
          name: rawName,
          kind,
          signature,
          docstring,
          lineStart: startIdx + 1,
          lineEnd: lineEnd + 1,
          isExported,
        },
        body,
        nextLine: lineEnd + 1,
      };
    }

    return null;
  }

  private isExported(accessMod: string | null): boolean {
    // Swift default access is `internal` (visible within module).
    // We treat public/open as exported; no modifier or internal as not exported.
    if (!accessMod) return false;
    const lower = accessMod.toLowerCase();
    return lower === "public" || lower === "open";
  }

  private extractDocstring(lines: string[], declLine: number): string | null {
    // Collect `///` or `/** ... */` comment block immediately before declLine
    const tripleSlash: string[] = [];
    let i = declLine - 1;

    // Skip blank lines
    while (i >= 0 && lines[i].trim() === "") i--;

    // Collect /// lines
    while (i >= 0 && lines[i].trim().startsWith("///")) {
      tripleSlash.unshift(lines[i].trim().replace(/^\/\/\/\s?/, ""));
      i--;
    }

    if (tripleSlash.length > 0) {
      return tripleSlash.join("\n");
    }

    // Try /** ... */ block comment ending just before declLine
    let j = declLine - 1;
    while (j >= 0 && lines[j].trim() === "") j--;
    if (j >= 0 && lines[j].trim().endsWith("*/")) {
      const blockLines: string[] = [];
      while (j >= 0) {
        const t = lines[j].trim();
        blockLines.unshift(t.replace(/^\/\*\*?\s?/, "").replace(/\s?\*\/$/, "").replace(/^\*\s?/, ""));
        if (t.startsWith("/**") || t.startsWith("/*")) break;
        j--;
      }
      const text = blockLines.join("\n").trim();
      return text || null;
    }

    return null;
  }

  private findBlockEnd(
    lines: string[],
    startIdx: number,
  ): { lineEnd: number; body: string | null } {
    // Count braces to find the matching closing `}`
    let depth = 0;
    let foundOpen = false;
    const bodyLines: string[] = [];

    for (let i = startIdx; i < lines.length; i++) {
      const raw = lines[i];
      bodyLines.push(raw);

      for (const ch of raw) {
        if (ch === "{") { depth++; foundOpen = true; }
        else if (ch === "}") { depth--; }
      }

      if (foundOpen && depth === 0) {
        return { lineEnd: i, body: bodyLines.join("\n") };
      }
    }

    // No closing brace found (e.g. protocol requirement, typealias) — single line
    return { lineEnd: startIdx, body: null };
  }

  private extractEdges(callerName: string, body: string): ParsedEdge[] {
    const edges: ParsedEdge[] = [];
    const seen = new Set<string>();
    CALL_RE.lastIndex = 0;
    let m: RegExpExecArray | null;
    while ((m = CALL_RE.exec(body)) !== null) {
      const calleeName = m[1];
      if (SWIFT_KEYWORDS.has(calleeName)) continue;
      if (calleeName === callerName) continue;
      if (!seen.has(calleeName)) {
        seen.add(calleeName);
        edges.push({ callerName, calleeName, calleeFile: null });
      }
    }
    return edges;
  }
}
