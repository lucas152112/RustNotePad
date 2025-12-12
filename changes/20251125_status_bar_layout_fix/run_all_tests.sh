#!/bin/bash
# 快速測試腳本 - 運行所有驗證
# 日期: 2025-11-25

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"

echo "====================================="
echo "  快速測試套件"
echo "====================================="
echo ""

# 1. 運行驗證腳本
echo "【1/3】運行驗證腳本..."
echo "─────────────────────────────────────"
bash "$SCRIPT_DIR/verify_fix.sh"
VERIFY_EXIT=$?
echo ""

# 2. 運行單元測試
echo "【2/3】運行單元測試..."
echo "─────────────────────────────────────"
cd "$REPO_ROOT/rustnotepad_gui"
if cargo test --quiet 2>&1 | tail -5; then
    echo "✅ 單元測試通過"
    UNIT_TEST_EXIT=0
else
    echo "❌ 單元測試失敗"
    UNIT_TEST_EXIT=1
fi
echo ""

# 3. 編譯發布版本（可選）
echo "【3/3】編譯檢查..."
echo "─────────────────────────────────────"
cd "$REPO_ROOT/rustnotepad_gui"
if cargo build --quiet 2>&1 | grep -i "error:"; then
    echo "❌ 編譯失敗"
    BUILD_EXIT=1
else
    echo "✅ 編譯成功"
    BUILD_EXIT=0
fi
echo ""

# 總結
echo "====================================="
echo "  測試總結"
echo "====================================="
echo ""

TOTAL_PASS=0
TOTAL_TESTS=3

if [ $VERIFY_EXIT -eq 0 ]; then
    echo "✅ 驗證測試：通過"
    ((TOTAL_PASS++))
else
    echo "❌ 驗證測試：失敗"
fi

if [ $UNIT_TEST_EXIT -eq 0 ]; then
    echo "✅ 單元測試：通過"
    ((TOTAL_PASS++))
else
    echo "❌ 單元測試：失敗"
fi

if [ $BUILD_EXIT -eq 0 ]; then
    echo "✅ 編譯測試：通過"
    ((TOTAL_PASS++))
else
    echo "❌ 編譯測試：失敗"
fi

echo ""
echo "總計：$TOTAL_PASS / $TOTAL_TESTS 通過"
echo ""

if [ $TOTAL_PASS -eq $TOTAL_TESTS ]; then
    echo "🎉 所有測試通過！"
    echo ""
    echo "📝 下一步："
    echo "   1. 執行手動測試（參考 manual_test.md）"
    echo "   2. 提交變更到版本控制"
    echo ""
    exit 0
else
    echo "⚠️  有測試失敗，請檢查錯誤訊息"
    exit 1
fi
