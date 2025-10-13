# Test Plan – Feature 3.5

## Unit tests
- Grammar loader success/failure cases
- Theme parser validation
- UDL schema serialization

## Integration tests
- Highlight diffs between standard and custom themes
- UDL import/export round trip from Notepad++ XML
- Folding behaviour across languages

## E2E scenarios
- Large document highlighting performance
- Real-time UDL edits with live preview
- Theme editor UI regression

## Tooling
- `cargo test --package highlight`
- Snapshot tests for syntax highlighting
