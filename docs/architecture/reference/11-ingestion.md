# Document Ingestion, Chunking & Embedding Pipeline

## Status

Design doc. No code yet. The proposal here is what we'll argue with before
shipping anything in `crates/sensei-ingest`.

## Motivation

Today sensei indexes code via tree-sitter — files become a typed graph
of language constructs the rest of the system can reason about. The
equivalent doesn't exist for *documents*: PDFs, Office files, slides,
markdown, HTML. If the daemon's knowledge plane is going to embed
arbitrary documents and serve retrieval over them, the naive path —
"read the bytes, split into N-character chunks, embed each chunk" —
loses everything that makes the document useful:

- Structural context: which heading the chunk lives under, which slide
  it comes from, which page or paragraph index.
- Non-text payloads: tables read as run-on text become noise; figures
  / charts / equations carry information that the surrounding prose
  doesn't repeat.
- Hierarchical retrieval: "show me the executive summary of every
  doc in this set" is impossible if the executive summary isn't
  marked as such.
- Citations: "this answer came from §3.2.1, p. 14" requires us to
  carry the path through the document, not just the text.

We need a layer between *bytes-on-disk* and *vectors-in-a-store* that
turns the former into a typed structured document, then derives chunks
from that structure rather than from raw byte offsets.

## Scope

**In scope** for the new crate:

- Loader: bytes → `Document` (a typed, hierarchical tree).
- Chunker: `Document` → `Iterator<Chunk>` with structural metadata
  attached to each chunk.
- Format implementations: at least Markdown, PDF, DOCX, and HTML in
  v1. PPTX, XLSX, and image-PDF OCR scoped as follow-ups.
- Block-typed extraction: text paragraphs, headings, code blocks,
  lists, tables, figures, equations — distinct enough that callers
  can route different block kinds to different downstream paths.
- TOC reconstruction when the format carries one (PDF outline,
  DOCX headings, Markdown ATX headings).

**Out of scope:**

- Embedding itself — that's `gateway-embedded` / the gateway trait.
  The crate produces `Chunk`s; the caller hands them to an
  embedding adapter and stores the resulting vectors.
- Retrieval, ranking, or any vector-store concerns.
- LLM-side reasoning (summarisation, question answering). Those
  consume the structured output but aren't part of ingestion.
- Code parsing — tree-sitter already owns that, with a richer
  language-aware AST than this crate could provide.

## Crate placement

New crate `crates/sensei-ingest` under the existing workspace,
peer to `gateway-embedded`. The gateway has no dependency on it
(the gateway is provider-agnostic); senseid (or any consumer) wires
the two together at the call site.

Reasoning:
- Pure data transformations, easy to test, no native deps in the
  core types. Format-specific extractors can pull heavy deps
  (lopdf, calamine, …) behind cargo features so consumers only
  compile in the formats they use.
- Reusable beyond embedding: a typed `Document` is useful for
  search, display, summarisation, and citation rendering. Coupling
  it to the inference layer would block those other consumers.

## Core types

The proposal — names and field shapes are placeholders, the
hierarchy is the load-bearing decision.

```rust
/// A loaded document: provenance + a tree of structured content.
pub struct Document {
    pub id: DocumentId,
    pub source: DocumentSource,
    pub format: DocumentFormat,
    pub title: Option<String>,
    /// Top-level metadata as the format provides it: author, created,
    /// language, page count, etc. Stays format-loose so loaders can
    /// surface what they actually have.
    pub metadata: HashMap<String, MetadataValue>,
    /// The body — a flat sequence of Blocks. Hierarchy is reconstructed
    /// via Block::heading_path / Block::section_id, not via nesting,
    /// because flat sequences are easier to chunk and iterate.
    pub blocks: Vec<Block>,
}

pub enum DocumentSource {
    File { path: PathBuf, mtime: SystemTime },
    Url(String),
    Inline { name: String, bytes_len: u64 },
}

pub enum DocumentFormat {
    Markdown,
    Pdf,
    Docx,
    Pptx,
    Xlsx,
    Html,
    PlainText,
    /// Loader can name an unrecognised format for diagnostics; chunker
    /// will treat it as PlainText.
    Other(String),
}

/// A unit of content. Variants are intentionally coarse — anything
/// finer (e.g. table-cell-level) belongs inside the variant's payload,
/// not in a new top-level variant.
pub enum Block {
    Heading {
        level: u8,            // 1-6, ATX / DOCX outline level
        text: String,
        anchor: SectionId,    // generated; used by Chunk.section_path
    },
    Paragraph { text: String, span: Span },
    List {
        ordered: bool,
        items: Vec<ListItem>,
    },
    CodeBlock {
        language: Option<String>,
        text: String,
    },
    Table {
        headers: Vec<String>,
        rows: Vec<Vec<String>>,
        /// A markdown-table rendering of the same content. Embedding a
        /// table as serialised text is noisy; embedding the markdown
        /// rendering captures column relationships better.
        rendered_markdown: String,
    },
    Figure {
        /// Optional caption from the source document.
        caption: Option<String>,
        /// Where the underlying image lives — usually a path under a
        /// blob store; the loader doesn't own image storage.
        image_ref: Option<BlobRef>,
        /// Optional alt-text or OCR result from a follow-up pass.
        described: Option<String>,
    },
    Equation { tex: String },
    PageBreak {                // PDF / PPTX page markers
        page_number: u32,
    },
    /// Block we recognised but won't process meaningfully yet —
    /// preserved so chunkers can decide to skip rather than guess.
    Unsupported { kind: String, raw: String },
}

pub struct Span {
    /// Byte offsets in the original source (when available).
    pub start: usize,
    pub end: usize,
    /// Page/slide where this block originated, if the format has pages.
    pub page: Option<u32>,
}

/// Stable id for a heading/section, derived from anchor + path so it's
/// reproducible across re-ingestion of the same document version.
pub struct SectionId(pub String);
```

### The Chunk type

```rust
/// What gets embedded. Carries enough metadata that the retrieval side
/// can re-rank, filter, and cite.
pub struct Chunk {
    pub document_id: DocumentId,
    /// Stable id derived from document_id + section_path + ordinal so a
    /// re-ingest of the same content produces the same chunk id.
    pub chunk_id: ChunkId,
    pub content: String,
    /// What kind of block(s) this chunk came from. Tables, code, and
    /// prose probably want different embedding models / strategies.
    pub kind: ChunkKind,
    /// Heading path through the document: ["3. Architecture",
    /// "3.2 Concurrency", "3.2.1 Locking"]. Built by the chunker by
    /// walking back through the most recent Heading blocks.
    pub section_path: Vec<String>,
    pub span: Span,
    /// Free-form additional metadata from the loader (page numbers,
    /// slide titles, code language tag, etc.).
    pub metadata: HashMap<String, MetadataValue>,
}

pub enum ChunkKind {
    Prose,
    Code,
    Table,
    FigureCaption,
    Equation,
    Mixed,        // chunk spans multiple block kinds (rare, last resort)
}
```

## Traits

```rust
/// Format-specific loader. One impl per format we support.
pub trait Loader: Send + Sync {
    fn format(&self) -> DocumentFormat;
    /// Does this loader recognise these bytes? Sniff-based — cheap
    /// MIME/magic-byte check. Used by the registry to pick a loader
    /// when the caller doesn't know the format.
    fn sniff(&self, bytes: &[u8]) -> bool;
    fn load(&self, source: DocumentSource, bytes: &[u8])
        -> Result<Document, IngestError>;
}

/// Chunking strategy. The default impl is structure-respecting:
/// chunks never cross block boundaries except via explicit merge, and
/// large blocks (long paragraphs, large tables) get split with overlap.
pub trait Chunker: Send + Sync {
    fn chunk(&self, doc: &Document) -> Vec<Chunk>;
}

/// Composes loaders + chunker. The orchestrator most callers use.
pub struct Ingest {
    loaders: Vec<Box<dyn Loader>>,
    chunker: Box<dyn Chunker>,
}

impl Ingest {
    pub fn process(&self, source: DocumentSource, bytes: &[u8])
        -> Result<(Document, Vec<Chunk>), IngestError> {
        let loader = self.pick_loader(bytes, &source)?;
        let doc = loader.load(source, bytes)?;
        let chunks = self.chunker.chunk(&doc);
        Ok((doc, chunks))
    }
}
```

The Loader / Chunker split lets us swap strategies independently —
e.g. a "semantic chunker" that uses sentence embeddings to find
natural break points can replace the default structure-respecting
one without touching the loaders.

## Pipeline shape

```
                 ┌───────────────────┐
   bytes ───►    │   Loader::load    │  ──► Document (Block tree + meta)
                 └───────────────────┘
                          │
                          ▼
                 ┌───────────────────┐
                 │  Chunker::chunk   │  ──► Vec<Chunk>
                 └───────────────────┘
                          │
                          ▼
                 ┌───────────────────┐
                 │  gateway::execute │  ──► Vec<embedding>
                 │   (TextEmbed)     │
                 └───────────────────┘
                          │
                          ▼
                   (vector store)
```

Sensei wires the last two steps; this crate stops at producing
`Vec<Chunk>`. Different ChunkKinds *may* be routed to different
embedding adapters by the caller (e.g. code chunks → a code
embedding model, prose chunks → general text embedding) — that
decision is policy, not part of the ingestion crate.

## Format scope, phased

**Phase 1 — pure-Rust formats with good libraries:**

- Markdown (parse with `pulldown-cmark`; ATX headings → Heading
  blocks, fenced code → CodeBlock, pipe tables → Table)
- HTML (parse with `scraper` / `html5ever`; map to the same Block
  shape, preserving headings/paragraphs/tables/figures)
- Plain text (paragraph splitter, no structure)
- DOCX (parse with `docx-rs` or hand-rolled OPC reader; outline
  levels → Heading blocks, tables → Table, embedded images →
  Figure)

**Phase 2 — PDF (structured-text path):**

- `lopdf` / `pdf-extract` for text-based PDFs
- Reconstruct sections from outline (`/Outlines` object) when
  present; fall back to heuristic heading detection when not
- Tables → best-effort: position-based column detection (cheap),
  or punt to "preserve as text"

**Phase 3 — Image-PDF + OCR:**

- Detect when a PDF page has no extractable text → mark as
  "needs OCR"
- Bring in `tesseract` (FFI to `libtesseract`) for OCR, or shell
  out to a sidecar Python process running `unstructured` /
  `marker` for the hard cases
- This is the format where the Rust ecosystem is weakest, so
  expect either heavy native deps or a Python sidecar

**Phase 4+ — PPTX, XLSX, EPUB:**

- PPTX via OPC + slide-XML parsing; each slide becomes a section,
  text frames → Paragraph blocks, embedded charts → Figure
- XLSX via `calamine`; each sheet becomes a section, tables
  reconstructed from named ranges where present

## Chunking strategy (default impl)

Not "split every 800 characters." The default chunker walks the
`Block` sequence and:

1. Tracks the heading path as it goes (each new Heading pops the
   stack to its level and pushes itself).
2. For each Block, decides whether to emit a chunk:
   - `Heading` → flushes the current accumulator and starts a new
     section context.
   - `Paragraph` → appends to the current accumulator; emits a
     chunk when accumulator exceeds the target token budget.
   - `Table`, `CodeBlock`, `Figure`, `Equation` → emit as
     standalone chunks (don't merge with surrounding prose), with
     their `kind` set so downstream can route them.
3. For prose chunks that grow too large, splits at sentence
   boundaries with a small overlap (configurable, ~50 tokens
   default).
4. Inherits `section_path` and `span` onto every chunk so
   retrieval can reconstruct provenance.

Token-budget-aware splitting needs a tokenizer. The chunker should
take a `Box<dyn Tokenize>` injected by the caller so we don't pin
ourselves to a specific embedding model's tokenizer (BERT vs
SentencePiece vs tiktoken differ).

## Tables and figures — treat them specially

Embedding a table as serialized "col1: val1, col2: val2, ..." text
is noisy. Two practical options:

1. **Render to markdown table, embed the markdown.** The pipe-
   delimited layout captures column relationships; modern embedding
   models understand it well.
2. **Multi-vector per table:** one chunk for the table caption /
   surrounding prose, one chunk for the rendered markdown, one
   chunk per row when rows are independent records. Retrieval then
   sees the most relevant view.

For figures: caption + alt-text + OCR-on-rasterized-image as three
candidate text payloads, all embedded under one `Figure` block.
Optional future addition: image-embedding model for the raster
itself (CLIP / SigLIP), stored alongside the text vectors.

This is exactly why `Chunk.kind` exists — the embedder can route
on it.

## Open questions / decisions to make later

- **Blob storage for figures.** The crate produces `BlobRef`s
  pointing somewhere; who owns the somewhere? Probably the
  consumer (senseid writes them under `~/.sensei/blobs/...` keyed
  by sha256). Worth being explicit before locking the type shape.
- **Versioning.** A document re-ingested after edits should
  produce stable chunk ids for unchanged sections but new ids for
  changed ones. The current `chunk_id = sha256(document_id +
  section_path + ordinal + content_hash)` approach gives that — but
  ordinal shifts break it if a section is inserted earlier.
  Worth designing the id scheme carefully so re-embedding is
  incremental.
- **Streaming vs full document.** Some PDFs are huge (1000+ pages).
  The trait shape is "bytes in, Document out" — for huge inputs we
  may need a streaming `LoadStream` that emits Blocks as they're
  parsed, so chunking can run in parallel.
- **OCR boundary.** Native Tesseract FFI vs Python sidecar vs
  managed service vs deferred-to-user. Each has very different
  packaging implications (binary size, runtime deps).
- **Tokenizer ownership.** Chunker needs a tokenizer to measure
  budget. The same tokenizer should match the embedding adapter
  used downstream. Pass it through, or pull it from the adapter
  itself via a new trait method?

## Why this crate makes more sense than extending `gateway-embedded`

`gateway-embedded` is about *inference engines* (llama.cpp,
fastembed, ort). Document parsing is a fundamentally different
problem with a different dependency profile:

- Inference adapters bring native ML deps (C++, ONNX runtime).
- Ingestion brings parsing deps (lopdf, calamine, OCR libraries).

Coupling them dilutes both. The crate boundary keeps each one
focused on doing one thing well. They meet at the call site, where
sensei (or any other consumer) feeds a `Chunk` iterator from
`sensei-ingest` into a gateway `InferenceRequest::TextEmbed`.

## Concrete first cut, when the time comes

1. Add `crates/sensei-ingest/` with the type definitions above and
   no loaders yet. Build green, types compile, no behaviour.
2. Implement the Markdown loader first — `pulldown-cmark` covers
   it cleanly and gives us end-to-end testing without binary
   deps.
3. Default structure-respecting chunker with a swappable tokenizer.
   Unit tests: heading path tracking, paragraph merging, table /
   code / figure isolation, token-budget split with overlap.
4. Wire `sensei-ingest` into senseid's knowledge plane as a
   pluggable "document ingestion" step that feeds the existing
   embedding flow through `Gateway::execute`. Add an HTTP endpoint
   so the app can submit documents for ingestion.
5. Add HTML + DOCX loaders.
6. Add PDF (structured-text path).
7. Decide OCR boundary based on actual use cases observed by then.

## What this unlocks

Once a `Document` has typed Block payloads + a heading path on
every chunk, several things become possible that aren't today:

- Retrieval with structural filters ("only return code blocks from
  files under the `auth` heading").
- Citation-grade answers ("this came from §3.2.1, page 14 of the
  Q3 strategy deck").
- Hierarchical summaries (section-level summaries computed
  bottom-up).
- Hybrid retrieval (keyword on Block text + vector on Chunk
  embedding), since the structure makes both queryable.

These aren't features of the ingest crate — they're features the
ingest crate makes feasible.
