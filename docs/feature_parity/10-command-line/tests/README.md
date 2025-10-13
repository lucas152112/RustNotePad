# Test Plan – Feature 3.10

## Unit tests
- Argument parsing permutations
- Conflict and validation rules

## Integration tests
- Launch scenarios with sessions/projects/themes
- Multi-instance vs single-instance enforcement

## E2E scenarios
- Shell integration tests on Windows/macOS/Linux
- CLI automation for file opening and encoding selection

## Tooling
- `cargo test --package cmdline`
- Cross-platform CI scripts under `scripts/ci`
