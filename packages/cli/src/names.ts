const ADJECTIVES = [
  "wild", "blue", "red", "new", "old", "swift", "dark", "bright",
  "cool", "warm", "soft", "bold", "calm", "clear", "deep", "fair",
  "free", "gold", "grey", "high", "jade", "keen", "lean", "lone",
  "mild", "neat", "open", "pale", "pure", "rich", "sage", "tall",
  "true", "vast", "wide", "wise", "young", "zeal",
];

const NOUNS = [
  "cat", "moon", "bird", "river", "ridge", "oak", "pine", "fox",
  "ash", "bay", "cape", "dale", "dune", "fern", "glen", "hawk",
  "isle", "lake", "lark", "leaf", "mead", "mist", "path", "peak",
  "pool", "reed", "rock", "rose", "rune", "sand", "seed", "star",
  "tide", "vale", "wave", "well", "wind", "wren",
];

export function generateRunName(): string {
  const adj = ADJECTIVES[Math.floor(Math.random() * ADJECTIVES.length)];
  const noun = NOUNS[Math.floor(Math.random() * NOUNS.length)];
  return `${adj}-${noun}`;
}
