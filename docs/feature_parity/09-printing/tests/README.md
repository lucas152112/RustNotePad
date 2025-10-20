# Test Plan – Feature 3.9（測試計畫 – 功能 3.9）

## Unit tests / 單元測試
- Pagination calculations  
  分頁計算
- Header/footer templating  
  頁首/頁尾模板
- Syntax colouring translation to print styles  
  語法色彩轉換為列印樣式

## Integration tests / 整合測試
- PDF output comparison snapshots  
  PDF 輸出快照比較
- Printer driver selection and fallback  
  列印驅動程式選取與回退
- Preview zoom/margin adjustments  
  預覽縮放與邊界調整

## E2E scenarios / 端到端情境
- Multi-page document print workflow  
  多頁文件列印流程
- Print to PDF across OS targets  
  跨平台列印為 PDF
- Cancel/resume print jobs  
  列印工作取消/繼續

## Tooling / 測試工具
- `cargo test --package printing`  
  `cargo test --package printing`
- Snapshot comparisons via reference PDFs  
  透過參考 PDF 進行快照比對
