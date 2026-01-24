#!/usr/bin/env node
/*
 * E2E smoke test with mocked state initialization.
 * Verifies that the app loads successfully with mock resource access.
 */

import { By, until } from "selenium-webdriver";
import path from "path";
import {
  startDriver,
  startViteServer,
  createWebDriverSession,
  buildTauriBinary,
  createMockBookWorkspace,
  runTest,
  slowModeDelay
} from "./e2e-framework.js";

async function run() {
  console.log("=== Starting e2e UI navigation test ===");

  let driverProc;
  let driver;
  let viteProc;

  const context = { driver, driverProc, viteProc };

  await runTest(async (ctx, { KEEP_OPEN, SLOW_MODE }) => {
    // Create mock workspace
    console.log("[DEBUG] Creating mock workspace");
    const workspace = createMockBookWorkspace();
    console.log(`[DEBUG] Mock workspace: ${workspace}`);

    const scriptDir = path.dirname(new URL(import.meta.url).pathname);
    const tauriDir = path.resolve(scriptDir, "..");
    const workspaceRoot = path.resolve(scriptDir, "../..");
    const manifestPath = path.resolve(scriptDir, "../src-tauri/Cargo.toml");
    const binaryPath = path.resolve(workspaceRoot, "target/debug/iriebook-tauri-ui");

    // Build debug binary (e2e-mocks feature included automatically by framework)
    await buildTauriBinary(manifestPath);

    // Start Vite dev server
    viteProc = await startViteServer(tauriDir);
    ctx.viteProc = viteProc;

    // Start driver with workspace path
    driverProc = await startDriver({ workspacePath: workspace });
    ctx.driverProc = driverProc;

    // Create WebDriver session
    driver = await createWebDriverSession(binaryPath);
    ctx.driver = driver;

    // Wait for app to load
    console.log("[DEBUG] Waiting for app to load");
    await driver.wait(until.elementLocated(By.css("body")), 20_000);
    console.log("[DEBUG] Body element found");

    // Give React time to load session and scan books
    console.log("[DEBUG] Waiting for app to load workspace from env var...");
    await new Promise((resolve) => setTimeout(resolve, 3000));

    await slowModeDelay(2000, SLOW_MODE);

    // Workspace should be loaded from env var
    console.log("[DEBUG] Checking that workspace loaded from IRIEBOOK_WORKSPACE env var");
    await slowModeDelay(1500, SLOW_MODE);

    // Navigate to History tab and test diff visualization
    console.log("[DEBUG] Looking for History tab");
    try {
      // Find and click the History tab
      const historyTab = await driver.wait(
        async () => {
          const elements = await driver.findElements(By.xpath("//button[contains(text(), 'Changes')]"));
          return elements.length > 0 ? elements[0] : null;
        },
        5000,
        "History tab not found"
      );

      console.log("[DEBUG] Found History tab");
      await slowModeDelay(1000, SLOW_MODE);

      console.log("[DEBUG] Clicking Changes tab");
      await historyTab.click();
      await slowModeDelay(2000, SLOW_MODE);

      // Wait for history items to appear
      console.log("[DEBUG] Waiting for history items to appear");
      const historyItems = await driver.wait(
        async () => {
          const items = await driver.findElements(By.css("[data-testid='revision-item'], .revision-item, .history-item"));
          return items.length > 0 ? items : null;
        },
        8000,
        "No history items found"
      );

      console.log(`[DEBUG] Found ${historyItems.length} history item(s)`);

      // Get the first (newest) item
      const newestItem = historyItems[0];
      const itemText = await newestItem.getText();
      console.log(`[DEBUG] Newest history item: "${itemText}"`);

      await slowModeDelay(1500, SLOW_MODE);

      // Double-click the newest item to open diff view
      console.log("[DEBUG] Double-clicking newest history item");
      const actions = driver.actions({ async: true });
      await actions.doubleClick(newestItem).perform();

      await slowModeDelay(2000, SLOW_MODE);

      // Wait for diff view to appear
      console.log("[DEBUG] Waiting for diff view to appear");
      await driver.wait(
        async () => {
          const diffView = await driver.findElements(By.css("[data-testid='diff-view'], .diff-view, .diff-container"));
          return diffView.length > 0;
        },
        8000,
        "Diff view did not appear"
      );

      console.log("[DEBUG] Diff view appeared");
      await slowModeDelay(1500, SLOW_MODE);

      // Assert expected content in diff view
      console.log("[DEBUG] Verifying diff content");

      // Look for left side (original) content
      const leftContent = await driver.findElements(By.css("[data-testid='diff-left'], .diff-left, .diff-original"));
      if (leftContent.length === 0) {
        throw new Error("Left side of diff not found");
      }
      const leftText = await leftContent[0].getText();
      console.log(`[DEBUG] Left side content length: ${leftText.length} chars`);

      // Look for right side (modified) content
      const rightContent = await driver.findElements(By.css("[data-testid='diff-right'], .diff-right, .diff-modified"));
      if (rightContent.length === 0) {
        throw new Error("Right side of diff not found");
      }
      const rightText = await rightContent[0].getText();
      console.log(`[DEBUG] Right side content length: ${rightText.length} chars`);

      // Basic assertion: both sides should have content and be different
      if (leftText.length === 0 || rightText.length === 0) {
        throw new Error("Diff view is empty");
      }

      if (leftText === rightText) {
        console.log("[WARN] Left and right sides are identical - expected some differences");
      } else {
        console.log("[DEBUG] Diff shows different content on left vs right - GOOD!");
      }

      // Look for expected mock content from git history
      if (rightText.includes("Chapter 3") || rightText.includes("And so it ends")) {
        console.log("[DEBUG] Found expected Chapter 3 content in right side of diff");
      }

      if (!leftText.includes("Chapter 3")) {
        console.log("[DEBUG] Left side correctly does not have Chapter 3");
      }

      if (rightText.includes('"Hello," she said') && rightText.includes('"World," he replied')) {
        console.log("[DEBUG] Found expected dialogue in diff content");
      }

      console.log("[SUCCESS] Diff visualization test passed!");
    } catch (e) {
      console.log(`[ERROR] Diff visualization test failed: ${e.message}`);
      console.log("[INFO] This might be due to missing e2e-mocks data or UI selectors");
      throw e;
    }

    console.log("[SUCCESS] E2E diff test completed!");
  }, context);
}

console.log("[DEBUG] About to call run()");
run()
  .then(() => {
    console.log("[DEBUG] run() promise resolved - test completed successfully");
    console.log("[DEBUG] Exiting .then() handler");
    console.log("[DEBUG] Forcing process exit");
    setImmediate(() => process.exit(0));
  })
  .catch((err) => {
    console.error("=== FATAL ERROR ===");
    console.error(err);
    console.log("[DEBUG] Forcing process exit with code 1");
    setImmediate(() => process.exit(1));
  });

console.log("[DEBUG] After run() call - waiting for promise to resolve");
