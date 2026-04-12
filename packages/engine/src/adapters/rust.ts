import { TreeSitterAdapter, type TSNode } from "./tree-sitter-base.js";
import type { ParsedSymbol, ParsedImport } from "@sensei/shared";

let _lang: unknown = null;

export class RustAdapter extends TreeSitterAdapter {
  readonly language = "rust";
  readonly extensions = [".rs"];

  protected getLanguage(): unknown {
    if (!_lang) _lang = require("tree-sitter-rust");
    return _lang;
  }

  protected extractSymbols(root: TSNode, lines: string[]): ParsedSymbol[] {
    const symbols: ParsedSymbol[] = [];
    this.visitNode(root, lines, symbols, false);
    return symbols;
  }

  private visitNode(
    node: TSNode,
    lines: string[],
    symbols: ParsedSymbol[],
    insideImpl: boolean,
  ): void {
    for (let i = 0; i < node.childCount; i++) {
      const child = node.child(i)!;
      switch (child.type) {
        case "function_item": {
          const nameNode = child.childForFieldName("name");
          if (!nameNode) break;
          const name = nameNode.text;
          const isExported = this.hasModifier(child, "pub");
          const docstring = this.extractRustDocstring(node, i);
          const kind = insideImpl ? "method" : "function";
          symbols.push({
            name,
            kind,
            signature: this.extractSignature(child, lines),
            docstring,
            lineStart: child.startPosition.row + 1,
            lineEnd: child.endPosition.row + 1,
            isExported,
          });
          break;
        }
        case "struct_item": {
          const nameNode = child.childForFieldName("name");
          if (!nameNode) break;
          symbols.push({
            name: nameNode.text,
            kind: "class",
            signature: this.extractSignature(child, lines),
            docstring: this.extractRustDocstring(node, i),
            lineStart: child.startPosition.row + 1,
            lineEnd: child.endPosition.row + 1,
            isExported: this.hasModifier(child, "pub"),
          });
          break;
        }
        case "enum_item": {
          const nameNode = child.childForFieldName("name");
          if (!nameNode) break;
          symbols.push({
            name: nameNode.text,
            kind: "enum",
            signature: this.extractSignature(child, lines),
            docstring: this.extractRustDocstring(node, i),
            lineStart: child.startPosition.row + 1,
            lineEnd: child.endPosition.row + 1,
            isExported: this.hasModifier(child, "pub"),
          });
          break;
        }
        case "trait_item": {
          const nameNode = child.childForFieldName("name");
          if (!nameNode) break;
          symbols.push({
            name: nameNode.text,
            kind: "interface",
            signature: this.extractSignature(child, lines),
            docstring: this.extractRustDocstring(node, i),
            lineStart: child.startPosition.row + 1,
            lineEnd: child.endPosition.row + 1,
            isExported: this.hasModifier(child, "pub"),
          });
          break;
        }
        case "type_item": {
          const nameNode = child.childForFieldName("name");
          if (!nameNode) break;
          symbols.push({
            name: nameNode.text,
            kind: "type",
            signature: this.extractSignature(child, lines),
            docstring: this.extractRustDocstring(node, i),
            lineStart: child.startPosition.row + 1,
            lineEnd: child.endPosition.row + 1,
            isExported: this.hasModifier(child, "pub"),
          });
          break;
        }
        case "const_item": {
          const nameNode = child.childForFieldName("name");
          if (!nameNode) break;
          symbols.push({
            name: nameNode.text,
            kind: "const",
            signature: this.extractSignature(child, lines),
            docstring: this.extractRustDocstring(node, i),
            lineStart: child.startPosition.row + 1,
            lineEnd: child.endPosition.row + 1,
            isExported: this.hasModifier(child, "pub"),
          });
          break;
        }
        case "impl_item": {
          // Recurse into impl blocks to extract methods
          const body = child.childForFieldName("body");
          if (body) {
            this.visitNode(body, lines, symbols, true);
          }
          break;
        }
        default:
          break;
      }
    }
  }

  private extractRustDocstring(parent: TSNode, symbolIndex: number): string | null {
    const docLines: string[] = [];
    for (let i = symbolIndex - 1; i >= 0; i--) {
      const sibling = parent.child(i)!;
      if (sibling.type === "line_comment" && sibling.text.startsWith("///")) {
        docLines.unshift(sibling.text.replace(/^\/\/\/\s?/, ""));
      } else {
        break;
      }
    }
    return docLines.length > 0 ? docLines.join("\n") : null;
  }

  protected extractImports(root: TSNode): ParsedImport[] {
    const imports: ParsedImport[] = [];
    for (let i = 0; i < root.childCount; i++) {
      const node = root.child(i)!;
      if (node.type === "use_declaration") {
        this.parseUseDeclaration(node, imports);
      }
    }
    return imports;
  }

  private parseUseDeclaration(node: TSNode, imports: ParsedImport[]): void {
    const text = node.text;
    // Strip leading `pub use` or `use` and trailing `;`
    const usePath = text.replace(/^(?:pub\s+)?use\s+/, "").replace(/;$/, "").trim();

    // Multi-import: path::{a, b, c}
    const multiMatch = usePath.match(/^(.*?)::\{([^}]+)\}$/);
    if (multiMatch) {
      const prefix = multiMatch[1].trim();
      const names = multiMatch[2]
        .split(",")
        .map((n) => n.trim())
        .filter(Boolean);
      imports.push({ targetPath: prefix, names });
      return;
    }

    // Single import: path::to::thing
    const parts = usePath.split("::");
    const name = parts[parts.length - 1].trim();
    imports.push({ targetPath: usePath, names: [name] });
  }
}
