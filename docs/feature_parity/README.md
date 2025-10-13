# Feature Parity Checklist

This directory tracks implementation progress for the twelve feature pillars listed in `AGENT.md` §3.  
Each subdirectory contains the same high-level structure:

- `README.md`: Scope overview and status checklist.  
- `design.md`: Detailed design notes, architecture decisions, and open questions (to be filled in).  
- `tests/`: Place holders for unit, integration, and end-to-end test specifications.  
- `compatibility.md`: Deviations from Notepad++ behaviour and mitigation notes.

> **Process expectation**: A feature is considered complete only after the design document, automated tests, end-to-end validation scripts, and compatibility notes are in place.

## Directory Map

1. `01-files-encoding-eol` — File lifecycle, encodings, and line endings  
2. `02-editing` — Core editing capabilities  
3. `03-search-replace` — Search and replace workflows  
4. `04-view-interface` — View management and UI chrome  
5. `05-syntax-highlighting-udl` — Syntax highlighting and user-defined languages  
6. `06-autocomplete-function-list` — Auto-completion and function list  
7. `07-macros-run` — Macros and external run commands  
8. `08-sessions-projects-workspaces` — Sessions, projects, and workspace handling  
9. `09-printing` — Printing and preview pipeline  
10. `10-command-line` — Command-line arguments and automation  
11. `11-plugins` — Plugin systems (Windows ABI compatibility and WASM host)  
12. `12-localization-preferences` — Localization, themes, and preference management
