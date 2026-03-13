export interface DocSection {
  heading: string;
  level: number;
  startLine: number;
  endLine: number;
  content: string;
  codeRefs: string[];
}

export class MarkdownAdapter {
  readonly extensions = [".md", ".mdx"];

  parse(_filePath: string, content: string): DocSection[] {
    const lines = content.split("\n");
    const sections: DocSection[] = [];
    let current: { heading: string; level: number; startLine: number; contentLines: string[] } | null = null;

    const finalize = (endLine: number) => {
      if (!current) return;
      const text = current.contentLines.join("\n").trim();
      sections.push({
        heading: current.heading,
        level: current.level,
        startLine: current.startLine,
        endLine,
        content: text,
        codeRefs: extractCodeRefs(text),
      });
    };

    for (let i = 0; i < lines.length; i++) {
      const h2 = lines[i].match(/^## (.+)/);
      const h3 = lines[i].match(/^### (.+)/);
      const heading = h2 || h3;

      if (heading) {
        finalize(i);  // end previous at line before this heading
        current = { heading: heading[1].trim(), level: h2 ? 2 : 3, startLine: i + 1, contentLines: [] };
      } else if (current) {
        current.contentLines.push(lines[i]);
      }
    }

    finalize(lines.length);
    return sections;
  }
}

function extractCodeRefs(text: string): string[] {
  const refs = new Set<string>();
  const regex = /`([A-Za-z_$][A-Za-z0-9_$]*(?:\(\))?)`/g;
  let match;
  while ((match = regex.exec(text)) !== null) {
    refs.add(match[1].replace(/\(\)$/, ""));
  }
  return Array.from(refs);
}
