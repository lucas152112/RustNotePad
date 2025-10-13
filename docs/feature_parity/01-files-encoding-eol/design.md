# Design Draft – Feature 3.1

## Current scope (Milestone 1)
- Focus on UTF-8 documents (with optional BOM) as the foundational happy path.
- Normalise in-memory representation to `\n` to simplify downstream text operations.
- Track original line-ending preference and BOM flag for faithful round-tripping.
- Persist using a write-to-temp + atomic rename flow to avoid partial writes.

## Architecture
- `crates/core::document::Document`
  - Stores `contents`, `LineEnding`, `has_bom`, and dirty state.
  - Provides `open`, `save`, `save_as`, and editing helpers (set contents, switch line endings, toggle BOM).
  - Detects line endings by scanning the raw byte stream for the first newline sentinel.
  - Normalises any CRLF/CR to LF using a streaming conversion to avoid intermediate allocations.
- Error surface expressed via `DocumentError` using `thiserror` for ergonomic propagation.
- Saving uses a sibling `tmp_rustnotepad` file followed by `fs::rename` to guard against crashes mid-write.

## Upcoming additions
- Extend encoding detection (UTF-16, legacy codepages) with pluggable decoders.
- Introduce file monitoring abstraction to alert on external modifications.
- Implement recent files list + file association management in `crates/settings`.
- Expose CLI conversion entry point once encoding pipeline is generalised.

## Decision log
- In-memory representation will remain LF-only; conversions happen on save/load. This keeps editor logic encoding-agnostic.
- BOM flag is tracked explicitly to avoid lossy round-tripping where users expect BOM preservation.
- Atomic save via temp file is preferred over in-place overwrite to reduce corruption risk during crashes.
