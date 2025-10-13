# Test Plan – Feature 3.2

## Unit tests
- Command dispatch (multi-caret operations)
- Folding state transitions
- Line operation utilities (trim, sort, dedup)

## Integration tests
- Split view synchronization
- Bookmark persistence across sessions
- Crash-safe save with simulated failures

## E2E scenarios
- Large file editing responsiveness (60 FPS target)
- Column selection editing with IME input
- Multi-instance conflict resolution validation

## Tooling
- `cargo test --package core`
- GUI automation via Playwright/Tauri harness
