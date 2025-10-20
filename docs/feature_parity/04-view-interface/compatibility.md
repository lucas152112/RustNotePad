# Compatibility Notes – Feature 3.4（相容性備註 – 功能 3.4）

## Known differences / 已知差異
- Tab drag/drop and split reordering are mocked; actual docking interactions will follow once integrated with Tauri.  
  分頁拖放與分割窗格重新排列目前僅為模擬，待與 Tauri 整合後才會提供完整停駐互動。
- Toolbar theme selector does not yet import/export Notepad++ `.xml` themes (JSON only in this milestone).  
  工具列主題選擇器尚未支援 Notepad++ `.xml` 主題的匯入/匯出，本階段僅支援 JSON。
- UI language dropdown updates metadata but does not load translation files (strings remain English placeholders).  
  介面語言下拉選單目前只更新後設資料，尚未載入翻譯檔案（字串仍為英文佔位）。
- Document map renders text lines only (no minimap scaling or syntax colour overlays).  
  文件地圖目前僅渲染文字行，未提供縮放或語法色彩覆層。
- CJK glyph coverage relies on a user-provided font. Place `NotoSansTC-Regular.otf` (or another Traditional Chinese font) under `assets/fonts/` to avoid tofu glyphs.  
  CJK 字形覆蓋需使用者自備字體，建議將 `NotoSansTC-Regular.otf`（或其他繁體中文字体）放入 `assets/fonts/` 以避免顯示方塊字。

## Validation checklist / 驗證檢查清單
- [x] Tab pin/lock behaviour mirrored in layout state (pending drag/drop)  
  已確認分頁釘選/鎖定狀態儲存在版面配置（拖放功能仍待完成）
- [ ] Theme import/export compatibility with Notepad++ `.xml`  
  尚未驗證與 Notepad++ `.xml` 主題匯入/匯出的相容性
- [ ] UI translations cross-checked with localisation assets  
  尚未與在地化資源交叉檢查 UI 翻譯
- [ ] Document map zoom/scroll fidelity  
  尚未驗證文件地圖縮放與捲動的精確度
