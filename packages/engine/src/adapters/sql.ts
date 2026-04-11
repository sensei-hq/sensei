import { readFile } from "node:fs/promises";
import type { FileEntry, ParsedFile, ParsedSymbol, ParsedEdge } from "@sensei/shared";

// Patterns for individual object types (case-insensitive, matched against a single line)
const OBJECT_PATTERNS: Array<{ re: RegExp; kind: ParsedSymbol["kind"] }> = [
  { re: /^[ \t]*CREATE\s+TABLE\s+(?:IF\s+NOT\s+EXISTS\s+)?([^\s(;]+)/i,               kind: "class" },
  { re: /^[ \t]*CREATE\s+VIEW\s+(?:IF\s+NOT\s+EXISTS\s+)?([^\s(;]+)/i,                kind: "type" },
  { re: /^[ \t]*CREATE(?:\s+OR\s+REPLACE)?\s+FUNCTION\s+([^\s(;]+)/i,                 kind: "function" },
  { re: /^[ \t]*CREATE(?:\s+OR\s+REPLACE)?\s+PROCEDURE\s+([^\s(;]+)/i,                kind: "function" },
  { re: /^[ \t]*CREATE(?:\s+OR\s+REPLACE)?\s+TRIGGER\s+([^\s(;]+)/i,                  kind: "function" },
  { re: /^[ \t]*CREATE\s+(?:UNIQUE\s+)?INDEX\s+(?:IF\s+NOT\s+EXISTS\s+)?([^\s(;]+)/i, kind: "const" },
  { re: /^[ \t]*CREATE\s+TYPE\s+(?:IF\s+NOT\s+EXISTS\s+)?([^\s(;]+)/i,                kind: "type" },
];

// DML reference patterns used to build edges inside function/procedure bodies
const DML_PATTERNS: RegExp[] = [
  /\bFROM\s+([A-Za-z_][A-Za-z0-9_.]*)/gi,
  /\bJOIN\s+([A-Za-z_][A-Za-z0-9_.]*)/gi,
  /\bINTO\s+([A-Za-z_][A-Za-z0-9_.]*)/gi,
  /\bUPDATE\s+([A-Za-z_][A-Za-z0-9_.]*)/gi,
  /\bDELETE\s+FROM\s+([A-Za-z_][A-Za-z0-9_.]*)/gi,
];

// Common SQL keywords that can appear in positions captured by DML_PATTERNS
const SQL_KEYWORDS = new Set([
  "SELECT", "INSERT", "UPDATE", "DELETE", "WHERE", "SET", "VALUES",
  "TABLE", "VIEW", "INDEX", "FUNCTION", "PROCEDURE", "TRIGGER",
  "DATABASE", "SCHEMA", "SEQUENCE", "AS", "ON", "USING", "WITH",
  "LATERAL", "CROSS", "INNER", "OUTER", "LEFT", "RIGHT", "FULL",
  "NATURAL", "RECURSIVE", "ONLY", "ALL", "DISTINCT", "TOP",
  "DUAL", "NEW", "OLD", "EXCLUDED", "INSERTED", "DELETED",
]);

export class SqlAdapter {
  readonly extensions = [".sql"];

  async parse(file: FileEntry): Promise<ParsedFile> {
    let source: string;
    try {
      source = await readFile(file.absPath, "utf-8");
    } catch {
      return { filePath: file.path, language: "sql", symbols: [], edges: [], imports: [] };
    }

    try {
      const lines = source.split("\n");
      const symbols: ParsedSymbol[] = [];
      const edges: ParsedEdge[] = [];

      let i = 0;
      while (i < lines.length) {
        const match = this.matchCreate(lines, i);
        if (match) {
          const { name, kind, lineStart, lineEnd, signature, docstring, body } = match;
          symbols.push({ name, kind, signature, docstring, lineStart, lineEnd, isExported: true });

          if (kind === "function" && body) {
            edges.push(...this.extractEdges(name, body));
          }

          // Advance past the statement (lineEnd is 1-based, convert back to 0-based)
          i = lineEnd;
        } else {
          i++;
        }
      }

      return { filePath: file.path, language: "sql", symbols, edges, imports: [] };
    } catch {
      return { filePath: file.path, language: "sql", symbols: [], edges: [], imports: [] };
    }
  }

  private matchCreate(
    lines: string[],
    startIdx: number,
  ): {
    name: string;
    kind: ParsedSymbol["kind"];
    lineStart: number;
    lineEnd: number;
    signature: string;
    docstring: string | null;
    body: string | null;
  } | null {
    const line = lines[startIdx];
    if (!line) return null;

    let matchedKind: ParsedSymbol["kind"] | null = null;
    let matchedName: string | null = null;

    for (const { re, kind } of OBJECT_PATTERNS) {
      const m = line.match(re);
      if (m) {
        matchedKind = kind;
        // Strip surrounding quotes if any
        matchedName = m[1].replace(/^["'`]|["'`]$/g, "").trim();
        break;
      }
    }

    if (!matchedKind || !matchedName) return null;

    const docstring = this.extractDocstring(lines, startIdx);

    // Signature: CREATE statement up to first `(` or ` AS `
    const sigMatch = line.match(/^(.*?)(?:\s*\(|\s+AS\b)/i);
    const signature = sigMatch ? sigMatch[1].trim() : line.trim();

    const { lineEnd, body } = this.findStatementEnd(lines, startIdx);

    return {
      name: matchedName,
      kind: matchedKind,
      lineStart: startIdx + 1, // 1-based
      lineEnd: lineEnd + 1,    // 1-based
      signature,
      docstring,
      body,
    };
  }

  private extractDocstring(lines: string[], statementLine: number): string | null {
    const commentLines: string[] = [];
    let i = statementLine - 1;

    while (i >= 0) {
      const trimmed = lines[i].trim();
      if (trimmed.startsWith("--")) {
        commentLines.unshift(trimmed.replace(/^--\s?/, ""));
        i--;
      } else {
        break;
      }
    }

    return commentLines.length > 0 ? commentLines.join("\n") : null;
  }

  private findStatementEnd(
    lines: string[],
    startIdx: number,
  ): { lineEnd: number; body: string | null } {
    const statementLines: string[] = [];
    let inDollarQuote = false;
    let dollarTag = "";

    for (let i = startIdx; i < lines.length; i++) {
      const raw = lines[i];
      statementLines.push(raw);

      if (!inDollarQuote) {
        // Detect opening dollar-quote tag (e.g. $$ or $body$)
        const dollarMatch = raw.match(/(\$[A-Za-z0-9_]*\$)/);
        if (dollarMatch) {
          inDollarQuote = true;
          dollarTag = dollarMatch[1];
        }
      } else {
        // Check if this line closes the dollar-quote block
        const closeIdx = raw.indexOf(dollarTag, raw.startsWith(dollarTag) ? 0 : 1);
        if (closeIdx !== -1) {
          inDollarQuote = false;
        }
        continue;
      }

      if (!inDollarQuote) {
        if (raw.trimEnd().endsWith(";")) {
          return { lineEnd: i, body: statementLines.join("\n") };
        }
        if (/\bEND\s*;?\s*$/i.test(raw)) {
          return { lineEnd: i, body: statementLines.join("\n") };
        }
      }
    }

    return { lineEnd: lines.length - 1, body: statementLines.join("\n") };
  }

  private extractEdges(callerName: string, body: string): ParsedEdge[] {
    const edges: ParsedEdge[] = [];
    const seen = new Set<string>();

    for (const pattern of DML_PATTERNS) {
      pattern.lastIndex = 0;
      let m: RegExpExecArray | null;
      while ((m = pattern.exec(body)) !== null) {
        const calleeName = m[1].trim();
        if (SQL_KEYWORDS.has(calleeName.toUpperCase())) continue;
        const key = `${callerName}:${calleeName}`;
        if (!seen.has(key)) {
          seen.add(key);
          edges.push({ callerName, calleeName, calleeFile: null });
        }
      }
    }

    return edges;
  }
}
