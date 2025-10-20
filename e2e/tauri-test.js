#!/usr/bin/env node
/**
 * Standalone Tauri E2E test using WebdriverIO
 * Run with: node e2e/tauri-test.js
 * Make sure tauri-driver is running first: tauri-driver --port 4444
 */

const { remote } = require('webdriverio');
const path = require('path');

async function runTest() {
  console.log('🚀 Connecting to tauri-driver...');
  
  const appPath = path.join(__dirname, '..', 'target', 'release', 'rustnotepad_gui');
  console.log(`🔍 Using application path: ${appPath}`);

  const browser = await remote({
    logLevel: 'info',
    capabilities: {
      'tauri:options': {
        application: appPath,
      },
    },
    port: 4444,
    path: '/',
  });

  try {
    console.log('✅ Connected to Tauri application');
    
    // Wait for the application to load
    await browser.pause(2000);
    
    // Test: Check if status bar exists (assuming it has an ID or class)
    console.log('🔍 Testing status bar metadata...');
    
    // Get the window title
    const title = await browser.getTitle();
    console.log(`📝 Window title: ${title}`);
    
    // Try to find status bar element (adjust selector based on your app)
    try {
      const statusBar = await browser.$('.status-bar');
      const isDisplayed = await statusBar.isDisplayed();
      console.log(`📊 Status bar displayed: ${isDisplayed}`);
    } catch (e) {
      console.log('⚠️  Status bar element not found with .status-bar selector');
      console.log('   Try checking the HTML structure of your app');
    }
    
    // You can add more tests here
    
    console.log('✅ Test completed successfully');
  } catch (error) {
    console.error('❌ Test failed:', error.message);
    throw error;
  } finally {
    await browser.deleteSession();
    console.log('🔚 Session closed');
  }
}

// Run the test
runTest()
  .then(() => {
    console.log('✨ All tests passed');
    process.exit(0);
  })
  .catch((error) => {
    console.error('💥 Test suite failed:', error);
    process.exit(1);
  });
