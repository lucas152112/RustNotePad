# Test Plan – Feature 3.11

## Unit tests
- Message translation logic
- WASM host capability enforcement
- Plugin metadata validation

## Integration tests
- Load/unload cycles for DLL & WASM plugins
- Plugin admin operations (install/update/remove)
- Sandbox permission enforcement

## E2E scenarios
- Install and run sample DLL plugin on Windows
- Install and run sample WASM plugins across platforms
- Plugin update with signature verification failure

## Tooling
- `cargo test --package plugin_winabi`
- `cargo test --package plugin_wasm`
- Automated plugin harness scripts under `scripts/dev`
