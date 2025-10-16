# Design Draft – Feature 3.2

## 1. Text Buffer Model
- 第一階段採用簡化的 `String` 緩衝（`EditorBuffer`），搭配 byte index 追蹤。 / Initial milestone relies on a simplified `String` buffer (`EditorBuffer`) with byte-index tracking.
- 後續將以 rope/piece-table 取代，確保巨檔編輯效率；介面會保持穩定以利替換。 / The API is designed so we can swap in a rope/piece table implementation later for large-file performance without rewriting callers.
- 緩衝維護 `Vec<Caret>`，每個游標含 position 與（選擇性）選取範圍。 / The buffer maintains a `Vec<Caret>` with position plus optional `Selection` range per caret.

## 2. Multi-Caret Editing Semantics
- 所有編輯操作皆以「取代範圍」抽象表述，透過 `apply_replacements` 同步至緩衝。 / All editing primitives are expressed as replacement plans and fed through `apply_replacements`.
- 取代規劃先以原始字串計算，再依起點排序，逐一套入並追蹤 offset，確保 byte index 在多重插入/刪除時仍成立。 / Replacement plans are computed against the pre-edit snapshot, sorted by start offset, then applied while tracking cumulative offsets so indices remain correct even when edits change string length.
- 游標在編輯後都會清除選取並落在插入點尾端，以模擬 Notepad++ 的預設行為。 / After each edit, carets collapse to the new insertion point (selection cleared) mirroring Notepad++ defaults.

## 3. Planned Editing Operations
- **Backspace/Delete**：已完成 backspace，前向刪除將採同一框架。 / Backspace implemented; forward-delete will reuse the same framework.
- **行操作**：修剪行尾空白、排序、去重、縮排行等會以批次取代方式實作，並同步更新游標 offset。 / Line ops (trim trailing whitespace, sort, dedupe, indent) will leverage batch replacements while adjusting caret offsets.
- **大小寫轉換**：針對選取範圍（無選取時整份文件），使用 Rust Unicode case folding。 / Case conversion will operate on selections or whole document using Rust's Unicode folding helpers.
- **矩形/多列選取**：後續增添 `ColumnBlock` 描述 rectangular spans，並在 `apply_replacements` 中拆解為逐行 replace。 / Column editing will extend the data model with a `ColumnBlock` primitive that explodes into per-line replacements.

## 4. Undo / Redo & Macro Hooks (Preview)
- `apply_replacements` 未來將回傳 `EditRecord`（含舊字串片段），給 undo/redo 與巨集記錄使用。 / The replacement engine will emit `EditRecord` snapshots so undo/redo and macro recording can replay operations.
- 巨集系統會在呼叫前後記錄 caret 集合與文字替換，確保多游標操作可回放。 / Macro recorder will capture caret states plus replacement payloads to faithfully replay multi-caret edits.

## 5. Crash-Safe Save Pipeline
- `Document::save_as` 已採暫存檔 + rename；後續會將編輯 buffer 與 Document 類型串接，確保儲存前同步內容。 / The existing atomic save flow (`Document::save_as`) will integrate with the editing buffer so in-memory changes flush safely before snapshotting.
- 儲存失敗時保留快照並阻擋緩衝區清除 dirty flag。 / On save failure we will keep the buffer marked dirty and persist recovery snapshots.

## 6. Implementation Status Snapshot (2024-Q2)
- `editor.rs` 現已支援 forward delete、換行插入、批次取代與 caret 驗證。 / `editor.rs` now exposes forward delete, newline insertion, batch replacements, and stricter caret validation.
- `line_ops.rs` 聚合行層級指令（縮排、去重、排序、大小寫轉換、修剪尾端空白）。 / `line_ops.rs` collects line-level commands (indent/outdent, dedupe, sort, case conversion, trailing-whitespace trim).
- `column_ops.rs` 實作矩形選取/貼上並自動補齊不足欄位。 / `column_ops.rs` implements rectangular selection edits with automatic padding for ragged lines.
- 書籤與折疊狀態分離為 `bookmarks.rs` 與 `folding.rs`，可供 GUI/Session 模組直接串接。 / Bookmark & folding state are encapsulated in `bookmarks.rs` and `folding.rs` for direct GUI/session integration.
- `split_view.rs` 定義雙面板與多執行個體策略，提供 tabs clone/move/detach API。 / `split_view.rs` models dual-pane & multi-instance strategy with tab clone/move/detach APIs.
- `document_map.rs` 提供 minimap/統計資料，供文件地圖、狀態列與性能監控使用。 / `document_map.rs` generates minimap slices and metrics for document map, status bar, and perf instrumentation.

## Decision log
- 以 `apply_replacements` 統一所有編輯命令，避免重複處理 index 修正。 / Normalise all editing commands through `apply_replacements` to centralise index shifting logic.
- 保持 `EditorBuffer` API 不依賴具體緩衝實作，為未來 rope/piece-table 替換預留空間。 / Keep the `EditorBuffer` API detached from the backing store to ease future rope/piece-table migration.
