# Compatibility Notes – Feature 3.1

Use this document to log any intentional deviations from Notepad++ v8.8.6 behaviour and their justification.

## Known differences
- Only UTF-8 (+ optional BOM) files are supported in Milestone 1, whereas Notepad++ allows many legacy encodings.
- File monitoring, recent file prompts, and file association handling are not yet implemented.

## Validation checklist
- [ ] Behaviour verified on Windows 10/11
- [ ] Behaviour verified on Ubuntu LTS
- [ ] Behaviour verified on macOS (Intel & Apple Silicon)
- [ ] Encoding conversions validated against Notepad++ reference cases
