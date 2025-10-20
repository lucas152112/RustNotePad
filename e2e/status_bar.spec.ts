import { test, expect } from '@playwright/test';
import { remote } from 'webdriverio';

// 確認狀態列顯示語言與主題資訊 / Validate status bar shows language + theme metadata.
test('status bar metadata', async () => {
  let driver: any = null;

  try {
    // 啟動 tauri-driver WebDriver 會話
    // 注意：需先手動執行 `tauri-driver` 命令啟動 WebDriver 服務
    driver = await remote({
      capabilities: {
        'tauri:options': {
          application: './target/debug/rustnotepad_gui'
        }
      } as any,
      logLevel: 'info',
      port: 4444,
      path: '/',
    });

    // 等待應用程式載入
    await driver.pause(1000);

    // 查找狀態列元素
    const statusBar = await driver.$('//*[@id="status_bar"]');
    const innerText = await statusBar.getText();
    
    // 驗證狀態列包含語言與主題資訊
    expect(innerText).toContain('Lang');
    expect(innerText).toContain('Theme');

  } finally {
    // 確保清理資源
    if (driver) {
      await driver.deleteSession();
    }
  }
});
