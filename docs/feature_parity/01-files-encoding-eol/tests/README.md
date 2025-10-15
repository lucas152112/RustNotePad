# 測試計畫 – 功能 3.1 / Test Plan – Feature 3.1

說明自動化測試如何涵蓋檔案操作、編碼轉換與 CLI 工作流程。 / Break down automated coverage for file operations, encoding conversions, and CLI workflows.

## 單元測試 / Unit Tests
- 緩衝序列化與反序列化（`document::tests::save_preserves_line_endings_and_bom` 已涵蓋）。 / Buffer serialization/deserialization (covered by `document::tests::save_preserves_line_endings_and_bom`).
- 編碼偵測輔助函式（`open_detects_line_endings_and_normalises_content` 已涵蓋）。 / Encoding detection helpers (covered by `open_detects_line_endings_and_normalises_content`).
- 行尾轉換工具函式。 / Line-ending conversion utilities.

## 整合測試 / Integration Tests
- 不同編碼的開啟 / 儲存往返檢驗。 / File open/save round-trips across encodings.
- 外部修改後的自動重新載入。 / Auto-reload when files change externally.
- CLI 轉檔成功與失敗案例（`apps/cli/tests/convert.rs` 已涵蓋）。 / CLI conversion success/failure cases (covered by `apps/cli/tests/convert.rs`).

## 端對端情境 / E2E Scenarios
- 500MB 以上大檔案開啟與儲存須在 2 秒內完成（SSD）。 / Large file (>500MB) open/save under 2 seconds (SSD assumption).
- 混合編碼工作區需提供自動提示。 / Mixed encoding workspace with automatic prompts.
- CLI 批次轉檔冒煙測試。 / CLI batch conversion smoke test.

## 工具與指令 / Tooling
- `cargo test --package rustnotepad_core`
- `cargo test --package settings`
- `cargo run --bin rustnotepad-cli convert ...`
