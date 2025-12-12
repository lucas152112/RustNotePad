#!/bin/bash
# 驗證狀態列布局修復
# 日期: 2025-11-25

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
TARGET_FILE="$REPO_ROOT/rustnotepad_gui/src/main.rs"

echo "====================================="
echo "  狀態列布局修復 - 驗證腳本"
echo "====================================="
echo ""

# 檢查文件是否存在
if [ ! -f "$TARGET_FILE" ]; then
    echo "❌ 錯誤：找不到目標文件 $TARGET_FILE"
    exit 1
fi

PASS_COUNT=0
TOTAL_COUNT=6

# 測試 1: 檢查渲染順序
echo "測試 1/6: 檢查渲染順序..."
if grep -A 15 "fn update(&mut self, ctx: &egui::Context, _frame: &mut Frame)" "$TARGET_FILE" | \
   grep "show_editor_area(ctx);" | head -1 | grep -q "show_editor_area" && \
   grep -A 16 "fn update(&mut self, ctx: &egui::Context, _frame: &mut Frame)" "$TARGET_FILE" | \
   grep "show_status_bar(ctx);" | head -1 | grep -q "show_status_bar"; then
    # 驗證 show_editor_area 在 show_status_bar 之前
    editor_line=$(grep -n "show_editor_area(ctx);" "$TARGET_FILE" | grep -v "fn show_editor_area" | head -1 | cut -d: -f1)
    status_line=$(grep -n "show_status_bar(ctx);" "$TARGET_FILE" | grep -v "fn show_status_bar" | head -1 | cut -d: -f1)
    
    if [ "$editor_line" -lt "$status_line" ]; then
        echo "   ✅ 通過：show_editor_area (行 $editor_line) 在 show_status_bar (行 $status_line) 之前"
        ((PASS_COUNT++))
    else
        echo "   ❌ 失敗：渲染順序錯誤"
    fi
else
    echo "   ❌ 失敗：找不到 show_editor_area 或 show_status_bar"
fi

# 測試 2: 檢查狀態列配置
echo ""
echo "測試 2/6: 檢查狀態列使用 TopBottomPanel::bottom..."
if grep -A 5 "fn show_status_bar" "$TARGET_FILE" | grep -q "TopBottomPanel::bottom"; then
    echo "   ✅ 通過：狀態列使用 TopBottomPanel::bottom"
    ((PASS_COUNT++))
else
    echo "   ❌ 失敗：狀態列配置不正確"
fi

# 測試 3: 檢查狀態列高度
echo ""
echo "測試 3/6: 檢查狀態列固定高度..."
if grep -A 5 "fn show_status_bar" "$TARGET_FILE" | grep -q "exact_height(24.0)"; then
    echo "   ✅ 通過：狀態列高度固定為 24px"
    ((PASS_COUNT++))
else
    echo "   ❌ 失敗：狀態列高度配置不正確"
fi

# 測試 4: 檢查編輯區使用 CentralPanel
echo ""
echo "測試 4/6: 檢查編輯區使用 CentralPanel..."
if grep -A 5 "fn show_editor_area" "$TARGET_FILE" | grep -q "CentralPanel::default()"; then
    echo "   ✅ 通過：編輯區使用 CentralPanel"
    ((PASS_COUNT++))
else
    echo "   ❌ 失敗：編輯區配置不正確"
fi

# 測試 5: 編譯檢查
echo ""
echo "測試 5/6: 編譯檢查..."
cd "$REPO_ROOT/rustnotepad_gui"
if cargo build --quiet 2>&1 | grep -qi "error"; then
    echo "   ❌ 失敗：編譯錯誤"
else
    echo "   ✅ 通過：編譯成功"
    ((PASS_COUNT++))
fi

# 測試 6: 檢查執行檔
echo ""
echo "測試 6/6: 檢查執行檔..."
if [ -f "$REPO_ROOT/target/debug/rustnotepad" ]; then
    echo "   ✅ 通過：執行檔存在"
    ((PASS_COUNT++))
else
    echo "   ⚠️  警告：執行檔不存在（可能需要完整編譯）"
fi

# 總結
echo ""
echo "====================================="
echo "  驗證結果"
echo "====================================="
echo ""
echo "通過測試：$PASS_COUNT / $TOTAL_COUNT"
echo ""

if [ "$PASS_COUNT" -eq "$TOTAL_COUNT" ]; then
    echo "🎉 所有測試通過！修復已正確應用。"
    echo ""
    echo "📝 建議的後續步驟："
    echo "   1. 運行單元測試："
    echo "      cd rustnotepad_gui && cargo test"
    echo ""
    echo "   2. 執行手動測試："
    echo "      - 啟動應用程式: ./target/debug/rustnotepad"
    echo "      - 打開新文件並驗證狀態列顯示正確"
    echo "      - 參考 manual_test.md 進行完整測試"
    echo ""
    echo "   3. 運行 E2E 測試（如果環境已配置）："
    echo "      npx playwright test e2e/status_bar.spec.ts"
    echo ""
    exit 0
elif [ "$PASS_COUNT" -ge 4 ]; then
    echo "⚠️  大部分測試通過，但有些項目需要注意。"
    echo "   建議檢查失敗的測試項目。"
    echo ""
    exit 0
else
    echo "❌ 多個測試失敗！"
    echo "   修復可能未正確應用，請檢查："
    echo "   1. 是否正確運行了 apply_fix.sh"
    echo "   2. 目標文件是否正確"
    echo "   3. 查看失敗的測試項目"
    echo ""
    exit 1
fi
