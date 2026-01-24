#!/usr/bin/env node
/**
 * E2E Test for Google Docs Authentication and Sync Flow
 *
 * Tests the ACTUAL user flow for linking a book to Google Docs:
 * 1. Create/select a book
 * 2. Click "Sync Selected" button
 * 3. Authenticate (if needed)
 * 4. Select a Google Doc from the list
 * 5. Link the doc
 * 6. Verify google-docs-sync.yaml is created
 *
 * Uses fake servers for both OAuth and Google Docs API - no real API calls!
 */

import { By, until } from "selenium-webdriver";
import path from "path";
import fs from "fs";
import {
  startDriver,
  startViteServer,
  createWebDriverSession,
  buildTauriBinary,
  createMockWorkspace,
  runTest,
  slowModeDelay,
} from "./e2e-framework.js";
import { startFakeOAuthServer, stopFakeOAuthServer } from "./fake-oauth-server.js";
import { startFakeGoogleDocsApi, stopFakeGoogleDocsApi } from "./fake-google-docs-api.js";

async function run() {
  console.log("=== Starting e2e Google Docs authentication and sync test ===");

  let driverProc;
  let driver;
  let viteProc;
  let oauthServer;
  let googleApiServer;
  let workspace;

  const context = { driver, driverProc, viteProc, oauthServer, googleApiServer };

  await runTest(
    async (ctx, { KEEP_OPEN, SLOW_MODE }) => {
      // Configure OAuth delay based on SLOW_MODE
      // SLOW_MODE: 4000ms delay (visible for manual observation)
      // Fast mode: 2500ms delay (gives Selenium time to detect dialog)
      process.env.OAUTH_DELAY_MS = SLOW_MODE ? "4000" : "2500";

      // Start fake OAuth server
      console.log("[DEBUG] Starting fake OAuth server on port 8787...");
      console.log(`[DEBUG] OAuth redirect delay: ${process.env.OAUTH_DELAY_MS}ms`);
      oauthServer = await startFakeOAuthServer();
      ctx.oauthServer = oauthServer;
      console.log("[DEBUG] Fake OAuth server ready");

      // Start fake Google Docs API server
      console.log("[DEBUG] Starting fake Google Docs API server on port 8788...");
      googleApiServer = await startFakeGoogleDocsApi();
      ctx.googleApiServer = googleApiServer;
      console.log("[DEBUG] Fake Google Docs API server ready");

      // Create mock workspace with a sample book
      console.log("[DEBUG] Creating mock workspace with sample book...");
      workspace = createMockWorkspace({
        prefix: "iriebook-e2e-google-auth-",
      });
      console.log(`[DEBUG] Mock workspace: ${workspace}`);

      // Create a sample book
      const bookDir = path.join(workspace, "sample-book");
      fs.mkdirSync(bookDir, { recursive: true });

      const bookContent = `# Sample Book

This is a test book for Google Docs sync e2e testing.

## Chapter 1

"This is a quote," she said.
`;

      fs.writeFileSync(path.join(bookDir, "book.md"), bookContent);

      const metadataContent = `---
title: Sample Test Book
author: Test Author
---
`;

      fs.writeFileSync(path.join(bookDir, "metadata.yaml"), metadataContent);

      console.log("[DEBUG] Sample book created");

      const scriptDir = path.dirname(new URL(import.meta.url).pathname);
      const tauriDir = path.resolve(scriptDir, "..");
      const workspaceRoot = path.resolve(scriptDir, "../..");
      const manifestPath = path.resolve(scriptDir, "../src-tauri/Cargo.toml");
      const binaryPath = path.resolve(
        workspaceRoot,
        "target/debug/iriebook-tauri-ui"
      );

      // Build debug binary (e2e-mocks feature included automatically by framework)
      console.log("[DEBUG] Building debug binary...");
      await buildTauriBinary(manifestPath);
      console.log("[DEBUG] Build complete!");

      // Start Vite dev server
      console.log("[DEBUG] Starting Vite dev server...");
      viteProc = await startViteServer(tauriDir);
      ctx.viteProc = viteProc;
      console.log("[DEBUG] Vite dev server ready!");

      // Start driver with custom OAuth URLs and Google Docs API URL
      console.log("[DEBUG] Starting tauri-driver with fake API URLs...");
      driverProc = await startDriver({
        workspacePath: workspace,
        env: {
          GOOGLE_OAUTH_AUTH_URL: "http://127.0.0.1:8787/o/oauth2/v2/auth",
          GOOGLE_OAUTH_TOKEN_URL: "http://127.0.0.1:8787/token",
          GOOGLE_DOCS_API_URL: "http://127.0.0.1:8788/drive/v3",  // Include full path
          GOOGLE_CLIENT_ID: "fake_client_id_for_testing",
          GOOGLE_CLIENT_SECRET: "fake_client_secret_for_testing",
        },
      });
      ctx.driverProc = driverProc;

      // Create WebDriver session
      console.log("[DEBUG] Creating WebDriver session...");
      driver = await createWebDriverSession(binaryPath);
      ctx.driver = driver;

      // Wait for app to load
      console.log("[DEBUG] Waiting for app to load...");
      await driver.wait(until.elementLocated(By.css("body")), 20_000);
      console.log("[DEBUG] Body element found");

      // Give app time to initialize and scan books
      await new Promise((resolve) => setTimeout(resolve, 3000));
      await slowModeDelay(1000, SLOW_MODE);

      // ========================================================================
      // Scenario 1: Initial State - Book Not Linked
      // ========================================================================
      console.log("[TEST] Scenario 1: Verifying book appears and is not linked");

      // Wait for book list to populate
      await driver.wait(async () => {
        const bookElements = await driver.findElements(
          By.css("[data-book-path]")
        );
        return bookElements.length > 0;
      }, 10000, "Book list did not populate");

      console.log("[DEBUG] Book list populated");

      // Find our sample book
      const bookElement = await driver.findElement(
        By.css("[data-book-path]")
      );
      const bookPath = await bookElement.getAttribute("data-book-path");
      console.log(`[DEBUG] Found book: ${bookPath}`);

      // Click on the book to view it (app already in Current Book mode via e2e-mocks)
      await bookElement.click();
      await slowModeDelay(500, SLOW_MODE);
      console.log("[DEBUG] Book selected and viewed (Current Book mode enabled by e2e-mocks)");

      // ========================================================================
      // Scenario 2: Click Sync - Triggers Auth Flow
      // ========================================================================
      console.log("[TEST] Scenario 2: Clicking Sync Selected button");

      // Find and click the "Sync Selected" button
      const syncButton = await driver.wait(
        until.elementLocated(By.css("[data-testid='sync-selected-button']")),
        5000
      );
      console.log("[DEBUG] Found 'Sync Selected' button");

      await syncButton.click();
      console.log("[DEBUG] Clicked 'Sync Selected' button");

      // Give time for the click to process, IPC calls to complete, and React to render
      // Flow: click → handleSyncSelected → ensureAuthForLink → googleCheckAuth (IPC) → startAuthFlow → setState → React render
      await new Promise((resolve) => setTimeout(resolve, 2000));

      // ========================================================================
      // Scenario 3: Auth Dialog Appears
      // ========================================================================
      console.log("[TEST] Scenario 3: Waiting for auth dialog to appear");

      // Get a snapshot of what Selenium can actually see in the DOM
      const domSnapshot = await driver.executeScript(() => {
        const body = document.body;
        const bodyHTML = body ? body.innerHTML : 'NO BODY';

        // Get all elements with data-testid
        const testIdElements = Array.from(document.querySelectorAll('[data-testid]')).map(el => el.getAttribute('data-testid'));

        // Check for dialog-specific elements
        const byTestId = document.querySelector('[data-testid="google-auth-dialog"]');
        const byRole = document.querySelector('[role="dialog"]');
        const allDialogs = document.querySelectorAll('[role="dialog"]');
        const radixDialogs = document.querySelectorAll('[data-state="open"]');

        return {
          authDialog: !!byTestId,
          anyDialog: !!byRole,
          dialogCount: allDialogs.length,
          radixCount: radixDialogs.length,
          testIdElements: testIdElements,
          bodyLength: bodyHTML.length,
          bodyPreview: bodyHTML.substring(0, 500) // First 500 chars
        };
      });
      console.log(`[DEBUG] DOM Snapshot:`, JSON.stringify(domSnapshot, null, 2));

      // Auth dialog should appear with loading state
      await driver.wait(
        until.elementLocated(By.css("[data-testid='google-auth-dialog']")),
        5000,
        "Auth dialog did not appear"
      );
      console.log("[DEBUG] ✅ Auth dialog appeared");

      const loadingDiv = await driver.wait(
        until.elementLocated(By.css("[data-testid='google-auth-loading']")),
        5000,
        "Auth loading state did not appear"
      );
      console.log("[DEBUG] ✅ Auth loading state visible (browser opened)");

      await slowModeDelay(1500, SLOW_MODE);

      // ========================================================================
      // Scenario 4: OAuth Completes and Dialog Closes
      // ========================================================================
      console.log("[TEST] Scenario 4: Waiting for OAuth to complete and dialog to close");

      // Wait for auth dialog to disappear (OAuth completed)
      await driver.wait(async () => {
        const dialogs = await driver.findElements(
          By.css("[data-testid='google-auth-dialog']")
        );
        return dialogs.length === 0;
      }, 10000, "Auth dialog did not close after OAuth completion");

      console.log("[DEBUG] ✅ Auth dialog closed (OAuth completed)");

      // ========================================================================
      // Scenario 5: Select Google Doc
      // ========================================================================
      console.log("[TEST] Scenario 5: Selecting Google Doc to link");

      // LinkGoogleDocDialog should appear after auth completes
      await driver.wait(
        until.elementLocated(By.css("[data-testid='link-google-doc-dialog']")),
        15000,
        "Link Google Doc dialog did not appear after authentication"
      );
      console.log("[DEBUG] LinkGoogleDocDialog appeared");

      await slowModeDelay(1000, SLOW_MODE);

      // Wait for docs list to load
      console.log("[DEBUG] Waiting for docs list to load...");
      await driver.wait(
        until.elementLocated(By.css("[data-testid^='google-doc-item-']")),
        10000,
        "Google Docs list did not load"
      );
      console.log("[DEBUG] Docs list loaded");

      // Verify the fake doc element exists (user confirmed they can see it)
      const docItem = await driver.findElement(
        By.css("[data-testid='google-doc-item-fake_doc_id_001']")
      );
      console.log("[DEBUG] Found fake doc item in DOM");

      await slowModeDelay(1500, SLOW_MODE);

      // Click the "Link" button for the first doc
      const linkButton = await driver.findElement(
        By.css("[data-testid='google-doc-link-button-fake_doc_id_001']")
      );
      console.log("[DEBUG] Found 'Link' button");

      await linkButton.click();
      console.log("[DEBUG] Clicked 'Link' button");

      await slowModeDelay(1000, SLOW_MODE);

      // ========================================================================
      // Scenario 6: Verify Link Created
      // ========================================================================
      console.log("[TEST] Scenario 6: Verifying google-docs-sync.yaml created");

      // Wait for dialog to close (indicates link completed)
      console.log("[DEBUG] Waiting for link to complete...");
      await driver.wait(async () => {
        const dialogs = await driver.findElements(
          By.css("[data-testid='link-google-doc-dialog']")
        );
        return dialogs.length === 0;
      }, 10000, "Link dialog did not close");

      console.log("[DEBUG] Dialog closed, link should be complete");

      // Give it a moment for the file to be written
      await new Promise((resolve) => setTimeout(resolve, 1000));

      // Verify google-docs-sync.yaml exists
      const yamlPath = path.join(bookDir, "google-docs-sync.yaml");
      console.log(`[DEBUG] Checking for file: ${yamlPath}`);

      // Wait for file to be created (with timeout)
      let fileExists = false;
      for (let i = 0; i < 10; i++) {
        if (fs.existsSync(yamlPath)) {
          fileExists = true;
          break;
        }
        await new Promise((resolve) => setTimeout(resolve, 500));
      }

      if (!fileExists) {
        throw new Error(
          `google-docs-sync.yaml was not created at ${yamlPath}`
        );
      }

      console.log("[DEBUG] File exists: google-docs-sync.yaml");

      // Verify file content
      const yamlContent = fs.readFileSync(yamlPath, "utf8");
      console.log(`[DEBUG] YAML content:\n${yamlContent}`);

      if (!yamlContent.includes("google-doc-id: fake_doc_id_001")) {
        throw new Error(
          `google-docs-sync.yaml missing correct doc ID. Content: ${yamlContent}`
        );
      }
      console.log("[DEBUG] File content verified: google-doc-id: fake_doc_id_001");

      if (!yamlContent.includes("sync-status:")) {
        throw new Error(
          `google-docs-sync.yaml missing sync-status. Content: ${yamlContent}`
        );
      }
      console.log("[DEBUG] File content verified: sync-status field present");

      await slowModeDelay(2000, SLOW_MODE);

      // ========================================================================
      // Scenario 7: Verify Book Content Was Synced from Google Docs
      // ========================================================================
      console.log("[TEST] Scenario 7: Verifying book.md content was updated from Google Docs");

      // The onLinked callback automatically triggers syncSingleBook after linking
      // Give it time to complete the sync and write the file
      await new Promise((resolve) => setTimeout(resolve, 2000));

      // Read the book.md content
      const bookMdPath = path.join(bookDir, "book.md");
      console.log(`[DEBUG] Reading book content from: ${bookMdPath}`);

      let updatedContent = "";
      // Wait for file to be updated (with timeout)
      for (let i = 0; i < 10; i++) {
        if (fs.existsSync(bookMdPath)) {
          updatedContent = fs.readFileSync(bookMdPath, "utf8");
          // Check if content has been updated (no longer the original sample book)
          if (updatedContent.includes("Sample Google Doc") &&
              updatedContent.includes("fake content from a mocked Google Doc")) {
            break;
          }
        }
        await new Promise((resolve) => setTimeout(resolve, 500));
      }

      console.log(`[DEBUG] Book content:\\n${updatedContent}`);

      // Expected content from fake-google-docs-api.js (lines 33-42)
      const expectedContent = `# Sample Google Doc

This is fake content from a mocked Google Doc.

## Chapter 1

"Hello," she said.

"World," he replied.
`;

      if (updatedContent.trim() !== expectedContent.trim()) {
        throw new Error(
          `book.md content does not match expected synced content from Google Docs.\\n\\nExpected:\\n${expectedContent}\\n\\nActual:\\n${updatedContent}`
        );
      }
      console.log("[DEBUG] ✅ Book content verified: matches content from Google Docs API");

      await slowModeDelay(2000, SLOW_MODE);

      console.log("[SUCCESS] All test scenarios passed!");
      console.log("[SUCCESS] Google Docs authentication, linking, and sync flow works correctly!");
    },
    context
  );

  // Cleanup servers
  console.log("[DEBUG] Stopping fake OAuth server");
  await stopFakeOAuthServer(oauthServer);
  console.log("[DEBUG] Fake OAuth server stopped");

  console.log("[DEBUG] Stopping fake Google Docs API server");
  await stopFakeGoogleDocsApi(googleApiServer);
  console.log("[DEBUG] Fake Google Docs API server stopped");

  // Optional: Clean up workspace
  if (!process.env.KEEP_OPEN && workspace) {
    console.log(`[DEBUG] Cleaning up workspace: ${workspace}`);
    fs.rmSync(workspace, { recursive: true, force: true });
  }
}

// Run the test
run().catch((err) => {
  console.error("Test failed:", err);
  process.exit(1);
});
