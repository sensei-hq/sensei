/**
 * Base class for tree-sitter powered language adapters.
 *
 * Subclasses implement language-specific symbol extraction by mapping
 * tree-sitter node types to ParsedSymbol, ParsedEdge, and ParsedImport.
 */
import { readFile } from "node:fs/promises";
import type { FileEntry, ParsedFile, ParsedSymbol, ParsedEdge, ParsedImport, SymbolKind } from "@sensei/shared";

// Lazy-load tree-sitter to avoid native module errors at import time
let _Parser: any = null;
function getParserClass(): any {
  if (!_Parser) _Parser = require("tree-sitter");
  return _Parser;
}

/** Tree-sitter syntax node — typed loosely to avoid requiring the native module at import time. */
export interface TSNode {
  type: string;
  text: string;
  childCount: number;
  child(index: number): TSNode | null;
  childForFieldName(name: string): TSNode | null;
  startPosition: { row: number; column: number };
  endPosition: { row: number; column: number };
  previousSibling: TSNode | null;
  nextSibling: TSNode | null;
  parent: TSNode | null;
  lastChild: TSNode | null;
}

export abstract class TreeSitterAdapter {
  abstract readonly language: string;
  abstract readonly extensions: string[];
  protected abstract getLanguage(): unknown; // tree-sitter language module

  private parser: any = null;

  protected getParser(): any {
    if (!this.parser) {
      const ParserClass = getParserClass();
      this.parser = new ParserClass();
      this.parser.setLanguage(this.getLanguage());
    }
    return this.parser;
  }

  async parse(file: FileEntry): Promise<ParsedFile> {
    try {
      const content = await readFile(file.absPath, "utf-8");
      const tree = this.getParser().parse(content);
      const root = tree.rootNode;
      const lines = content.split("\n");

      const symbols = this.extractSymbols(root, lines);
      const imports = this.extractImports(root);
      const edges = this.extractEdges(root, symbols);

      return {
        filePath: file.path,
        language: this.language,
        symbols,
        edges,
        imports,
      };
    } catch {
      return { filePath: file.path, language: this.language, symbols: [], edges: [], imports: [] };
    }
  }

  protected abstract extractSymbols(root: TSNode, lines: string[]): ParsedSymbol[];
  protected abstract extractImports(root: TSNode): ParsedImport[];

  /**
   * Default edge extraction: for each function, scan its body for calls
   * to other known symbol names.
   */
  protected extractEdges(root: TSNode, symbols: ParsedSymbol[]): ParsedEdge[] {
    const edges: ParsedEdge[] = [];
    const symbolNames = new Set(symbols.map(s => s.name));

    for (const sym of symbols) {
      if (sym.kind !== "function" && sym.kind !== "method") continue;

      // Find the tree-sitter node for this symbol's body
      const node = this.findNodeAtLine(root, sym.lineStart - 1);
      if (!node) continue;

      const calls = this.findCallsInNode(node, symbolNames);
      for (const callee of calls) {
        if (callee !== sym.name) {
          edges.push({ callerName: sym.name, calleeName: callee, calleeFile: null });
        }
      }
    }
    return edges;
  }

  /** Find calls to known symbols within a node's subtree. */
  protected findCallsInNode(node: TSNode, knownNames: Set<string>): string[] {
    const calls: string[] = [];
    const walk = (n: TSNode) => {
      if (n.type === "call" || n.type === "call_expression") {
        const fn = n.childForFieldName("function") ?? n.child(0);
        if (fn) {
          // Handle simple name calls and method calls
          const name = fn.type === "identifier" ? fn.text
            : fn.type === "attribute" || fn.type === "member_expression" || fn.type === "field_expression"
              ? fn.childForFieldName("attribute")?.text ?? fn.lastChild?.text
              : null;
          if (name && knownNames.has(name)) calls.push(name);
        }
      }
      for (let i = 0; i < n.childCount; i++) walk(n.child(i)!);
    };
    walk(node);
    return [...new Set(calls)];
  }

  /** Find the node at a given 0-based line. */
  protected findNodeAtLine(root: TSNode, line: number): TSNode | null {
    for (let i = 0; i < root.childCount; i++) {
      const c = root.child(i)!;
      if (c.startPosition.row === line) return c;
      if (c.startPosition.row <= line && c.endPosition.row >= line) {
        // Recurse into nested nodes
        const nested = this.findNodeAtLine(c, line);
        return nested ?? c;
      }
    }
    return null;
  }

  /** Extract docstring from a node (looks for first string child in body). */
  protected extractDocstring(node: TSNode): string | null {
    // Language-specific — override in subclass
    return null;
  }

  /** Extract the signature line from a node's text. */
  protected extractSignature(node: TSNode, lines: string[]): string {
    const startLine = node.startPosition.row;
    // Take lines until we hit a body start (indent, {, :, etc.)
    let sig = lines[startLine]?.trim() ?? "";
    if (sig.length > 200) sig = sig.slice(0, 200) + "…";
    return sig;
  }

  /** Helper: check if a node has a modifier/keyword child. */
  protected hasModifier(node: TSNode, ...keywords: string[]): boolean {
    for (let i = 0; i < node.childCount; i++) {
      const c = node.child(i)!;
      if (keywords.includes(c.text) || keywords.includes(c.type)) return true;
    }
    return false;
  }
}
