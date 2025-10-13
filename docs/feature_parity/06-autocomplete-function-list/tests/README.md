# Test Plan – Feature 3.6

## Unit tests
- Completion ranking heuristics
- Function list parser fixtures per language
- LSP message handling parsers

## Integration tests
- Mixed-language workspace with UDL fallback
- LSP reconnect and diagnostics flow
- Completion latency under load

## E2E scenarios
- User toggles between built-in and LSP completion
- Function list navigation in large file
- Offline editing without LSP

## Tooling
- `cargo test --package autocomplete`
- `cargo test --package function_list`
- Mock LSP server harness
