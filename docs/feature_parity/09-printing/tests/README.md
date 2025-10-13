# Test Plan – Feature 3.9

## Unit tests
- Pagination calculations
- Header/footer templating
- Syntax colouring translation to print styles

## Integration tests
- PDF output comparison snapshots
- Printer driver selection and fallback
- Preview zoom/margin adjustments

## E2E scenarios
- Multi-page document print workflow
- Print to PDF across OS targets
- Cancel/resume print jobs

## Tooling
- `cargo test --package printing`
- Snapshot comparisons via reference PDFs
