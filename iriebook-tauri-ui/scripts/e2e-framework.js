#!/usr/bin/env node
/**
 * E2E Test Framework
 *
 * Reusable utilities for Tauri e2e tests using Selenium WebDriver.
 * Handles driver setup, Vite dev server, mock workspace creation, and cleanup.
 */

import { Builder } from "selenium-webdriver";
import { spawn, spawnSync } from "child_process";
import { mkdtempSync, mkdirSync, writeFileSync } from "fs";
import path from "path";
import os from "os";
import net from "net";

// =============================================================================
// Constants and Configuration
// =============================================================================

export const DEFAULT_START_TIMEOUT_MS = 30_000;
export const DRIVER_PORT = process.env.TAURI_DRIVER_PORT || "9555";
export const NATIVE_PORT = process.env.WEBKIT_WEBDRIVER_PORT || "9556";
export const DRIVER_URL = process.env.TAURI_DRIVER_URL || `http://127.0.0.1:${DRIVER_PORT}`;
export const NATIVE_DRIVER = process.env.WEBKIT_WEBDRIVER_PATH || "/usr/bin/WebKitWebDriver";
export const VITE_PORT = "1420";

// =============================================================================
// Port Utilities
// =============================================================================

/**
 * Wait for a TCP port to become available.
 * @param {number|string} port - Port number to check
 * @param {string} label - Label for logging
 * @param {number} timeout - Timeout in milliseconds
 * @returns {Promise<void>}
 */
export function waitForPort(port, label, timeout = DEFAULT_START_TIMEOUT_MS) {
  return new Promise((resolve, reject) => {
    const timeoutHandle = setTimeout(
      () => reject(new Error(`${label} port ${port} did not become available in time`)),
      timeout
    );
    const startTime = Date.now();
    let attempts = 0;

    const checkPort = () => {
      attempts++;
      if (attempts === 1 || attempts % 10 === 0) {
        process.stderr.write(`[${label}] checking port ${port} (attempt ${attempts})...\n`);
      }

      const socket = new net.Socket();
      socket.setTimeout(100);

      socket.on("connect", () => {
        socket.destroy();
        clearTimeout(timeoutHandle);
        const elapsed = Date.now() - startTime;
        process.stderr.write(`[${label}] port ${port} ready after ${elapsed}ms (${attempts} attempts)\n`);
        resolve();
      });

      socket.on("timeout", () => {
        socket.destroy();
        setTimeout(checkPort, 100);
      });

      socket.on("error", (err) => {
        if (attempts % 10 === 0) {
          process.stderr.write(`[${label}] port ${port} error: ${err.code}\n`);
        }
        socket.destroy();
        setTimeout(checkPort, 100);
      });

      socket.connect(port, "127.0.0.1");
    };

    checkPort();
  });
}

// =============================================================================
// Process Management
// =============================================================================

/**
 * Kill stale processes by name.
 * @param {string[]} processNames - Process names to kill
 * @returns {Promise<void>}
 */
export async function killStaleProcesses(...processNames) {
  console.log(`[DEBUG] Killing stale processes: ${processNames.join(", ")}`);

  for (const name of processNames) {
    spawn("pkill", ["-x", name]).on("error", () => { });
  }

  await new Promise((resolve) => setTimeout(resolve, 500));
}

/**
 * Configure a spawned process to not keep the event loop alive.
 * @param {ChildProcess} proc - Process to configure
 */
export function unrefProcess(proc) {
  proc.unref();
  if (proc.stderr) proc.stderr.on("data", (d) => process.stderr.write(`[${proc.spawnfile}] ${d}`));
  if (proc.stdout) proc.stdout.on("data", (d) => process.stderr.write(`[${proc.spawnfile}] ${d}`));
}

/**
 * Clean up a spawned process and its streams.
 * @param {ChildProcess} proc - Process to clean up
 * @param {string} label - Label for logging
 * @param {number} timeout - Timeout in milliseconds to wait for exit
 * @returns {Promise<void>}
 */
export async function cleanupProcess(proc, label, timeout = 2000) {
  if (!proc) return;

  console.log(`[DEBUG] Killing ${label} process`);
  proc.kill("SIGTERM");

  console.log(`[DEBUG] Waiting for ${label} to exit`);
  await Promise.race([
    new Promise((resolve) => {
      proc.once("exit", () => {
        console.log(`[DEBUG] ${label} exit event received`);
        resolve();
      });
    }),
    new Promise((resolve) => {
      const timeoutHandle = setTimeout(() => {
        console.log(`[DEBUG] ${label} exit timeout (${timeout}ms)`);
        resolve();
      }, timeout);
      timeoutHandle.unref();
    })
  ]);

  // Clean up streams
  proc.stdout?.removeAllListeners();
  proc.stderr?.removeAllListeners();
  proc.stdout?.unref();
  proc.stderr?.unref();

  console.log(`[DEBUG] ${label} cleanup complete`);
}

// =============================================================================
// Tauri Driver Management
// =============================================================================

/**
 * Start tauri-driver with optional environment variables.
 * @param {Object} options - Configuration options
 * @param {string} [options.workspacePath] - Optional workspace path to set IRIEBOOK_WORKSPACE env var
 * @param {Object} [options.env] - Additional environment variables
 * @returns {Promise<ChildProcess>} - The driver process
 */
export async function startDriver({ workspacePath, env = {} } = {}) {
  await killStaleProcesses("WebKitWebDriver", "tauri-driver");

  const binary = process.env.TAURI_DRIVER_PATH || "tauri-driver";
  const args = [
    "--native-driver", NATIVE_DRIVER,
    "--native-port", NATIVE_PORT,
    "--port", DRIVER_PORT
  ];

  console.log(`[DEBUG] startDriver: spawning ${binary}`);
  if (workspacePath) {
    console.log(`[DEBUG] startDriver: setting IRIEBOOK_WORKSPACE=${workspacePath}`);
  }

  const driverEnv = {
    ...process.env,
    ...env
  };

  if (workspacePath) {
    driverEnv.IRIEBOOK_WORKSPACE = workspacePath;
  }

  const proc = spawn(binary, args, { env: driverEnv });
  unrefProcess(proc);

  proc.on("error", (err) => console.error(`[DEBUG] startDriver: process error: ${err.message}`));
  proc.on("exit", (code, signal) => console.log(`[DEBUG] startDriver: exited with code ${code}, signal ${signal}`));

  await waitForPort(DRIVER_PORT, "driver");
  console.log("[DEBUG] startDriver: complete");
  return proc;
}

// =============================================================================
// Vite Dev Server Management
// =============================================================================

/**
 * Start Vite dev server.
 * @param {string} cwd - Working directory (usually tauri-ui root)
 * @param {string} port - Port to run Vite on
 * @returns {Promise<ChildProcess>} - The Vite process
 */
export async function startViteServer(cwd, port = VITE_PORT) {
  // Kill any existing Vite processes
  console.log("[DEBUG] Cleaning up any existing Vite processes...");
  spawn("pkill", ["-f", `vite.*--port.*${port}`]).on("error", () => { });
  await new Promise((resolve) => setTimeout(resolve, 500));

  console.log(`[DEBUG] Starting Vite dev server on port ${port}...`);
  const viteProc = spawn("npm", ["run", "dev", "--", "--host", "--port", port], {
    cwd,
    stdio: "pipe"
  });

  unrefProcess(viteProc);
  viteProc.on("error", (err) => console.error(`[DEBUG] Vite process error: ${err.message}`));

  // Wait for Vite to be ready
  console.log("[DEBUG] Waiting for Vite dev server to be ready...");
  await waitForPort(port, "vite");
  console.log("[DEBUG] Vite dev server ready!");

  return viteProc;
}

/**
 * Clean up Vite dev server.
 * @param {ChildProcess} viteProc - Vite process to clean up
 * @param {string} port - Port Vite is running on
 * @returns {Promise<void>}
 */
export async function cleanupViteServer(viteProc, port = VITE_PORT) {
  if (!viteProc) return;

  console.log("[DEBUG] Stopping Vite dev server");
  viteProc.kill("SIGTERM");

  // Kill any child Vite processes spawned by npm
  spawn("pkill", ["-P", viteProc.pid.toString()]).on("error", () => {});

  console.log("[DEBUG] Waiting for Vite to exit");
  await Promise.race([
    new Promise((resolve) => {
      viteProc.once("exit", () => {
        console.log("[DEBUG] Vite exit event received");
        resolve();
      });
    }),
    new Promise((resolve) => {
      const timeout = setTimeout(() => {
        console.log("[DEBUG] Vite exit timeout (2s)");
        resolve();
      }, 2000);
      timeout.unref();
    })
  ]);

  // Extra cleanup: kill any remaining vite processes
  spawn("pkill", ["-f", `vite.*--port.*${port}`]).on("error", () => {});

  // Remove all listeners and unref streams
  viteProc.stdout?.removeAllListeners();
  viteProc.stderr?.removeAllListeners();
  viteProc.stdout?.unref();
  viteProc.stderr?.unref();

  console.log("[DEBUG] Vite cleanup complete");
}

// =============================================================================
// WebDriver Session Management
// =============================================================================

/**
 * Create a WebDriver session for Tauri app.
 * @param {string} binaryPath - Path to Tauri binary
 * @returns {Promise<WebDriver>} - The WebDriver instance
 */
export async function createWebDriverSession(binaryPath) {
  console.log("[DEBUG] Creating WebDriver session");
  console.log(`[DEBUG] Using binary: ${binaryPath}`);

  const capabilities = {
    browserName: "wry",
    "tauri:options": {
      application: binaryPath,
    },
  };

  const driver = await new Builder()
    .usingServer(DRIVER_URL)
    .withCapabilities(capabilities)
    .build();

  // Maximize window for better visibility
  await driver.manage().window().maximize();

  console.log("[DEBUG] WebDriver session created");
  return driver;
}

// =============================================================================
// Build Management
// =============================================================================

/**
 * Build Tauri binary for e2e testing.
 * Automatically includes 'e2e-mocks' feature for proper test setup.
 * @param {string} manifestPath - Path to Cargo.toml
 * @param {string[]} additionalFeatures - Additional features to enable (optional)
 * @returns {Promise<void>}
 */
export async function buildTauriBinary(manifestPath, additionalFeatures = []) {
  // Always include e2e-mocks feature for e2e tests
  const features = ["e2e-mocks", ...additionalFeatures];

  console.log(`[DEBUG] Building debug binary for e2e with features: ${features.join(", ")}...`);

  const args = ["build", "--manifest-path", manifestPath, "--features", features.join(",")];

  const buildProc = spawn("cargo", args, { stdio: "inherit" });

  await new Promise((resolve, reject) => {
    buildProc.on("exit", (code) =>
      code === 0 ? resolve() : reject(new Error(`Build failed: ${code}`))
    );
    buildProc.on("error", reject);
  });

  console.log("[DEBUG] Build complete!");
}

// =============================================================================
// Mock Workspace Creation
// =============================================================================

/**
 * Create a temporary workspace with git repository.
 * @param {Object} options - Configuration options
 * @param {string} [options.prefix] - Prefix for temp directory name
 * @param {Function} [options.setupCallback] - Callback to customize workspace setup
 * @returns {string} - Path to created workspace
 */
export function createMockWorkspace({ prefix = "iriebook-e2e-", setupCallback } = {}) {
  const workspace = mkdtempSync(path.join(os.tmpdir(), prefix));

  // Initialize git repository
  console.log("[DEBUG] Initializing git repository...");
  spawnSync("git", ["init"], { cwd: workspace });
  spawnSync("git", ["config", "user.name", "E2E Test"], { cwd: workspace });
  spawnSync("git", ["config", "user.email", "e2e@test.com"], { cwd: workspace });

  // Allow callback to customize workspace
  if (setupCallback) {
    setupCallback(workspace);
  }

  // Verify git history
  const logResult = spawnSync("git", ["log", "--oneline"], { cwd: workspace, encoding: "utf8" });
  console.log(`[DEBUG] Created git repository at ${workspace}`);
  if (logResult.stdout) {
    console.log(`[DEBUG] Git history:\n${logResult.stdout}`);
  }

  return workspace;
}

/**
 * Create a mock book workspace with sample book and git history.
 * This is the default workspace setup used in e2e-diff-test.js.
 * @returns {string} - Path to created workspace
 */
export function createMockBookWorkspace() {
  return createMockWorkspace({
    setupCallback: (workspace) => {
      const bookDir = path.join(workspace, "sample-book");
      mkdirSync(bookDir, { recursive: true });

      // Create initial version with Chapter 1 only
      const initialMd = `# Sample Book

## Chapter 1

"Hello," she said.

"World," he replied.
`;
      writeFileSync(path.join(bookDir, "book.md"), initialMd, "utf8");

      const metadata = `title: "Sample Book"
author: "E2E"
language: en
`;
      writeFileSync(path.join(bookDir, "metadata.yaml"), metadata, "utf8");

      // First commit
      console.log("[DEBUG] Creating first commit...");
      spawnSync("git", ["add", "."], { cwd: workspace });
      spawnSync("git", ["commit", "-m", "feat: initial book structure with chapter one"], { cwd: workspace });

      // Update with Chapter 2
      const withChapter2 = initialMd + `
## Chapter 2

"The journey begins," she whispered.
`;
      writeFileSync(path.join(bookDir, "book.md"), withChapter2, "utf8");

      // Second commit
      console.log("[DEBUG] Creating second commit...");
      spawnSync("git", ["add", "."], { cwd: workspace });
      spawnSync("git", ["commit", "-m", "feat: expand chapter two with dialogue"], { cwd: workspace });

      // Update with Chapter 3
      const finalMd = withChapter2 + `
## Chapter 3

"And so it ends," he concluded.
`;
      writeFileSync(path.join(bookDir, "book.md"), finalMd, "utf8");

      // Third commit (newest)
      console.log("[DEBUG] Creating third commit...");
      spawnSync("git", ["add", "."], { cwd: workspace });
      spawnSync("git", ["commit", "-m", "feat: add chapter three with conclusion"], { cwd: workspace });
    }
  });
}

// =============================================================================
// Test Runner Helpers
// =============================================================================

/**
 * Run a test with automatic cleanup and KEEP_OPEN/SLOW_MODE support.
 * @param {Function} testFn - Async test function to run
 * @param {Object} context - Test context (driver, driverProc, viteProc, etc.)
 * @returns {Promise<void>}
 */
export async function runTest(testFn, context = {}) {
  const KEEP_OPEN = process.env.KEEP_OPEN === "true";
  const SLOW_MODE = process.env.SLOW_MODE !== "false"; // Default to slow mode

  if (SLOW_MODE) {
    console.log("[INFO] Running in SLOW_MODE - adding delays for visibility");
  }

  try {
    await testFn(context, { KEEP_OPEN, SLOW_MODE });

    if (KEEP_OPEN) {
      console.log("");
      console.log("===========================================");
      console.log("KEEP_OPEN mode: Window will stay open");
      console.log("Press Ctrl+C to close the app and exit");
      console.log("===========================================");
      console.log("");
      await new Promise(() => { });
    } else {
      if (SLOW_MODE) {
        console.log("[INFO] Pausing 3 seconds before cleanup...");
        await new Promise((resolve) => setTimeout(resolve, 3000));
      }
      console.log("=== Test passed! ===");
    }
  } catch (error) {
    console.error("=== Test failed! ===");
    console.error(`[ERROR] ${error.message}`);
    console.error(`[ERROR] Stack: ${error.stack}`);

    if (KEEP_OPEN) {
      console.log("");
      console.log("Test failed but KEEP_OPEN=true, window will stay open for inspection");
      console.log("Press Ctrl+C to close");
      await new Promise(() => { });
    }
    throw error;
  } finally {
    if (!KEEP_OPEN) {
      await cleanup(context);
    }
  }
}

/**
 * Clean up test resources.
 * @param {Object} context - Test context with driver, driverProc, viteProc
 */
export async function cleanup({ driver, driverProc, viteProc }) {
  console.log("[DEBUG] Cleanup starting");

  if (driver) {
    console.log("[DEBUG] Quitting WebDriver");
    await driver.quit();
    console.log("[DEBUG] WebDriver quit complete");
  }

  if (driverProc) {
    await cleanupProcess(driverProc, "driver");
  }

  if (viteProc) {
    await cleanupViteServer(viteProc);
  }

  console.log("[DEBUG] Cleanup complete");
}

// =============================================================================
// Timing Helpers
// =============================================================================

/**
 * Add a delay if SLOW_MODE is enabled.
 * @param {number} ms - Delay in milliseconds
 * @param {boolean} slowMode - Whether slow mode is enabled
 * @returns {Promise<void>}
 */
export async function slowModeDelay(ms, slowMode = true) {
  if (slowMode) {
    await new Promise((resolve) => setTimeout(resolve, ms));
  }
}
