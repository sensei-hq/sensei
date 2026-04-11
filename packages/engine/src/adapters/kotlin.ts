import { readFile } from "node:fs/promises";
import type { FileEntry, ParsedFile, ParsedSymbol, ParsedEdge, ParsedImport } from "@sensei/shared";

export class KotlinAdapter {
  readonly extensions = [".kt", ".kts"];

  async parse(file: FileEntry): Promise<ParsedFile> {
    let source: string;
    try {
      source = await readFile(file.absPath, "utf-8");
    } catch {
      return { filePath: file.path, language: "kotlin", symbols: [], edges: [], imports: [] };
    }

    try {
      const lines = source.split("\n");
      const symbols = this.extractSymbols(lines);
      const imports = this.extractImports(lines);
      const edges = this.extractEdges(lines, symbols);
      return { filePath: file.path, language: "kotlin", symbols, edges, imports };
    } catch {
      return { filePath: file.path, language: "kotlin", symbols: [], edges: [], imports: [] };
    }
  }

  private extractKDoc(lines: string[], lineIndex: number): string | null {
    // Walk backwards from lineIndex - 1 to find a /** ... */ block immediately before
    let i = lineIndex - 1;
    while (i >= 0 && lines[i].trim() === "") i--;
    if (i < 0 || !lines[i].trim().endsWith("*/")) return null;

    const endLine = i;
    while (i >= 0 && !lines[i].trim().startsWith("/**")) i--;
    if (i < 0) return null;

    const docLines = lines.slice(i, endLine + 1);
    const cleaned = docLines
      .map(l => l.trim().replace(/^\/\*\*/, "").replace(/^\*\//, "").replace(/^\*\s?/, "").trim())
      .filter(l => l.length > 0);
    return cleaned.length > 0 ? cleaned.join(" ").trim() : null;
  }

  private extractSymbols(lines: string[]): ParsedSymbol[] {
    const symbols: ParsedSymbol[] = [];

    // fun [modifiers] name(...) — top-level and member functions
    const funRe = /^(\s*)((?:(?:public|private|internal|protected|open|override|suspend|inline|infix|operator|tailrec|external|abstract)\s+)*)fun\s+(?:<[^>]*>\s*)?(\w+)\s*\(/;
    // class variants
    const classRe = /^(\s*)((?:(?:public|private|internal|protected|open|abstract|sealed|data|inner|value|annotation|enum)\s+)*)class\s+(\w+)/;
    // object declarations
    const objectRe = /^(\s*)((?:(?:public|private|internal|protected|open|abstract)\s+)*)object\s+(\w+)/;
    // interface
    const interfaceRe = /^(\s*)((?:(?:public|private|internal|protected|sealed|fun)\s+)*)interface\s+(\w+)/;
    // enum class
    const enumRe = /^(\s*)((?:(?:public|private|internal|protected)\s+)*)enum\s+class\s+(\w+)/;
    // const val or top-level ALL_CAPS val/var
    const constRe = /^(\s*)((?:(?:public|private|internal|protected)\s+)*)(?:const\s+)?(val|var)\s+([A-Z_][A-Z0-9_]*)\s*[=:]/;
    // typealias
    const typealiasRe = /^(\s*)((?:(?:public|private|internal|protected)\s+)*)typealias\s+(\w+)\s*=/;

    for (let i = 0; i < lines.length; i++) {
      const line = lines[i];

      // enum class — must be checked before plain class
      const enumMatch = line.match(enumRe);
      if (enumMatch) {
        const mods = enumMatch[2] ?? "";
        const name = enumMatch[3];
        const sig = this.extractSignature(lines, i);
        const docstring = this.extractKDoc(lines, i);
        const endLine = this.findBlockEnd(lines, i);
        symbols.push({
          name,
          kind: "enum",
          signature: sig,
          docstring,
          lineStart: i + 1,
          lineEnd: endLine,
          isExported: !mods.includes("private") && !mods.includes("internal"),
        });
        continue;
      }

      // class (data class, sealed class, abstract class, etc.)
      const classMatch = line.match(classRe);
      if (classMatch) {
        const mods = classMatch[2] ?? "";
        const name = classMatch[3];
        const sig = this.extractSignature(lines, i);
        const docstring = this.extractKDoc(lines, i);
        const endLine = this.findBlockEnd(lines, i);
        symbols.push({
          name,
          kind: "class",
          signature: sig,
          docstring,
          lineStart: i + 1,
          lineEnd: endLine,
          isExported: !mods.includes("private") && !mods.includes("internal"),
        });
        continue;
      }

      // object
      const objectMatch = line.match(objectRe);
      if (objectMatch) {
        const mods = objectMatch[2] ?? "";
        const name = objectMatch[3];
        const sig = this.extractSignature(lines, i);
        const docstring = this.extractKDoc(lines, i);
        const endLine = this.findBlockEnd(lines, i);
        symbols.push({
          name,
          kind: "class",
          signature: sig,
          docstring,
          lineStart: i + 1,
          lineEnd: endLine,
          isExported: !mods.includes("private") && !mods.includes("internal"),
        });
        continue;
      }

      // interface
      const ifaceMatch = line.match(interfaceRe);
      if (ifaceMatch) {
        const mods = ifaceMatch[2] ?? "";
        const name = ifaceMatch[3];
        const sig = this.extractSignature(lines, i);
        const docstring = this.extractKDoc(lines, i);
        const endLine = this.findBlockEnd(lines, i);
        symbols.push({
          name,
          kind: "interface",
          signature: sig,
          docstring,
          lineStart: i + 1,
          lineEnd: endLine,
          isExported: !mods.includes("private") && !mods.includes("internal"),
        });
        continue;
      }

      // fun
      const funMatch = line.match(funRe);
      if (funMatch) {
        const mods = funMatch[2] ?? "";
        const name = funMatch[3];
        const sig = this.extractSignature(lines, i);
        const docstring = this.extractKDoc(lines, i);
        const endLine = this.findBlockEnd(lines, i);
        symbols.push({
          name,
          kind: "function",
          signature: sig,
          docstring,
          lineStart: i + 1,
          lineEnd: endLine,
          isExported: !mods.includes("private") && !mods.includes("internal"),
        });
        continue;
      }

      // const val / top-level ALL_CAPS
      const constMatch = line.match(constRe);
      if (constMatch) {
        const mods = constMatch[2] ?? "";
        const name = constMatch[4];
        const sig = line.split("=")[0].trim();
        const docstring = this.extractKDoc(lines, i);
        symbols.push({
          name,
          kind: "const",
          signature: sig,
          docstring,
          lineStart: i + 1,
          lineEnd: i + 1,
          isExported: !mods.includes("private") && !mods.includes("internal"),
        });
        continue;
      }

      // typealias
      const typealiasMatch = line.match(typealiasRe);
      if (typealiasMatch) {
        const mods = typealiasMatch[2] ?? "";
        const name = typealiasMatch[3];
        const sig = line.trim();
        const docstring = this.extractKDoc(lines, i);
        symbols.push({
          name,
          kind: "type",
          signature: sig,
          docstring,
          lineStart: i + 1,
          lineEnd: i + 1,
          isExported: !mods.includes("private") && !mods.includes("internal"),
        });
      }
    }

    return symbols;
  }

  private extractSignature(lines: string[], startLine: number): string {
    // Collect declaration up to `{` or `=` (expression body)
    let sig = "";
    for (let i = startLine; i < lines.length && i < startLine + 10; i++) {
      const l = lines[i];
      const braceIdx = l.indexOf("{");
      const eqIdx = l.indexOf("=");
      const earliest =
        braceIdx !== -1 && eqIdx !== -1 ? Math.min(braceIdx, eqIdx) :
        braceIdx !== -1 ? braceIdx :
        eqIdx !== -1 ? eqIdx : -1;
      if (earliest !== -1) {
        sig += " " + l.slice(0, earliest);
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
      // Expression body (no braces): single line with `=`
      if (!found && lines[i].includes("=") && i === startLine) return i + 1;
    }
    return startLine + 1;
  }

  private extractImports(lines: string[]): ParsedImport[] {
    const imports: ParsedImport[] = [];
    const importRe = /^\s*import\s+([\w.]+)(\.\*)?/;
    for (const line of lines) {
      const m = line.match(importRe);
      if (m) {
        const fullPath = m[1] + (m[2] ?? "");
        const isWildcard = !!m[2];
        const name = isWildcard ? "*" : fullPath.split(".").pop() ?? fullPath;
        imports.push({ targetPath: fullPath, names: [name] });
      }
    }
    return imports;
  }

  private extractEdges(lines: string[], symbols: ParsedSymbol[]): ParsedEdge[] {
    const edges: ParsedEdge[] = [];
    const funSymbols = symbols.filter(s => s.kind === "function");
    const callRe = /\b(\w+)\s*\(/g;

    for (const fn of funSymbols) {
      const bodyLines = lines.slice(fn.lineStart - 1, fn.lineEnd);
      const seen = new Set<string>();
      for (const line of bodyLines) {
        let m: RegExpExecArray | null;
        callRe.lastIndex = 0;
        while ((m = callRe.exec(line)) !== null) {
          const callee = m[1];
          if (/^(if|for|while|when|try|catch|return|throw|object|fun|class|val|var)$/.test(callee)) continue;
          if (callee === fn.name) continue;
          if (!seen.has(callee)) {
            seen.add(callee);
            edges.push({ callerName: fn.name, calleeName: callee, calleeFile: null });
          }
        }
      }
    }

    return edges;
  }
}
