# Test Plan – Feature 3.12

## Unit tests
- Localization loader/parsing with plural rules
- Theme conversion utilities
- Preference schema validation

## Integration tests
- Runtime locale switch with live UI
- Theme import/export cross-check
- Preference sync between GUI and config files

## E2E scenarios
- Language installer workflow
- Theme editor full cycle (create/edit/share)
- Preference import/export via UI

## Tooling
- `cargo test --package settings`
- Localization snapshot tests
- GUI automation for preference dialogs
