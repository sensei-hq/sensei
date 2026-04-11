import { readFile } from "node:fs/promises";
import type { FileEntry, ParsedFile, ParsedSymbol, ParsedEdge, ParsedImport } from "@sensei/shared";

export class PythonAdapter {
  readonly extensions = [".py"];

  async parse(file: FileEntry): Promise<ParsedFile> {
    try {
      const content = await readFile(file.absPath, "utf-8");
      const lines = content.split("\n");

      const symbols = this.extractSymbols(lines);
      const imports = this.extractImports(lines);
      const edges = this.extractEdges(lines, symbols);

      return { filePath: file.path, language: "python", symbols, edges, imports };
    } catch {
      return { filePath: file.path, language: "python", symbols: [], edges: [], imports: [] };
    }
  }

  /** Returns indentation level (number of leading spaces, treating tab=4) */
  private indentOf(line: string): number {
    let count = 0;
    for (const ch of line) {
      if (ch === " ") count++;
      else if (ch === "\t") count += 4;
      else break;
    }
    return count;
  }

  /**
   * Find the last line (1-indexed) of a Python block starting at startLine (0-indexed).
   * The block ends when we hit a non-blank, non-comment line at the same or lesser indentation.
   */
  private findBlockEnd(lines: string[], startLine: number): number {
    const headerIndent = this.indentOf(lines[startLine]);
    let lastBodyLine = startLine + 1; // default: at least the next line

    for (let i = startLine + 1; i < lines.length; i++) {
      const raw = lines[i];
      const trimmed = raw.trim();

      // Skip blank lines and comment-only lines — they don't end the block
      if (trimmed === "" || trimmed.startsWith("#")) {
        lastBodyLine = i;
        continue;
      }

      const indent = this.indentOf(raw);
      if (indent <= headerIndent) {
        // Back at or before the header level — block has ended
        return lastBodyLine + 1; // 1-indexed
      }
      lastBodyLine = i;
    }

    return lastBodyLine + 1; // 1-indexed
  }

  /** Extract triple-quoted docstring immediately after a def/class line (0-indexed startLine). */
  private extractDocstring(lines: string[], defLine: number): string | null {
    // Look at the line(s) right after the def/class header
    const bodyStart = defLine + 1;
    if (bodyStart >= lines.length) return null;

    // Collect continuation of the def line (multi-line signatures end with :)
    // Find the actual first body line after the `:` 
    let firstBodyLine = bodyStart;
    // If the def header spans multiple lines, skip until we pass it
    // (heuristic: keep skipping lines that don't contain a statement but end the signature)
    // For simplicity, just look at the next non-blank line
    while (firstBodyLine < lines.length && lines[firstBodyLine].trim() === "") {
      firstBodyLine++;
    }
    if (firstBodyLine >= lines.length) return null;

    const firstTrimmed = lines[firstBodyLine].trim();

    // Check for triple-quote docstring
    for (const q of ['"""', "'''"]) {
      if (firstTrimmed.startsWith(q)) {
        // Could be single-line: """..."""
        const rest = firstTrimmed.slice(q.length);
        const closeIdx = rest.indexOf(q);
        if (closeIdx !== -1) {
          return rest.slice(0, closeIdx).trim() || null;
        }
        // Multi-line docstring
        const docLines: string[] = [rest];
        for (let k = firstBodyLine + 1; k < lines.length; k++) {
          const kLine = lines[k];
          const kTrimmed = kLine.trim();
          const endIdx = kTrimmed.indexOf(q);
          if (endIdx !== -1) {
            docLines.push(kTrimmed.slice(0, endIdx));
            break;
          }
          docLines.push(kTrimmed);
        }
        const result = docLines.join("\n").trim();
        return result || null;
      }
    }

    return null;
  }

  private extractSymbols(lines: string[]): ParsedSymbol[] {
    const symbols: ParsedSymbol[] = [];

    const defRe = /^(\s*)(async\s+)?def\s+(\w+)\s*(\(.*)/;
    const classRe = /^(\s*)class\s+(\w+)/;
    const constRe = /^([A-Z][A-Z0-9_]*)\s*=/;

    for (let i = 0; i < lines.length; i++) {
      const line = lines[i];
      const trimmed = line.trim();

      // def / async def
      const defMatch = line.match(defRe);
      if (defMatch) {
        const indent = defMatch[1];
        const name = defMatch[3];
        const isTopLevel = indent === "" || indent.length === 0;
        const isExported = !name.startsWith("_");

        // Build signature: collect full def line (may span multiple lines ending with :)
        let sig = trimmed;
        // If params span multiple lines, append until we find the closing ) and :
        let sigEnd = i;
        if (!sig.includes("):") && !sig.endsWith(":")) {
          for (let k = i + 1; k < lines.length && k < i + 20; k++) {
            const kTrimmed = lines[k].trim();
            sig += " " + kTrimmed;
            sigEnd = k;
            if (kTrimmed.endsWith(":") || kTrimmed.includes("):")) break;
          }
        }

        const lineEnd = this.findBlockEnd(lines, i);
        const docstring = this.extractDocstring(lines, i);

        symbols.push({
          name,
          kind: "function",
          signature: sig,
          docstring,
          lineStart: i + 1,
          lineEnd,
          isExported,
        });
        continue;
      }

      // class
      const classMatch = line.match(classRe);
      if (classMatch) {
        const name = classMatch[2];
        const isExported = !name.startsWith("_");
        const sig = trimmed.endsWith(":") ? trimmed : trimmed + ":";
        const lineEnd = this.findBlockEnd(lines, i);
        const docstring = this.extractDocstring(lines, i);

        symbols.push({
          name,
          kind: "class",
          signature: sig,
          docstring,
          lineStart: i + 1,
          lineEnd,
          isExported,
        });
        continue;
      }

      // Top-level ALL_CAPS const (not inside a function/class — check zero indentation)
      if (this.indentOf(line) === 0) {
        const constMatch = trimmed.match(constRe);
        if (constMatch && !trimmed.startsWith("#") && !trimmed.startsWith("def") && !trimmed.startsWith("class")) {
          const name = constMatch[1];
          symbols.push({
            name,
            kind: "const",
            signature: trimmed.split("\n")[0].trim(),
            docstring: null,
            lineStart: i + 1,
            lineEnd: i + 1,
            isExported: true,
          });
        }
      }
    }

    return symbols;
  }

  private extractImports(lines: string[]): ParsedImport[] {
    const imports: ParsedImport[] = [];

    // import x
    const importRe = /^import\s+(\S+)/;
    // from x import y, z  /  from . import y
    const fromImportRe = /^from\s+(\S+)\s+import\s+(.+)/;

    for (const line of lines) {
      const trimmed = line.trim();

      const fromMatch = trimmed.match(fromImportRe);
      if (fromMatch) {
        const targetPath = fromMatch[1];
        const rest = fromMatch[2].trim();
        // Handle parenthesised imports on one line: from x import (a, b)
        const cleaned = rest.replace(/[()]/g, "");
        const names = cleaned
          .split(",")
          .map(n => n.trim().split(/\s+as\s+/)[0].trim())
          .filter(Boolean);
        imports.push({ targetPath, names });
        continue;
      }

      const importMatch = trimmed.match(importRe);
      if (importMatch) {
        const mod = importMatch[1];
        const name = mod.split(".").pop() ?? mod;
        imports.push({ targetPath: mod, names: [name] });
      }
    }

    return imports;
  }

  private extractEdges(lines: string[], symbols: ParsedSymbol[]): ParsedEdge[] {
    const edges: ParsedEdge[] = [];
    const symbolNames = new Set(symbols.map(s => s.name));
    const callRe = /\b(\w+)\s*\(/g;

    for (const sym of symbols) {
      if (sym.kind !== "function") continue;

      const bodyStart = sym.lineStart - 1; // 0-indexed
      const bodyEnd = sym.lineEnd - 1;     // 0-indexed

      const seenCallees = new Set<string>();
      for (let i = bodyStart; i < bodyEnd && i < lines.length; i++) {
        let match: RegExpExecArray | null;
        callRe.lastIndex = 0;
        while ((match = callRe.exec(lines[i])) !== null) {
          const callee = match[1];
          if (callee !== sym.name && symbolNames.has(callee) && !seenCallees.has(callee)) {
            seenCallees.add(callee);
            edges.push({ callerName: sym.name, calleeName: callee, calleeFile: null });
          }
        }
      }
    }

    return edges;
  }
}
