import type { ParsedSymbol, ParsedImport, SymbolKind } from "@sensei/shared";
import { TreeSitterAdapter, type TSNode } from "./tree-sitter-base.js";

let _lang: unknown = null;

export class PythonAdapter extends TreeSitterAdapter {
  readonly language = "python";
  readonly extensions = [".py"];

  protected getLanguage() {
    if (!_lang) _lang = require("tree-sitter-python");
    return _lang;
  }

  protected extractSymbols(root: TSNode, lines: string[]): ParsedSymbol[] {
    const symbols: ParsedSymbol[] = [];
    this.walkSymbols(root, lines, symbols, false);
    return symbols;
  }

  private walkSymbols(node: TSNode, lines: string[], symbols: ParsedSymbol[], insideClass: boolean): void {
    for (let i = 0; i < node.childCount; i++) {
      const c = node.child(i)!;

      if (c.type === "function_definition") {
        const name = c.childForFieldName("name")?.text ?? "";
        const kind: SymbolKind = insideClass ? "method" : "function";
        symbols.push({
          name,
          kind,
          signature: this.extractSignature(c, lines),
          docstring: this.extractPythonDocstring(c),
          lineStart: c.startPosition.row + 1,
          lineEnd: c.endPosition.row + 1,
          isExported: !name.startsWith("_"),
        });
      } else if (c.type === "class_definition") {
        const name = c.childForFieldName("name")?.text ?? "";
        symbols.push({
          name,
          kind: "class",
          signature: this.extractSignature(c, lines),
          docstring: this.extractPythonDocstring(c),
          lineStart: c.startPosition.row + 1,
          lineEnd: c.endPosition.row + 1,
          isExported: !name.startsWith("_"),
        });
        // Recurse into class body for methods
        const body = c.childForFieldName("body");
        if (body) this.walkSymbols(body, lines, symbols, true);
      } else if (c.type === "expression_statement" && !insideClass) {
        // Top-level constants: TIMEOUT = 30
        const expr = c.child(0);
        if (expr?.type === "assignment") {
          const left = expr.childForFieldName("left");
          if (left?.type === "identifier" && left.text === left.text.toUpperCase() && left.text.length > 1) {
            symbols.push({
              name: left.text,
              kind: "const",
              signature: lines[c.startPosition.row]?.trim() ?? "",
              docstring: null,
              lineStart: c.startPosition.row + 1,
              lineEnd: c.endPosition.row + 1,
              isExported: !left.text.startsWith("_"),
            });
          }
        }
      }
    }
  }

  private extractPythonDocstring(node: TSNode): string | null {
    const body = node.childForFieldName("body");
    if (!body) return null;
    const first = body.child(0);
    if (!first || first.type !== "expression_statement") return null;
    const str = first.child(0);
    if (!str || str.type !== "string") return null;
    let text = str.text;
    if (text.startsWith('"""') || text.startsWith("'''")) text = text.slice(3, -3).trim();
    else if (text.startsWith('"') || text.startsWith("'")) text = text.slice(1, -1).trim();
    return text || null;
  }

  protected extractImports(root: TSNode): ParsedImport[] {
    const imports: ParsedImport[] = [];
    for (let i = 0; i < root.childCount; i++) {
      const c = root.child(i)!;
      if (c.type === "import_statement") {
        for (let j = 0; j < c.childCount; j++) {
          const name = c.child(j)!;
          if (name.type === "dotted_name") {
            imports.push({ targetPath: name.text, names: [name.text.split(".").at(-1) ?? name.text] });
          }
        }
      } else if (c.type === "import_from_statement") {
        const module = c.childForFieldName("module_name");
        const targetPath = module?.text ?? ".";
        const names: string[] = [];
        for (let j = 0; j < c.childCount; j++) {
          const child = c.child(j)!;
          if (child.type === "dotted_name" && child !== module) names.push(child.text);
          else if (child.type === "aliased_import") names.push(child.childForFieldName("name")?.text ?? child.text);
        }
        if (names.length > 0 || targetPath !== ".") imports.push({ targetPath, names });
      }
    }
    return imports;
  }
}
