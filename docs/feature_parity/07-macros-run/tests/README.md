# Test Plan – Feature 3.7

## Unit tests
- Macro serialization/deserialization
- Repeat count handling
- Command dispatch integrity

## Integration tests
- External tool execution sandbox
- Environment variable propagation
- Macro persistence across restarts

## E2E scenarios
- Record, save, and replay complex macro
- Run menu output capture
- Macro + run combination sequences

## Tooling
- `cargo test --package macros`
- `cargo test --package runexec`
- GUI automation for macro recorder UI
