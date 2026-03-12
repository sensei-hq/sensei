export interface GapEntry {
  pattern: string;
  count: number;
  suggested_tool: string;
}

/**
 * Maps bash command prefix patterns to the sensei tool that could replace them.
 * Keys are regex strings matched against the start of bash command strings.
 */
export const BASH_TO_TOOL_PATTERNS: Record<string, string> = {
  "^grep\\s":       "search_index (semantic or full-text)",
  "^rg\\s":         "search_index (semantic or full-text)",
  "^find\\s":       "Glob",
  "^ls\\s|^ls$":    "Glob or LS",
  "^cat\\s":        "Read (or get_file_context)",
  "^head\\s":       "Read",
  "^tail\\s":       "Read",
  "^sed\\s":        "Edit",
  "^awk\\s":        "Edit or Bash (if transforming data)",
  "^curl\\s|^wget\\s": "WebFetch",
};

/**
 * Given a list of raw bash command strings, return gap entries
 * sorted by frequency descending.
 */
export function detectGapPatterns(commands: string[]): GapEntry[] {
  // Count matches per pattern
  const counts = new Map<string, { count: number; suggested_tool: string; pattern: string }>();

  for (const [regexStr, tool] of Object.entries(BASH_TO_TOOL_PATTERNS)) {
    const re = new RegExp(regexStr, "i");
    const matched = commands.filter(c => re.test(c.trim()));
    if (matched.length > 0) {
      const display = regexStr.replace(/^\^/, "").replace(/\\s.*$/, " ...");
      counts.set(regexStr, { count: matched.length, suggested_tool: tool, pattern: display });
    }
  }

  return Array.from(counts.values()).sort((a, b) => b.count - a.count);
}
