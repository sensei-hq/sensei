import { TreeSitterAdapter, type TSNode } from "./tree-sitter-base.js";
import type { ParsedSymbol, ParsedImport } from "@sensei/shared";

let _lang: unknown = null;

export class KotlinAdapter extends TreeSitterAdapter {
  readonly language = "kotlin";
  readonly extensions = [".kt", ".kts"];

  protected getLanguage(): unknown {
    if (!_lang) _lang = require("tree-sitter-kotlin");
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
          const name = this.getSimpleIdentifier(child);
          if (!name) break;
          symbols.push({
            name,
            kind: "function",
            signature: this.extractSignature(child, lines),
            docstring: this.extractKdoc(node, i),
            lineStart: child.startPosition.row + 1,
            lineEnd: child.endPosition.row + 1,
            isExported: this.isKotlinExported(child),
          });
          // Recurse into function body
          const fnBody = child.childForFieldName("body");
          if (fnBody) this.visitNode(fnBody, lines, symbols);
          break;
        }
        case "class_declaration": {
          const name = this.getSimpleIdentifier(child);
          if (!name) break;
          symbols.push({
            name,
            kind: "class",
            signature: this.extractSignature(child, lines),
            docstring: this.extractKdoc(node, i),
            lineStart: child.startPosition.row + 1,
            lineEnd: child.endPosition.row + 1,
            isExported: this.isKotlinExported(child),
          });
          const classBody = child.childForFieldName("body");
          if (classBody) this.visitNode(classBody, lines, symbols);
          break;
        }
        case "object_declaration": {
          const name = this.getSimpleIdentifier(child);
          if (!name) break;
          symbols.push({
            name,
            kind: "class",
            signature: this.extractSignature(child, lines),
            docstring: this.extractKdoc(node, i),
            lineStart: child.startPosition.row + 1,
            lineEnd: child.endPosition.row + 1,
            isExported: this.isKotlinExported(child),
          });
          const objBody = child.childForFieldName("body");
          if (objBody) this.visitNode(objBody, lines, symbols);
          break;
        }
        case "interface_declaration": {
          const name = this.getSimpleIdentifier(child);
          if (!name) break;
          symbols.push({
            name,
            kind: "interface",
            signature: this.extractSignature(child, lines),
            docstring: this.extractKdoc(node, i),
            lineStart: child.startPosition.row + 1,
            lineEnd: child.endPosition.row + 1,
            isExported: this.isKotlinExported(child),
          });
          const ifaceBody = child.childForFieldName("body");
          if (ifaceBody) this.visitNode(ifaceBody, lines, symbols);
          break;
        }
        case "type_alias": {
          const name = this.getSimpleIdentifier(child);
          if (!name) break;
          symbols.push({
            name,
            kind: "type",
            signature: this.extractSignature(child, lines),
            docstring: this.extractKdoc(node, i),
            lineStart: child.startPosition.row + 1,
            lineEnd: child.endPosition.row + 1,
            isExported: this.isKotlinExported(child),
          });
          break;
        }
        default:
          // Recurse into class bodies, object bodies, etc.
          if (
            child.type === "class_body" ||
            child.type === "enum_class_body" ||
            child.type === "function_body"
          ) {
            this.visitNode(child, lines, symbols);
          }
          break;
      }
    }
  }

  /**
   * Kotlin uses `simple_identifier` as the name child rather than a "name" field
   * in many node types.
   */
  private getSimpleIdentifier(node: TSNode): string | null {
    // Try field name first
    const byField = node.childForFieldName("name");
    if (byField) return byField.text;
    // Fall back to first simple_identifier child
    for (let i = 0; i < node.childCount; i++) {
      const c = node.child(i)!;
      if (c.type === "simple_identifier") return c.text;
    }
    return null;
  }

  /**
   * A Kotlin declaration is NOT exported if it has a `visibility_modifier`
   * child with text `private` or `internal`.
   */
  private isKotlinExported(node: TSNode): boolean {
    for (let i = 0; i < node.childCount; i++) {
      const c = node.child(i)!;
      if (c.type === "visibility_modifier") {
        const mod = c.text;
        if (mod === "private" || mod === "internal") return false;
      }
    }
    return true;
  }

  private extractKdoc(parent: TSNode, symbolIndex: number): string | null {
    for (let i = symbolIndex - 1; i >= 0; i--) {
      const sibling = parent.child(i)!;
      if (sibling.type === "multiline_comment" && sibling.text.startsWith("/**")) {
        const raw = sibling.text;
        const cleaned = raw
          .replace(/^\/\*\*/, "")
          .replace(/\*\/$/, "")
          .split("\n")
          .map((l) => l.trim().replace(/^\*\s?/, ""))
          .filter((l) => l.length > 0)
          .join(" ")
          .trim();
        return cleaned || null;
      }
      // Stop if we hit non-whitespace non-comment content
      if (sibling.text.trim() !== "" && sibling.type !== "multiline_comment") break;
    }
    return null;
  }

  protected extractImports(root: TSNode): ParsedImport[] {
    const imports: ParsedImport[] = [];
    for (let i = 0; i < root.childCount; i++) {
      const node = root.child(i)!;
      if (node.type === "import_header") {
        const text = node.text;
        // e.g. "import kotlin.collections.List" or "import kotlin.collections.*"
        const m = text.match(/^import\s+([\w.]+)(\.\*)?/);
        if (m) {
          const fullPath = m[1] + (m[2] ?? "");
          const isWildcard = !!m[2];
          const name = isWildcard ? "*" : (m[1].split(".").pop() ?? m[1]);
          imports.push({ targetPath: fullPath, names: [name] });
        }
      }
    }
    return imports;
  }
}
