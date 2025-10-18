# Tutorial – Feature 3.3 (Search & Replace)

This walkthrough illustrates how to exercise the new search stack through the CLI and the core APIs.  
It mirrors the Notepad++ “Find” and “Replace” workflows, including bookmarks and multi-file traversal.

## 1. Command-line search

```bash
# Search the current directory (recursively) for "todo"
rustnotepad-cli search "todo"

# Regex search with whole-word matching and dot-all semantics
rustnotepad-cli search "fn\\s+(\\w+)" src --regex --whole-word --case-sensitive --dot-matches-newline

# Replace without writing to disk (dry run)
rustnotepad-cli search "TODO" src --replace "DONE"

# Replace and persist the changes
rustnotepad-cli search "API_KEY" configs --replace "<redacted>" --apply
```

The output mirrors the Notepad++ “Find Results” pane:

```
Search "todo" (3 hits in 2 files)
  src/lib.rs (2 hits)
    Line 42 (Col 5): // todo: handle edge cases
    Line 88 (Col 12): let todo_item = ...
  README.md (1 hits)
    Line 10 (Col 1): - [ ] todo: write usage guide
```

## 2. Programmatic workflow with `SearchSession`

```rust
use rustnotepad_core::{Document, SearchSession};
use rustnotepad_search::{SearchOptions, SearchScope};

let mut doc = Document::open("examples/sample.txt")?;

let mut options = SearchOptions::new("error");
options.case_sensitive = false;

let mut session = SearchSession::new(options)?;
session.refresh(&doc)?; // populate matches

// Navigate results
let first = session.find_next().unwrap();
println!("First match at line {}", first.line);

// Restrict to a subsection (e.g., the "Warnings" chapter)
if let Some(start) = doc.contents().find("Warnings") {
    let end = doc.contents()[start..]
        .find("\n\n")
        .map(|delta| start + delta)
        .unwrap_or_else(|| doc.contents().len());
    session.set_selection_scope(start, end);
    session.refresh(&doc)?;
}

// Replace all hits in the current scope
let replacements = session.replace_all("warning", &mut doc)?;
println!("Updated {} occurrences", replacements);

// Bookmark every hit
let mut bookmarks = rustnotepad_core::BookmarkManager::default();
session.mark_all(&mut bookmarks);
```

Key points:

- `SearchSession::refresh` is the entry point for recomputing matches whenever the document changes.
- `find_next` / `find_previous` honour wrap-around rules and selection-only scopes.
- `replace_current` and `replace_all` update the `Document`, preserving encoding/EOL metadata.
- `mark_all` / `mark_current` integrate with `BookmarkManager`, so GUI components can toggle bookmarks consistently.
- `report()` feeds `SearchReport` objects directly into result panels or telemetry.

## 3. Multi-file orchestration

For custom tools (e.g., plugins, scripts), reuse `search_in_files`:

```rust
use rustnotepad_search::{search_in_files, FileSearchInput, SearchOptions};

let inputs = vec![
    FileSearchInput::new("src/lib.rs", std::fs::read_to_string("src/lib.rs")?),
    FileSearchInput::new("src/main.rs", std::fs::read_to_string("src/main.rs")?),
];

let options = SearchOptions::new("unsafe");
let report = search_in_files(inputs, &options)?;

for entry in report.results {
    println!("{} — {} hits", entry.path.unwrap().display(), entry.matches.len());
}
```

`SearchReport::search_in_results` allows chaining secondary filters (“Find in Find results”), and the new CLI output mirrors the same shape for easy inspection.
