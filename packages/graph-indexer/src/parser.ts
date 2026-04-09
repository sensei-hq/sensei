/**
 * Tagged comment extractor — finds WHY/DECISION/HACK/NOTE/TODO line comments.
 * Full symbol extraction is delegated to TypeScriptAdapter from @sensei/engine.
 */

export interface ParsedComment {
  id: string;
  tag: string;
  text: string;
  line: number;
}

const TAG_RE = /\/\/\s*(WHY|DECISION|HACK|NOTE|TODO):\s*(.+)/;

export function extractTaggedComments(
  filePath: string,
  content: string
): ParsedComment[] {
  const comments: ParsedComment[] = [];
  const lines = content.split("\n");
  for (let i = 0; i < lines.length; i++) {
    const match = TAG_RE.exec(lines[i]);
    if (match) {
      comments.push({
        id: `comment:${filePath}:${i + 1}`,
        tag: match[1],
        text: match[2].trim(),
        line: i + 1,
      });
    }
  }
  return comments;
}


