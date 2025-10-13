# Test Plan – Feature 3.8

## Unit tests
- Session serializer/deserializer
- Project filter matching
- Workspace metadata operations

## Integration tests
- Session restore with split views and caret positions
- Cross-project search index coherence
- Workspace switch persistence

## E2E scenarios
- Multi-project workflow switching
- Project panel drag/drop interactions
- Session auto-save and crash recovery

## Tooling
- `cargo test --package project`
- GUI automation for session/project UI
