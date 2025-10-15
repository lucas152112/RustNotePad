# 設計草稿 – 功能 3.1 / Design Draft – Feature 3.1

## 目前範圍（第一階段）/ Current Scope (Milestone 1)
- 聚焦 UTF-8 與 UTF-16（LE/BE，可含 BOM）做為主要幸福路徑。 / Focus on UTF-8 and UTF-16 (LE/BE, optional BOM) documents as the foundational happy paths.
- 內部統一轉為 `\n` 行尾，簡化後續處理。 / Normalise in-memory representation to `\n` to simplify downstream operations.
- 保存原始行尾偏好與 BOM 狀態，以確保回存一致性。 / Track original line ending preference and BOM flag for faithful round-tripping.
- 儲存流程採暫存檔再原子換名，避免部分寫入。 / Persist using write-to-temp plus atomic rename to avoid partial writes.

## 架構 / Architecture
- `crates/core::document::Document`
  - 儲存 `contents`、`LineEnding`、`has_bom` 與 dirty 狀態。 / Stores `contents`, `LineEnding`, `has_bom`, and dirty state.
  - 提供 `open`、`save`、`save_as` 以及行尾/內容/BOM 操作。 / Provides `open`, `save`, `save_as`, and editing helpers (set contents, switch line endings, toggle BOM).
  - 透過掃描原始位元組找到第一個換行符推斷行尾。 / Detects line endings by scanning raw bytes for the first newline sentinel.
  - 流式轉換 CRLF/CR 為 LF，避免額外配置。 / Normalises CRLF/CR to LF using a streaming conversion to avoid extra allocations.
- 錯誤以 `DocumentError`（thiserror）呈現，便於傳遞。 / Error surface expressed via `DocumentError` using `thiserror` for ergonomic propagation.
- 儲存時採 `tmp_rustnotepad` 檔案搭配 `fs::rename`，降低崩潰風險。 / Saving uses a sibling `tmp_rustnotepad` file followed by `fs::rename` to mitigate crash risk.

## 待辦擴充 / Upcoming Additions
- 引入外掛式解碼器與使用者介面，支援更多舊編碼。 / Extend detection to legacy codepages with pluggable decoders and user override UX.
- 建立跨平台檔案監控抽象，偵測外部變更。 / Introduce file monitoring abstraction to alert on external modifications.
- 在 `crates/settings` 實作最近文件與檔案關聯管理。 / Implement recent file list and file association management in `crates/settings`.
- 編碼管線完備後提供 CLI 轉檔入口。 / Expose CLI conversion entry point once the encoding pipeline generalises.

## 決策紀錄 / Decision Log
- 內部儲存維持 LF，載入與儲存時進行轉換，讓編輯器與編碼解耦。 / Keep LF-only internal storage; convert on load/save so editor logic stays encoding-agnostic.
- 顯式追蹤 BOM 以避免使用者期望的 BOM 遺失。 / Track BOM explicitly to avoid losing BOM where users expect it.
- 儲存採原子換名以降低意外中斷造成的檔案損毀。 / Prefer atomic rename saves to reduce corruption risk on crashes.
