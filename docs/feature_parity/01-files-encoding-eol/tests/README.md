# Test Plan – Feature 3.1

Break down automated coverage for file operations, encoding conversions, and CLI workflows.

## Unit tests
- Buffer serialization/deserialization *(implemented via `document::tests::save_preserves_line_endings_and_bom`)*
- Encoding detection helpers *(covered by `open_detects_line_endings_and_normalises_content`)*
- Line-ending conversion utilities

## Integration tests
- File open/save round-trips across encodings
- Auto-reload upon external modification
- CLI conversion (success & failure cases)

## E2E scenarios
- Large file (>500MB) open and save under 2s (SSD assumption)
- Mixed encoding workspace with automatic prompts
- CLI batch conversion smoke test

## Tooling
- `cargo test --package rustnotepad_core`
- `cargo test --package settings`
- `cargo run --bin rustnotepad-cli convert ...`
