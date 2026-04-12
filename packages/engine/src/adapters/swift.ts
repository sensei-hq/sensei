import { TreeSitterAdapter, type TSNode } from "./tree-sitter-base.js";
import type { ParsedSymbol, ParsedImport, SymbolKind } from "@sensei/shared";

let _lang: unknown = null;

export class SwiftAdapter extends TreeSitterAdapter {
  readonly language = "swift";
  readonly extensions = [".swift"];

  protected getLanguage(): unknown {
    if (!_lang) _lang = require("tree-sitter-swift");
    return _lang;
  }

  protected extractSymbols(root: TSNode, lines: string[]): ParsedSymbol[] {
    const symbols: ParsedSymbol[] = [];
    this.visitNode(root, lines, symbols);
    return symbols;
  }

  private visitNode(node: TSNode, lines: string[], symbols: ParsedSymbol[]): void {
    for (let i = 0; i < node.childCount; i++) {
      const child = node.child(i)!;
      switch (child.type) {
        case "function_declaration": {
          const nameNode = child.childForFieldName("name");
          if (!nameNode) break;
          symbols.push({
            name: nameNode.text,
            kind: "function",
            signature: this.extractSignature(child, lines),
            docstring: this.extractSwiftDocstring(node, i),
            lineStart: child.startPosition.row + 1,
            lineEnd: child.endPosition.row + 1,
            isExported: this.isSwiftExported(child),
          });
          // Recurse into the function body
          const fnBody = child.childForFieldName("body");
          if (fnBody) this.visitNode(fnBody, lines, symbols);
          break;
        }
        case "class_declaration": {
          const nameNode = child.childForFieldName("name");
          if (!nameNode) break;
          symbols.push({
            name: nameNode.text,
            kind: "class",
            signature: this.extractSignature(child, lines),
            docstring: this.extractSwiftDocstring(node, i),
            lineStart: child.startPosition.row + 1,
            lineEnd: child.endPosition.row + 1,
            isExported: this.isSwiftExported(child),
          });
          const classBody = child.childForFieldName("body");
          if (classBody) this.visitNode(classBody, lines, symbols);
          break;
        }
        case "struct_declaration": {
          const nameNode = child.childForFieldName("name");
          if (!nameNode) break;
          symbols.push({
            name: nameNode.text,
            kind: "class" as SymbolKind,
            signature: this.extractSignature(child, lines),
            docstring: this.extractSwiftDocstring(node, i),
            lineStart: child.startPosition.row + 1,
            lineEnd: child.endPosition.row + 1,
            isExported: this.isSwiftExported(child),
          });
          const structBody = child.childForFieldName("body");
          if (structBody) this.visitNode(structBody, lines, symbols);
          break;
        }
        case "protocol_declaration": {
          const nameNode = child.childForFieldName("name");
          if (!nameNode) break;
          symbols.push({
            name: nameNode.text,
            kind: "interface" as SymbolKind,
            signature: this.extractSignature(child, lines),
            docstring: this.extractSwiftDocstring(node, i),
            lineStart: child.startPosition.row + 1,
            lineEnd: child.endPosition.row + 1,
            isExported: this.isSwiftExported(child),
          });
          const protoBody = child.childForFieldName("body");
          if (protoBody) this.visitNode(protoBody, lines, symbols);
          break;
        }
        case "enum_declaration": {
          const nameNode = child.childForFieldName("name");
          if (!nameNode) break;
          symbols.push({
            name: nameNode.text,
            kind: "enum",
            signature: this.extractSignature(child, lines),
            docstring: this.extractSwiftDocstring(node, i),
            lineStart: child.startPosition.row + 1,
            lineEnd: child.endPosition.row + 1,
            isExported: this.isSwiftExported(child),
          });
          const enumBody = child.childForFieldName("body");
          if (enumBody) this.visitNode(enumBody, lines, symbols);
          break;
        }
        case "typealias_declaration": {
          const nameNode = child.childForFieldName("name");
          if (!nameNode) break;
          symbols.push({
            name: nameNode.text,
            kind: "type",
            signature: this.extractSignature(child, lines),
            docstring: this.extractSwiftDocstring(node, i),
            lineStart: child.startPosition.row + 1,
            lineEnd: child.endPosition.row + 1,
            isExported: this.isSwiftExported(child),
          });
          break;
        }
        default:
          break;
      }
    }
  }

  /**
   * A Swift declaration is exported if it has an `access_control_modifier`
   * child with text `public` or `open`.
   */
  private isSwiftExported(node: TSNode): boolean {
    for (let i = 0; i < node.childCount; i++) {
      const c = node.child(i)!;
      if (c.type === "access_control_modifier") {
        const mod = c.text;
        if (mod === "public" || mod === "open") return true;
      }
    }
    return false;
  }

  /**
   * Collect `///` doc comment lines immediately preceding the symbol node.
   */
  private extractSwiftDocstring(parent: TSNode, symbolIndex: number): string | null {
    const docLines: string[] = [];
    for (let i = symbolIndex - 1; i >= 0; i--) {
      const sibling = parent.child(i)!;
      if (sibling.type === "comment" && sibling.text.startsWith("///")) {
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
      if (node.type === "import_declaration") {
        // e.g. "import Foundation" or "import UIKit.UIView"
        const text = node.text;
        const m = text.match(/^import\s+([\w.]+)/);
        if (m) {
          const mod = m[1].trim();
          imports.push({ targetPath: mod, names: [mod] });
        }
      }
    }
    return imports;
  }
}
