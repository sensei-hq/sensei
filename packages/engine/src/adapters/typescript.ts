import { Project, SyntaxKind, type SourceFile } from "ts-morph";
import type { FileEntry, ParsedFile, ParsedSymbol, ParsedEdge, ParsedImport, SymbolKind } from "@sensei/shared";

export class TypeScriptAdapter {
  readonly extensions = [".ts", ".tsx", ".js", ".jsx", ".mjs", ".cjs"];

  async parse(file: FileEntry): Promise<ParsedFile> {
    const project = new Project({
      useInMemoryFileSystem: false,
      compilerOptions: { allowJs: true, jsx: 1 },
    });

    let sf: SourceFile;
    try {
      sf = project.addSourceFileAtPath(file.absPath);
    } catch {
      return { filePath: file.path, language: "typescript", symbols: [], edges: [], imports: [] };
    }

    const symbols = this.extractSymbols(sf, file.path);
    const imports = this.extractImports(sf);
    const edges = this.extractEdges(sf, symbols);

    return { filePath: file.path, language: "typescript", symbols, edges, imports };
  }

  private extractSymbols(sf: SourceFile, filePath: string): ParsedSymbol[] {
    const results: ParsedSymbol[] = [];

    // Exported functions
    for (const fn of sf.getFunctions()) {
      if (!fn.isExported()) continue;
      results.push({
        name: fn.getName() ?? "<anonymous>",
        kind: "function",
        signature: fn.getSignature()?.getDeclaration()?.getText()?.split("{")[0]?.trim() ?? null,
        docstring: this.getDocstring(fn),
        lineStart: fn.getStartLineNumber(),
        lineEnd: fn.getEndLineNumber(),
        isExported: true,
      });
    }

    // Exported classes
    for (const cls of sf.getClasses()) {
      if (!cls.isExported()) continue;
      results.push({
        name: cls.getName() ?? "<anonymous>",
        kind: "class",
        signature: `class ${cls.getName()}`,
        docstring: this.getDocstring(cls),
        lineStart: cls.getStartLineNumber(),
        lineEnd: cls.getEndLineNumber(),
        isExported: true,
      });
    }

    // Exported interfaces
    for (const iface of sf.getInterfaces()) {
      if (!iface.isExported()) continue;
      results.push({
        name: iface.getName(),
        kind: "interface",
        signature: `interface ${iface.getName()}`,
        docstring: this.getDocstring(iface),
        lineStart: iface.getStartLineNumber(),
        lineEnd: iface.getEndLineNumber(),
        isExported: true,
      });
    }

    // Exported type aliases
    for (const ta of sf.getTypeAliases()) {
      if (!ta.isExported()) continue;
      results.push({
        name: ta.getName(),
        kind: "type",
        signature: `type ${ta.getName()} = ${ta.getTypeNode()?.getText() ?? "unknown"}`,
        docstring: this.getDocstring(ta),
        lineStart: ta.getStartLineNumber(),
        lineEnd: ta.getEndLineNumber(),
        isExported: true,
      });
    }

    // Exported enums
    for (const en of sf.getEnums()) {
      if (!en.isExported()) continue;
      results.push({
        name: en.getName(),
        kind: "enum",
        signature: `enum ${en.getName()}`,
        docstring: this.getDocstring(en),
        lineStart: en.getStartLineNumber(),
        lineEnd: en.getEndLineNumber(),
        isExported: true,
      });
    }

    // Exported const declarations
    for (const vs of sf.getVariableStatements()) {
      if (!vs.isExported()) continue;
      for (const decl of vs.getDeclarations()) {
        const name = decl.getName();
        results.push({
          name,
          kind: "const",
          signature: `const ${name}`,
          docstring: this.getDocstring(vs),
          lineStart: vs.getStartLineNumber(),
          lineEnd: vs.getEndLineNumber(),
          isExported: true,
        });
      }
    }

    return results;
  }

  private extractImports(sf: SourceFile): ParsedImport[] {
    return sf.getImportDeclarations().map(decl => ({
      targetPath: decl.getModuleSpecifierValue(),
      names: [
        ...decl.getNamedImports().map(n => n.getName()),
        ...(decl.getDefaultImport() ? [decl.getDefaultImport()!.getText()] : []),
      ],
    }));
  }

  private extractEdges(sf: SourceFile, symbols: ParsedSymbol[]): ParsedEdge[] {
    const edges: ParsedEdge[] = [];
    const symbolNames = new Set(symbols.map(s => s.name));

    for (const fn of sf.getFunctions()) {
      if (!fn.isExported()) continue;
      const callerName = fn.getName();
      if (!callerName) continue;

      const calls = fn.getDescendantsOfKind(SyntaxKind.CallExpression);
      for (const call of calls) {
        const expr = call.getExpression();
        const calleeName = expr.getText().split(".").pop() ?? expr.getText();
        if (calleeName && calleeName !== callerName) {
          edges.push({ callerName, calleeName, calleeFile: null });
        }
      }
    }

    return edges;
  }

  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  private getDocstring(node: any): string | null {
    try {
      const jsDoc = node.getJsDocs?.();
      if (jsDoc && jsDoc.length > 0) {
        return jsDoc[0].getDescription().trim() || null;
      }
    } catch {}
    return null;
  }
}
