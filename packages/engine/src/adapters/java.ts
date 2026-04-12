import { TreeSitterAdapter, type TSNode } from "./tree-sitter-base.js";
import type { ParsedSymbol, ParsedImport } from "@sensei/shared";

let _lang: unknown = null;

export class JavaAdapter extends TreeSitterAdapter {
  readonly language = "java";
  readonly extensions = [".java"];

  protected getLanguage(): unknown {
    if (!_lang) _lang = require("tree-sitter-java");
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
        case "class_declaration": {
          const nameNode = child.childForFieldName("name");
          if (!nameNode) break;
          const isExported = this.hasJavaModifier(child, "public");
          symbols.push({
            name: nameNode.text,
            kind: "class",
            signature: this.extractSignature(child, lines),
            docstring: this.extractJavadoc(node, i),
            lineStart: child.startPosition.row + 1,
            lineEnd: child.endPosition.row + 1,
            isExported,
          });
          // Recurse into class body
          const classBody = child.childForFieldName("body");
          if (classBody) this.visitNode(classBody, lines, symbols);
          break;
        }
        case "interface_declaration": {
          const nameNode = child.childForFieldName("name");
          if (!nameNode) break;
          symbols.push({
            name: nameNode.text,
            kind: "interface",
            signature: this.extractSignature(child, lines),
            docstring: this.extractJavadoc(node, i),
            lineStart: child.startPosition.row + 1,
            lineEnd: child.endPosition.row + 1,
            isExported: this.hasJavaModifier(child, "public"),
          });
          const ifaceBody = child.childForFieldName("body");
          if (ifaceBody) this.visitNode(ifaceBody, lines, symbols);
          break;
        }
        case "enum_declaration": {
          const nameNode = child.childForFieldName("name");
          if (!nameNode) break;
          symbols.push({
            name: nameNode.text,
            kind: "enum",
            signature: this.extractSignature(child, lines),
            docstring: this.extractJavadoc(node, i),
            lineStart: child.startPosition.row + 1,
            lineEnd: child.endPosition.row + 1,
            isExported: this.hasJavaModifier(child, "public"),
          });
          const enumBody = child.childForFieldName("body");
          if (enumBody) this.visitNode(enumBody, lines, symbols);
          break;
        }
        case "method_declaration": {
          const nameNode = child.childForFieldName("name");
          if (!nameNode) break;
          const isExported = this.hasJavaModifier(child, "public") || this.hasJavaModifier(child, "protected");
          symbols.push({
            name: nameNode.text,
            kind: "method",
            signature: this.extractSignature(child, lines),
            docstring: this.extractJavadoc(node, i),
            lineStart: child.startPosition.row + 1,
            lineEnd: child.endPosition.row + 1,
            isExported,
          });
          break;
        }
        case "constructor_declaration": {
          const nameNode = child.childForFieldName("name");
          if (!nameNode) break;
          const isExported = this.hasJavaModifier(child, "public") || this.hasJavaModifier(child, "protected");
          symbols.push({
            name: nameNode.text,
            kind: "method",
            signature: this.extractSignature(child, lines),
            docstring: this.extractJavadoc(node, i),
            lineStart: child.startPosition.row + 1,
            lineEnd: child.endPosition.row + 1,
            isExported,
          });
          break;
        }
        case "field_declaration": {
          // Only capture static final constants (ALL_CAPS convention is a bonus, but we capture all)
          if (this.hasJavaModifier(child, "static") && this.hasJavaModifier(child, "final")) {
            // field_declaration may have multiple declarators
            for (let j = 0; j < child.childCount; j++) {
              const declarator = child.child(j)!;
              if (declarator.type === "variable_declarator") {
                const nameNode = declarator.childForFieldName("name");
                if (nameNode) {
                  symbols.push({
                    name: nameNode.text,
                    kind: "const",
                    signature: this.extractSignature(child, lines),
                    docstring: this.extractJavadoc(node, i),
                    lineStart: child.startPosition.row + 1,
                    lineEnd: child.endPosition.row + 1,
                    isExported: this.hasJavaModifier(child, "public"),
                  });
                }
              }
            }
          }
          break;
        }
        default:
          // Recurse for nested blocks (e.g. anonymous classes, lambdas — skip deep nesting)
          break;
      }
    }
  }

  private hasJavaModifier(node: TSNode, modifier: string): boolean {
    const modifiers = node.childForFieldName("modifiers");
    if (!modifiers) return false;
    for (let i = 0; i < modifiers.childCount; i++) {
      const m = modifiers.child(i)!;
      if (m.text === modifier) return true;
    }
    return false;
  }

  private extractJavadoc(parent: TSNode, symbolIndex: number): string | null {
    // Walk backwards in the parent's children to find a block_comment starting with /**
    for (let i = symbolIndex - 1; i >= 0; i--) {
      const sibling = parent.child(i)!;
      if (sibling.type === "block_comment" && sibling.text.startsWith("/**")) {
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
      // Skip whitespace-only nodes
      if (sibling.type !== "line_comment" && sibling.text.trim() !== "") break;
    }
    return null;
  }

  protected extractImports(root: TSNode): ParsedImport[] {
    const imports: ParsedImport[] = [];
    for (let i = 0; i < root.childCount; i++) {
      const node = root.child(i)!;
      if (node.type === "import_declaration") {
        const text = node.text;
        // e.g. "import java.util.List;" or "import static java.util.Collections.*;"
        const m = text.match(/^import\s+(?:static\s+)?([\w.]+)(\.\*)?;/);
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
