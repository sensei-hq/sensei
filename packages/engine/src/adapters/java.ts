import { readFile } from "node:fs/promises";
import type { FileEntry, ParsedFile, ParsedSymbol, ParsedEdge, ParsedImport } from "@sensei/shared";

export class JavaAdapter {
  readonly extensions = [".java"];

  async parse(file: FileEntry): Promise<ParsedFile> {
    let source: string;
    try {
      source = await readFile(file.absPath, "utf-8");
    } catch {
      return { filePath: file.path, language: "java", symbols: [], edges: [], imports: [] };
    }

    try {
      const lines = source.split("\n");
      const symbols = this.extractSymbols(lines);
      const imports = this.extractImports(lines);
      const edges = this.extractEdges(lines, symbols);
      return { filePath: file.path, language: "java", symbols, edges, imports };
    } catch {
      return { filePath: file.path, language: "java", symbols: [], edges: [], imports: [] };
    }
  }

  private extractDocstring(lines: string[], lineIndex: number): string | null {
    // Walk backwards from lineIndex - 1 to find a /** ... */ block immediately before
    let i = lineIndex - 1;
    // Skip blank lines
    while (i >= 0 && lines[i].trim() === "") i--;
    if (i < 0 || !lines[i].trim().endsWith("*/")) return null;

    const endLine = i;
    while (i >= 0 && !lines[i].trim().startsWith("/**")) i--;
    if (i < 0) return null;

    const docLines = lines.slice(i, endLine + 1);
    // Strip leading * and /** */ markers
    const cleaned = docLines
      .map(l => l.trim().replace(/^\/\*\*/, "").replace(/^\*\//, "").replace(/^\*\s?/, "").trim())
      .filter(l => l.length > 0);
    return cleaned.length > 0 ? cleaned.join(" ").trim() : null;
  }

  private extractSymbols(lines: string[]): ParsedSymbol[] {
    const symbols: ParsedSymbol[] = [];

    const classRe = /^(\s*)(public\s+|protected\s+|private\s+)?(abstract\s+)?(class)\s+(\w+)/;
    const interfaceRe = /^(\s*)(public\s+|protected\s+|private\s+)?(interface)\s+(\w+)/;
    const enumRe = /^(\s*)(public\s+|protected\s+|private\s+)?(enum)\s+(\w+)/;
    const methodRe = /^(\s*)(public\s+|protected\s+|private\s+)?(static\s+)?((?:final\s+|abstract\s+|synchronized\s+|native\s+)*)([\w<>\[\],\s]+?)\s+(\w+)\s*\(/;
    const constRe = /^(\s*)(public\s+|protected\s+|private\s+)?static\s+final\s+([\w<>\[\],\s]+?)\s+([A-Z_][A-Z0-9_]*)\s*=/;

    for (let i = 0; i < lines.length; i++) {
      const line = lines[i];

      // Class
      const classMatch = line.match(classRe);
      if (classMatch) {
        const access = classMatch[2]?.trim() ?? "";
        const name = classMatch[5];
        const sig = this.extractSignature(lines, i);
        const docstring = this.extractDocstring(lines, i);
        const endLine = this.findBlockEnd(lines, i);
        symbols.push({
          name,
          kind: "class",
          signature: sig,
          docstring,
          lineStart: i + 1,
          lineEnd: endLine,
          isExported: access === "public",
        });
        continue;
      }

      // Interface
      const ifaceMatch = line.match(interfaceRe);
      if (ifaceMatch) {
        const access = ifaceMatch[2]?.trim() ?? "";
        const name = ifaceMatch[4];
        const sig = this.extractSignature(lines, i);
        const docstring = this.extractDocstring(lines, i);
        const endLine = this.findBlockEnd(lines, i);
        symbols.push({
          name,
          kind: "interface",
          signature: sig,
          docstring,
          lineStart: i + 1,
          lineEnd: endLine,
          isExported: access === "public",
        });
        continue;
      }

      // Enum
      const enumMatch = line.match(enumRe);
      if (enumMatch) {
        const access = enumMatch[2]?.trim() ?? "";
        const name = enumMatch[4];
        const sig = this.extractSignature(lines, i);
        const docstring = this.extractDocstring(lines, i);
        const endLine = this.findBlockEnd(lines, i);
        symbols.push({
          name,
          kind: "enum",
          signature: sig,
          docstring,
          lineStart: i + 1,
          lineEnd: endLine,
          isExported: access === "public",
        });
        continue;
      }

      // Static final constant
      const constMatch = line.match(constRe);
      if (constMatch) {
        const access = constMatch[2]?.trim() ?? "";
        const name = constMatch[4];
        const sig = line.split("=")[0].trim();
        const docstring = this.extractDocstring(lines, i);
        symbols.push({
          name,
          kind: "const",
          signature: sig,
          docstring,
          lineStart: i + 1,
          lineEnd: i + 1,
          isExported: access === "public",
        });
        continue;
      }

      // Method — must be inside a class body (indented)
      const methodMatch = line.match(methodRe);
      if (methodMatch) {
        const indent = methodMatch[1];
        if (indent.length > 0) {
          const access = methodMatch[2]?.trim() ?? "";
          const name = methodMatch[6];
          // Skip control-flow keywords
          if (/^(if|for|while|switch|catch|return|throw|new)$/.test(name)) continue;
          const sig = this.extractSignature(lines, i);
          const docstring = this.extractDocstring(lines, i);
          const endLine = this.findBlockEnd(lines, i);
          symbols.push({
            name,
            kind: "method",
            signature: sig,
            docstring,
            lineStart: i + 1,
            lineEnd: endLine,
            isExported: access === "public" || access === "protected",
          });
        }
      }
    }

    return symbols;
  }

  private extractSignature(lines: string[], startLine: number): string {
    let sig = "";
    for (let i = startLine; i < lines.length && i < startLine + 10; i++) {
      const l = lines[i];
      const braceIdx = l.indexOf("{");
      const semiIdx = l.indexOf(";");
      if (braceIdx !== -1 && (semiIdx === -1 || braceIdx < semiIdx)) {
        sig += " " + l.slice(0, braceIdx);
        break;
      }
      if (semiIdx !== -1) {
        sig += " " + l.slice(0, semiIdx);
        break;
      }
      sig += " " + l;
    }
    return sig.trim().replace(/\s+/g, " ");
  }

  private findBlockEnd(lines: string[], startLine: number): number {
    let depth = 0;
    let found = false;
    for (let i = startLine; i < lines.length; i++) {
      for (const ch of lines[i]) {
        if (ch === "{") { depth++; found = true; }
        else if (ch === "}") { depth--; }
      }
      if (found && depth === 0) return i + 1;
    }
    return startLine + 1;
  }

  private extractImports(lines: string[]): ParsedImport[] {
    const imports: ParsedImport[] = [];
    const importRe = /^\s*import\s+(static\s+)?([\w.]+)(\.\*)?;/;
    for (const line of lines) {
      const m = line.match(importRe);
      if (m) {
        const fullPath = m[2] + (m[3] ?? "");
        const isWildcard = !!m[3];
        const name = isWildcard ? "*" : fullPath.split(".").pop() ?? fullPath;
        imports.push({ targetPath: fullPath, names: [name] });
      }
    }
    return imports;
  }

  private extractEdges(lines: string[], symbols: ParsedSymbol[]): ParsedEdge[] {
    const edges: ParsedEdge[] = [];
    const methodSymbols = symbols.filter(s => s.kind === "method");
    const callRe = /\b(\w+)\s*\(/g;

    for (const method of methodSymbols) {
      const bodyLines = lines.slice(method.lineStart - 1, method.lineEnd);
      const seen = new Set<string>();
      for (const line of bodyLines) {
        let m: RegExpExecArray | null;
        callRe.lastIndex = 0;
        while ((m = callRe.exec(line)) !== null) {
          const callee = m[1];
          if (/^(if|for|while|switch|catch|return|throw|new|super|this)$/.test(callee)) continue;
          if (callee === method.name) continue;
          if (!seen.has(callee)) {
            seen.add(callee);
            edges.push({ callerName: method.name, calleeName: callee, calleeFile: null });
          }
        }
      }
    }

    return edges;
  }
}
