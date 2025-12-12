#!/bin/bash
# 一鍵應用狀態列布局修復
# 日期: 2025-11-25

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
TARGET_FILE="$REPO_ROOT/rustnotepad_gui/src/main.rs"

echo "====================================="
echo "  狀態列布局修復 - 自動應用腳本"
echo "====================================="
echo ""

# 檢查目標文件是否存在
if [ ! -f "$TARGET_FILE" ]; then
    echo "❌ 錯誤：找不到目標文件 $TARGET_FILE"
    exit 1
fi

echo "📁 目標文件：$TARGET_FILE"
echo ""

# 備份原始文件
BACKUP_FILE="${TARGET_FILE}.backup.$(date +%Y%m%d_%H%M%S)"
echo "💾 建立備份：$BACKUP_FILE"
cp "$TARGET_FILE" "$BACKUP_FILE"

# 檢查是否已經應用過修復
if grep -A 15 "fn update.*App for RustNotePadApp" "$TARGET_FILE" | grep -q "show_editor_area.*show_status_bar"; then
    echo ""
    echo "✅ 修復已經應用過了！"
    echo "   狀態列已經在編輯區之後渲染"
    echo ""
    echo "如需重新應用，請先還原到修改前的版本。"
    exit 0
fi

echo "🔧 應用修復..."
echo ""

# 使用 sed 進行修改
# 在 show_editor_area(ctx); 之後添加 show_status_bar(ctx);
# 並刪除原來在前面的 show_status_bar(ctx);
sed -i.tmp '
    /fn update(&mut self, ctx: &egui::Context, _frame: &mut Frame)/,/^[[:space:]]*}[[:space:]]*$/ {
        # 先標記要刪除的 show_status_bar 行
        /self\.show_status_bar(ctx);.*$/ {
            # 如果這行在 show_editor_area 之前，則刪除
            /self\.show_toolbar(ctx);/,/self\.show_editor_area(ctx);/ {
                d
            }
        }
        # 在 show_editor_area 之後添加 show_status_bar
        /self\.show_editor_area(ctx);/ a\
        self.show_status_bar(ctx);
    }
' "$TARGET_FILE"

# 檢查是否成功修改
if grep -A 15 "fn update.*App for RustNotePadApp" "$TARGET_FILE" | grep -q "show_editor_area.*show_status_bar"; then
    echo "✅ 修復成功應用！"
    echo ""
    echo "📋 修改摘要："
    echo "   - 將 show_status_bar() 移至 show_editor_area() 之後"
    echo "   - 確保狀態列在編輯區之後渲染"
    echo ""
    
    # 清理臨時文件
    rm -f "${TARGET_FILE}.tmp"
    
    # 編譯測試
    echo "🔨 編譯測試..."
    cd "$REPO_ROOT/rustnotepad_gui"
    if cargo build --quiet 2>&1 | grep -i "error:"; then
        echo "❌ 編譯失敗！正在還原..."
        cp "$BACKUP_FILE" "$TARGET_FILE"
        echo "   已還原到原始版本"
        exit 1
    else
        echo "✅ 編譯成功"
    fi
    
    echo ""
    echo "====================================="
    echo "  修復已成功應用並通過編譯測試"
    echo "====================================="
    echo ""
    echo "📝 後續步驟："
    echo "   1. 運行驗證腳本: ./verify_fix.sh"
    echo "   2. 執行手動測試（參考 manual_test.md）"
    echo "   3. 運行單元測試: cd rustnotepad_gui && cargo test"
    echo ""
    echo "💾 備份文件：$BACKUP_FILE"
    echo "   （如需還原，請執行：cp $BACKUP_FILE $TARGET_FILE）"
    echo ""
else
    echo "❌ 修復應用失敗！"
    echo "   正在還原到原始版本..."
    cp "$BACKUP_FILE" "$TARGET_FILE"
    rm -f "${TARGET_FILE}.tmp"
    echo "   已還原"
    exit 1
fi
