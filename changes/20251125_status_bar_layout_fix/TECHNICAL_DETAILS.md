# 技術細節說明

## egui 布局系統原理

### 面板類型與渲染順序

egui 提供了幾種主要的面板類型：

1. **TopBottomPanel**：固定在頂部或底部
2. **SidePanel**：固定在左側或右側
3. **CentralPanel**：佔用所有剩餘空間

### Z-軸層級規則

在 egui 中，UI 元素的渲染順序決定了它們的 Z-軸層級：
- **後渲染的元素會顯示在上層**
- CentralPanel 會自動佔用所有未被其他面板使用的空間

### 本次修復的關鍵

#### 問題場景
```
渲染順序（錯誤）：
1. TopBottomPanel::top (選單列)    ← 第一層
2. TopBottomPanel::top (工具列)    ← 第二層
3. TopBottomPanel::bottom (狀態列) ← 第三層  ❌
4. CentralPanel (編輯區)           ← 第四層，會覆蓋狀態列

結果：編輯區覆蓋狀態列
```

#### 修復後
```
渲染順序（正確）：
1. TopBottomPanel::top (選單列)    ← 第一層
2. TopBottomPanel::top (工具列)    ← 第二層
3. CentralPanel (編輯區)           ← 第三層
4. TopBottomPanel::bottom (狀態列) ← 第四層  ✅ 最上層

結果：狀態列始終可見
```

## 代碼分析

### show_status_bar 實現

```rust
fn show_status_bar(&mut self, ctx: &egui::Context) {
    egui::TopBottomPanel::bottom("status_bar")
        .resizable(false)              // 不可調整大小
        .exact_height(24.0)            // 固定高度 24px
        .frame(egui::Frame::none()
            .inner_margin(Margin::same(0.0)))  // 無內邊距
        .show(ctx, |ui| {
            self.render_status_bar_row(ui);
        });
}
```

**關鍵配置**：
- `bottom("status_bar")`：指定為底部面板，ID 為 "status_bar"
- `resizable(false)`：禁止用戶調整大小
- `exact_height(24.0)`：固定高度，確保布局一致

### show_editor_area 實現

```rust
fn show_editor_area(&mut self, ctx: &egui::Context) {
    egui::CentralPanel::default().show(ctx, |ui| {
        // 編輯區內容
        // ...
    });
}
```

**關鍵特性**：
- `CentralPanel::default()`：使用默認中央面板
- 自動佔用所有剩餘空間
- 必須在所有固定面板之後渲染（如果要讓固定面板可見）

## 為什麼這個順序很重要？

### egui 的空間分配機制

1. **第一階段**：固定面板（TopBottomPanel、SidePanel）先聲明它們需要的空間
2. **第二階段**：CentralPanel 佔用剩餘的所有空間
3. **渲染階段**：按照代碼調用順序渲染

### 錯誤順序的問題

```rust
// 錯誤示例
self.show_status_bar(ctx);    // 聲明需要底部 24px
self.show_editor_area(ctx);   // 佔用剩餘空間，但會重疊到狀態列

// 問題：CentralPanel 的渲染會覆蓋之前渲染的 TopBottomPanel
```

### 正確順序的原理

```rust
// 正確示例
self.show_editor_area(ctx);   // 先佔用剩餘空間
self.show_status_bar(ctx);    // 後渲染，確保在最上層

// 優點：狀態列在最後渲染，不會被任何元素覆蓋
```

## 相關 egui API

### TopBottomPanel 配置選項

```rust
pub struct TopBottomPanel {
    pub fn top(id: impl Into<Id>) -> Self
    pub fn bottom(id: impl Into<Id>) -> Self
    
    pub fn resizable(mut self, resizable: bool) -> Self
    pub fn min_height(mut self, height: f32) -> Self
    pub fn max_height(mut self, height: f32) -> Self
    pub fn exact_height(mut self, height: f32) -> Self
    pub fn default_height(mut self, height: f32) -> Self
    
    pub fn frame(mut self, frame: Frame) -> Self
    pub fn show(self, ctx: &Context, add_contents: impl FnOnce(&mut Ui))
}
```

### CentralPanel 特性

```rust
pub struct CentralPanel {
    pub fn default() -> Self
    
    pub fn frame(mut self, frame: Frame) -> Self
    pub fn show(self, ctx: &Context, add_contents: impl FnOnce(&mut Ui))
}
```

**重要**：CentralPanel 沒有大小配置，因為它總是佔用剩餘空間。

## 測試驗證

### 單元測試覆蓋

現有的單元測試已經覆蓋了狀態列的基本功能：

```rust
#[test]
fn macro_insert_text_updates_editor_state() {
    // ...
    assert!(
        app.status.lines >= 2,
        "status bar should reflect multi-line content"
    );
    // ...
}
```

這個測試驗證了：
- 狀態列能正確反映多行內容
- 狀態列的 `lines` 欄位正確更新

### 布局測試策略

雖然很難在單元測試中驗證 UI 布局，但可以通過以下方式間接驗證：

1. **代碼審查**：檢查渲染順序
2. **編譯測試**：確保沒有語法錯誤
3. **手動測試**：視覺確認狀態列位置
4. **E2E 測試**：使用 WebDriver 驗證元素可見性

## 性能影響

### 渲染性能

**修改前後的性能影響**：
- ✅ 無額外的渲染開銷
- ✅ 只是調整了渲染順序，不增加渲染次數
- ✅ 狀態列仍然是固定高度，不影響布局計算

### 內存影響

- ✅ 無額外的內存分配
- ✅ 不改變數據結構
- ✅ 只是調整函數調用順序

## 潛在的副作用

### 已檢查的潛在問題

1. **其他面板的顯示**：✅ 不影響
   - 選單列、工具列、側邊欄的渲染順序未改變
   - 只調整了狀態列和編輯區的相對順序

2. **事件處理**：✅ 不影響
   - egui 的事件處理與渲染順序無關
   - 滑鼠事件、鍵盤事件仍然正常工作

3. **焦點管理**：✅ 不影響
   - 編輯區的焦點管理獨立於渲染順序
   - 狀態列不接受焦點，不影響焦點流

4. **主題樣式**：✅ 不影響
   - 狀態列的顏色、字體等樣式不變
   - 只是確保它在正確的層級上

## egui 版本兼容性

本修復基於 egui 的標準 API，不使用任何實驗性功能：

- ✅ 兼容 egui 0.20+
- ✅ 兼容 eframe 0.20+
- ✅ 不依賴特定平台

## 最佳實踐建議

### UI 布局順序的一般原則

```rust
impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        // 1. 頂部固定面板
        egui::TopBottomPanel::top("menu").show(ctx, |ui| { /* ... */ });
        egui::TopBottomPanel::top("toolbar").show(ctx, |ui| { /* ... */ });
        
        // 2. 側邊固定面板
        egui::SidePanel::left("sidebar").show(ctx, |ui| { /* ... */ });
        egui::SidePanel::right("properties").show(ctx, |ui| { /* ... */ });
        
        // 3. 其他底部面板（如果需要在中央面板下層）
        egui::TopBottomPanel::bottom("output").show(ctx, |ui| { /* ... */ });
        
        // 4. 中央面板（佔用剩餘空間）
        egui::CentralPanel::default().show(ctx, |ui| { /* ... */ });
        
        // 5. 最重要的固定面板（確保在最上層）
        egui::TopBottomPanel::bottom("status").show(ctx, |ui| { /* ... */ });
        
        // 6. 浮動窗口和對話框
        egui::Window::new("Settings").show(ctx, |ui| { /* ... */ });
    }
}
```

### 關鍵原則

1. **CentralPanel 應該在大多數固定面板之後**
2. **最重要的 UI 元素應該最後渲染**
3. **保持一致的渲染順序，避免閃爍**

## 參考資料

- [egui 官方文檔](https://docs.rs/egui/)
- [egui GitHub 倉庫](https://github.com/emilk/egui)
- [eframe 示例](https://github.com/emilk/egui/tree/master/examples)

## 修改歷史

| 日期 | 版本 | 說明 |
|------|------|------|
| 2025-11-25 | 1.0 | 初始版本，修復狀態列被覆蓋問題 |
