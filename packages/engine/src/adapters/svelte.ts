import { Project } from "ts-morph";
import { basename, extname } from "node:path";
import { readFile } from "node:fs/promises";
import { TypeScriptAdapter } from "./typescript.js";
import type { FileEntry, ParsedFile } from "@sensei/shared";

/**
 * Parses Svelte single-file components by extracting the <script> block and
 * running it through the TypeScript adapter. The component itself is added as
 * a "component" symbol so it appears in the graph even when there is no script.
 */
export class SvelteAdapter extends TypeScriptAdapter {
  override readonly extensions = [".svelte"];

  override async parse(file: FileEntry): Promise<ParsedFile> {
    let source: string;
    try {
      source = await readFile(file.absPath, "utf-8");
    } catch {
      return { filePath: file.path, language: "svelte", symbols: [], edges: [], imports: [] };
    }

    const componentName = basename(file.path, extname(file.path));
    const totalLines = source.split("\n").length;

    // Match the first <script> block (ignoring <script context="module">)
    const scriptMatch = source.match(/<script([^>]*)>([\s\S]*?)<\/script>/);
    if (!scriptMatch || scriptMatch.index === undefined) {
      return {
        filePath: file.path,
        language: "svelte",
        symbols: [{
          name: componentName,
          kind: "component",
          signature: `<${componentName} />`,
          docstring: null,
          lineStart: 1,
          lineEnd: totalLines,
          isExported: true,
        }],
        edges: [],
        imports: [],
      };
    }

    const scriptAttrs = scriptMatch[1];
    const scriptContent = scriptMatch[2];
    const isTS = /lang\s*=\s*["']ts["']/.test(scriptAttrs);

    // Compute line offset: how many newlines before the closing `>` of the opening tag
    const openTagEnd = scriptMatch.index + scriptMatch[0].indexOf(">") + 1;
    const lineOffset = (source.slice(0, openTagEnd).match(/\n/g) ?? []).length;

    // Parse the script block using an in-memory ts-morph project
    const project = new Project({
      useInMemoryFileSystem: true,
      compilerOptions: { allowJs: true, jsx: 1 },
    });
    const virtualExt = isTS ? ".ts" : ".js";
    const sf = project.createSourceFile(
      file.path.replace(/\.svelte$/, virtualExt),
      scriptContent,
    );

    const rawSymbols = this.extractSymbols(sf, file.path);
    const imports = this.extractImports(sf);
    const edges = this.extractEdges(sf, rawSymbols);

    // Shift line numbers by the offset so they point to positions in the .svelte file
    const symbols = rawSymbols.map(s => ({
      ...s,
      lineStart: s.lineStart + lineOffset,
      lineEnd: s.lineEnd + lineOffset,
    }));

    // Prepend the component itself as the primary symbol
    symbols.unshift({
      name: componentName,
      kind: "component" as const,
      signature: `<${componentName} />`,
      docstring: null,
      lineStart: 1,
      lineEnd: totalLines,
      isExported: true,
    });

    return { filePath: file.path, language: "svelte", symbols, edges, imports };
  }
}
