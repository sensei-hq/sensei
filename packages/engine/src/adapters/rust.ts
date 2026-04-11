import { readFile } from "node:fs/promises";
import type { FileEntry, ParsedFile, ParsedSymbol, ParsedEdge, ParsedImport } from "@sensei/shared";

export class RustAdapter {
  readonly extensions = [".rs"];

  async parse(file: FileEntry): Promise<ParsedFile> {
    try {
      const content = await readFile(file.absPath, "utf-8");
      const lines = content.split("\n");

      const symbols = this.extractSymbols(lines);
      const imports = this.extractImports(lines);
      const edges = this.extractEdges(lines, symbols);

      return { filePath: file.path, language: "rust", symbols, edges, imports };
    } catch {
      return { filePath: file.path, language: "rust", symbols: [], edges: [], imports: [] };
    }
  }

  private extractDocstring(lines: string[], lineIndex: number): string | null {
    const docLines: string[] = [];
    let i = lineIndex - 1;
    while (i >= 0) {
      const trimmed = lines[i].trim();
      if (trimmed.startsWith("///") || trimmed.startsWith("//!")) {
        docLines.unshift(trimmed.replace(/^\/\/[/!]\s?/, ""));
        i--;
      } else {
        break;
      }
    }
    return docLines.length > 0 ? docLines.join("\n") : null;
  }

  private findMatchingBrace(lines: string[], startLine: number): number {
    let depth = 0;
    let found = false;
    for (let i = startLine; i < lines.length; i++) {
      for (const ch of lines[i]) {
        if (ch === "{") {
          depth++;
          found = true;
        } else if (ch === "}") {
          depth--;
          if (found && depth === 0) {
            return i + 1; // 1-indexed
          }
        }
      }
    }
    return startLine + 1;
  }

  private extractSymbols(lines: string[]): ParsedSymbol[] {
    const symbols: ParsedSymbol[] = [];

    const fnRe = /^(pub(?:\([^)]*\))?\s+)?(?:async\s+)?fn\s+(\w+)\s*([^{;]*)/;
    const structRe = /^(pub(?:\([^)]*\))?\s+)?struct\s+(\w+)/;
    const enumRe = /^(pub(?:\([^)]*\))?\s+)?enum\s+(\w+)/;
    const traitRe = /^(pub(?:\([^)]*\))?\s+)?trait\s+(\w+)/;
    const typeRe = /^(pub(?:\([^)]*\))?\s+)?type\s+(\w+)/;
    const constRe = /^(pub(?:\([^)]*\))?\s+)?const\s+(\w+)/;
    const implRe = /^impl(?:<[^>]*>)?\s+(?:\w+::)*(\w+)(?:<[^>]*>)?(?:\s+for\s+\S+)?\s*\{/;

    let i = 0;
    while (i < lines.length) {
      const line = lines[i];
      const trimmed = line.trim();

      // impl blocks — extract methods inside
      const implMatch = trimmed.match(implRe);
      if (implMatch) {
        const implLineEnd = this.findMatchingBrace(lines, i);

        for (let j = i + 1; j < implLineEnd - 1 && j < lines.length; j++) {
          const innerTrimmed = lines[j].trim();
          const methodMatch = innerTrimmed.match(/^(pub(?:\([^)]*\))?\s+)?(?:async\s+)?fn\s+(\w+)\s*([^{;]*)/);
          if (methodMatch) {
            const isPub = !!methodMatch[1];
            const name = methodMatch[2];
            const sigLine = innerTrimmed.split("{")[0].split(";")[0].trim();
            const docstring = this.extractDocstring(lines, j);
            const methodLineEnd = this.findMatchingBrace(lines, j);

            symbols.push({
              name,
              kind: "method",
              signature: sigLine,
              docstring,
              lineStart: j + 1,
              lineEnd: methodLineEnd,
              isExported: isPub,
            });
          }
        }

        i = implLineEnd;
        continue;
      }

      // Top-level fn
      if (!trimmed.startsWith("//")) {
        const fnMatch = trimmed.match(fnRe);
        if (fnMatch) {
          const isPub = !!fnMatch[1];
          const name = fnMatch[2];
          const sigLine = trimmed.split("{")[0].split(";")[0].trim();
          const docstring = this.extractDocstring(lines, i);
          const hasBrace = trimmed.includes("{");
          const lineEnd = hasBrace ? this.findMatchingBrace(lines, i) : i + 1;

          symbols.push({
            name,
            kind: "function",
            signature: sigLine,
            docstring,
            lineStart: i + 1,
            lineEnd,
            isExported: isPub,
          });
          i++;
          continue;
        }

        // struct
        const structMatch = trimmed.match(structRe);
        if (structMatch) {
          const isPub = !!structMatch[1];
          const name = structMatch[2];
          const sigLine = trimmed.split("{")[0].split(";")[0].trim();
          const docstring = this.extractDocstring(lines, i);
          const hasBrace = trimmed.includes("{");
          const lineEnd = hasBrace ? this.findMatchingBrace(lines, i) : i + 1;

          symbols.push({
            name,
            kind: "class",
            signature: sigLine,
            docstring,
            lineStart: i + 1,
            lineEnd,
            isExported: isPub,
          });
          i++;
          continue;
        }

        // enum
        const enumMatch = trimmed.match(enumRe);
        if (enumMatch) {
          const isPub = !!enumMatch[1];
          const name = enumMatch[2];
          const sigLine = trimmed.split("{")[0].split(";")[0].trim();
          const docstring = this.extractDocstring(lines, i);
          const hasBrace = trimmed.includes("{");
          const lineEnd = hasBrace ? this.findMatchingBrace(lines, i) : i + 1;

          symbols.push({
            name,
            kind: "enum",
            signature: sigLine,
            docstring,
            lineStart: i + 1,
            lineEnd,
            isExported: isPub,
          });
          i++;
          continue;
        }

        // trait
        const traitMatch = trimmed.match(traitRe);
        if (traitMatch) {
          const isPub = !!traitMatch[1];
          const name = traitMatch[2];
          const sigLine = trimmed.split("{")[0].split(";")[0].trim();
          const docstring = this.extractDocstring(lines, i);
          const hasBrace = trimmed.includes("{");
          const lineEnd = hasBrace ? this.findMatchingBrace(lines, i) : i + 1;

          symbols.push({
            name,
            kind: "interface",
            signature: sigLine,
            docstring,
            lineStart: i + 1,
            lineEnd,
            isExported: isPub,
          });
          i++;
          continue;
        }

        // type alias
        const typeMatch = trimmed.match(typeRe);
        if (typeMatch) {
          const isPub = !!typeMatch[1];
          const name = typeMatch[2];
          const sigLine = trimmed.split(";")[0].trim();
          const docstring = this.extractDocstring(lines, i);

          symbols.push({
            name,
            kind: "type",
            signature: sigLine,
            docstring,
            lineStart: i + 1,
            lineEnd: i + 1,
            isExported: isPub,
          });
          i++;
          continue;
        }

        // const
        const constMatch = trimmed.match(constRe);
        if (constMatch) {
          const isPub = !!constMatch[1];
          const name = constMatch[2];
          const sigLine = trimmed.split(";")[0].trim();
          const docstring = this.extractDocstring(lines, i);

          symbols.push({
            name,
            kind: "const",
            signature: sigLine,
            docstring,
            lineStart: i + 1,
            lineEnd: i + 1,
            isExported: isPub,
          });
          i++;
          continue;
        }
      }

      i++;
    }

    return symbols;
  }

  private extractImports(lines: string[]): ParsedImport[] {
    const imports: ParsedImport[] = [];
    const useRe = /^(?:pub\s+)?use\s+([^;]+);/;

    for (const line of lines) {
      const trimmed = line.trim();
      const match = trimmed.match(useRe);
      if (!match) continue;

      const usePath = match[1].trim();

      // Multi-import: path::{a, b, c}
      const multiMatch = usePath.match(/^(.*?)::\{([^}]+)\}$/);
      if (multiMatch) {
        const prefix = multiMatch[1].trim();
        const names = multiMatch[2]
          .split(",")
          .map(n => n.trim())
          .filter(Boolean);
        imports.push({ targetPath: prefix, names });
        continue;
      }

      // Single import: path::to::thing
      const parts = usePath.split("::");
      const name = parts[parts.length - 1].trim();
      imports.push({ targetPath: usePath, names: [name] });
    }

    return imports;
  }

  private extractEdges(lines: string[], symbols: ParsedSymbol[]): ParsedEdge[] {
    const edges: ParsedEdge[] = [];
    const symbolNames = new Set(symbols.map(s => s.name));
    const callRe = /\b(\w+)\s*\(/g;

    for (const sym of symbols) {
      if (sym.kind !== "function" && sym.kind !== "method") continue;

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
