# E2E Test Framework

Reusable utilities for writing Tauri e2e tests using Selenium WebDriver.

## Overview

The e2e framework (`e2e-framework.js`) provides a complete infrastructure for testing Tauri applications with WebDriver. It handles:

- **Driver management**: Starting/stopping tauri-driver and WebKitWebDriver
- **Vite dev server**: Starting/stopping the dev server with proper cleanup
- **Mock workspaces**: Creating temporary git repositories for testing
- **WebDriver sessions**: Creating and configuring WebDriver instances
- **Build management**: Building Tauri binaries with custom features
- **Process cleanup**: Properly cleaning up all spawned processes
- **Test utilities**: SLOW_MODE and KEEP_OPEN support for debugging

## Quick Start

### Basic Test Template

```javascript
#!/usr/bin/env node
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
  console.log("=== Starting my e2e test ===");

  let driverProc;
  let driver;
  let viteProc;
  const context = { driver, driverProc, viteProc };

  await runTest(async (ctx, { KEEP_OPEN, SLOW_MODE }) => {
    // 1. Create mock workspace
    const workspace = createMockBookWorkspace();

    // 2. Set up paths
    const scriptDir = path.dirname(new URL(import.meta.url).pathname);
    const tauriDir = path.resolve(scriptDir, "..");
    const manifestPath = path.resolve(scriptDir, "../src-tauri/Cargo.toml");
    const binaryPath = path.resolve(scriptDir, "../../target/debug/iriebook-tauri-ui");

    // 3. Build binary with e2e-mocks feature
    await buildTauriBinary(manifestPath, ["e2e-mocks"]);

    // 4. Start Vite dev server
    viteProc = await startViteServer(tauriDir);
    ctx.viteProc = viteProc;

    // 5. Start tauri-driver with workspace
    driverProc = await startDriver({ workspacePath: workspace });
    ctx.driverProc = driverProc;

    // 6. Create WebDriver session
    driver = await createWebDriverSession(binaryPath);
    ctx.driver = driver;

    // 7. Wait for app to load
    await driver.wait(until.elementLocated(By.css("body")), 20_000);
    await slowModeDelay(2000, SLOW_MODE);

    // 8. YOUR TEST LOGIC HERE
    const button = await driver.findElement(By.css("[data-testid='my-button']"));
    await button.click();

    // 9. Assert results
    const result = await driver.findElement(By.css("[data-testid='result']"));
    const text = await result.getText();
    if (text !== "Expected Value") {
      throw new Error(`Expected "Expected Value", got "${text}"`);
    }

    console.log("[SUCCESS] Test passed!");
  }, context);
}

run()
  .then(() => process.exit(0))
  .catch((err) => {
    console.error(err);
    process.exit(1);
  });
```

### Running Tests

```bash
# Normal mode (fast, auto-closes)
node scripts/e2e-my-test.js

# Slow mode (adds delays for visibility)
SLOW_MODE=true node scripts/e2e-my-test.js

# Keep window open for debugging
KEEP_OPEN=true node scripts/e2e-my-test.js

# Disable slow mode
SLOW_MODE=false node scripts/e2e-my-test.js
```

## API Reference

### Process Management

#### `startDriver(options)`
Start tauri-driver with optional environment variables.

```javascript
const driverProc = await startDriver({
  workspacePath: "/tmp/my-workspace",  // Sets IRIEBOOK_WORKSPACE
  env: { MY_VAR: "value" }             // Additional env vars
});
```

#### `startViteServer(cwd, port = "1420")`
Start Vite dev server and wait for it to be ready.

```javascript
const viteProc = await startViteServer("/path/to/tauri-ui", "1420");
```

#### `cleanupProcess(proc, label, timeout = 2000)`
Clean up a spawned process and its streams.

```javascript
await cleanupProcess(viteProc, "vite");
```

#### `killStaleProcesses(...processNames)`
Kill stale processes by name before starting tests.

```javascript
await killStaleProcesses("WebKitWebDriver", "tauri-driver", "iriebook-tauri-ui");
```

### WebDriver

#### `createWebDriverSession(binaryPath)`
Create a WebDriver session for the Tauri app.

```javascript
const driver = await createWebDriverSession("/path/to/binary");
```

### Build Management

#### `buildTauriBinary(manifestPath, features = [])`
Build Tauri binary with specified Cargo features.

```javascript
await buildTauriBinary("/path/to/Cargo.toml", ["e2e-mocks", "dev"]);
```

### Mock Workspaces

#### `createMockWorkspace({ prefix, setupCallback })`
Create a temporary git workspace with custom setup.

```javascript
const workspace = createMockWorkspace({
  prefix: "my-test-",
  setupCallback: (workspacePath) => {
    // Create files, commits, etc.
    const file = path.join(workspacePath, "test.txt");
    fs.writeFileSync(file, "content");
    spawnSync("git", ["add", "."], { cwd: workspacePath });
    spawnSync("git", ["commit", "-m", "test"], { cwd: workspacePath });
  }
});
```

#### `createMockBookWorkspace()`
Create a mock book workspace with 3 commits and sample content. This is the default workspace used in diff tests.

```javascript
const workspace = createMockBookWorkspace();
// Returns workspace with:
// - sample-book/book.md (3 chapters)
// - sample-book/metadata.yaml
// - 3 git commits with progressive content
```

### Test Utilities

#### `runTest(testFn, context)`
Run a test with automatic cleanup and KEEP_OPEN/SLOW_MODE support.

```javascript
const context = { driver, driverProc, viteProc };

await runTest(async (ctx, { KEEP_OPEN, SLOW_MODE }) => {
  // Test logic
  // ctx.driver, ctx.viteProc, ctx.driverProc will be cleaned up automatically
}, context);
```

#### `cleanup({ driver, driverProc, viteProc })`
Clean up test resources manually (usually not needed with `runTest`).

```javascript
await cleanup({ driver, driverProc, viteProc });
```

#### `slowModeDelay(ms, slowMode = true)`
Add a delay if SLOW_MODE is enabled.

```javascript
await slowModeDelay(2000, SLOW_MODE);  // 2s delay in slow mode, instant otherwise
```

### Port Utilities

#### `waitForPort(port, label, timeout = 30000)`
Wait for a TCP port to become available.

```javascript
await waitForPort(1420, "vite", 30000);
```

## Best Practices

### 1. Use `runTest` for Automatic Cleanup

Always use `runTest` to ensure proper cleanup even if test fails:

```javascript
await runTest(async (ctx, { KEEP_OPEN, SLOW_MODE }) => {
  // Your test logic
  // Cleanup happens automatically
}, context);
```

### 2. Update Context as You Go

Update the context object as you create resources so cleanup works properly:

```javascript
viteProc = await startViteServer(tauriDir);
ctx.viteProc = viteProc;  // Important!

driverProc = await startDriver({ workspacePath: workspace });
ctx.driverProc = driverProc;  // Important!
```

### 3. Use Slow Mode for Debugging

Add delays for visibility when debugging:

```javascript
await slowModeDelay(1500, SLOW_MODE);  // Wait 1.5s in slow mode
```

### 4. Use `data-testid` Attributes

Add `data-testid` attributes to components for reliable selectors:

```typescript
// Component
<button data-testid="submit-button">Submit</button>

// Test
const button = await driver.findElement(By.css("[data-testid='submit-button']"));
```

### 5. Create Custom Mock Workspaces

For domain-specific tests, create custom mock workspaces:

```javascript
function createMockTranslationWorkspace() {
  return createMockWorkspace({
    prefix: "translation-test-",
    setupCallback: (workspace) => {
      const bookDir = path.join(workspace, "my-book");
      mkdirSync(bookDir, { recursive: true });

      writeFileSync(
        path.join(bookDir, "en.md"),
        "# English Version\n\nHello world"
      );

      spawnSync("git", ["add", "."], { cwd: workspace });
      spawnSync("git", ["commit", "-m", "feat: add english version"], { cwd: workspace });
    }
  });
}
```

## Environment Variables

- `KEEP_OPEN` - Keep window open after test (default: `false`)
- `SLOW_MODE` - Add delays for visibility (default: `true`)
- `TAURI_DRIVER_PORT` - tauri-driver port (default: `9555`)
- `WEBKIT_WEBDRIVER_PORT` - WebKitWebDriver port (default: `9556`)
- `TAURI_DRIVER_URL` - tauri-driver URL (default: `http://127.0.0.1:9555`)
- `WEBKIT_WEBDRIVER_PATH` - WebKitWebDriver path (default: `/usr/bin/WebKitWebDriver`)
- `TAURI_DRIVER_PATH` - tauri-driver path (default: `tauri-driver` from PATH)

## Troubleshooting

### Test Hangs After Completion

Make sure you're using `runTest` which handles cleanup and process exit:

```javascript
await runTest(async (ctx, { KEEP_OPEN, SLOW_MODE }) => {
  // Test logic
}, context);
```

### WebDriver Connection Errors

The framework automatically kills stale processes before starting. If you still have issues:

```bash
# Manually kill processes
pkill -x WebKitWebDriver
pkill -x tauri-driver
pkill -f "vite.*--port.*1420"
```

### Port Already in Use

Change the port using environment variables:

```bash
TAURI_DRIVER_PORT=9666 WEBKIT_WEBDRIVER_PORT=9667 node scripts/e2e-my-test.js
```

### Build Failures

Make sure you have the required features in Cargo.toml:

```toml
[features]
e2e-mocks = []
```

## Example: Creating a New Test

Here's how to create a test for a new feature (e.g., testing book search):

```javascript
#!/usr/bin/env node
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
  console.log("=== Starting book search e2e test ===");

  let driverProc;
  let driver;
  let viteProc;
  const context = { driver, driverProc, viteProc };

  await runTest(async (ctx, { KEEP_OPEN, SLOW_MODE }) => {
    // Standard setup
    const workspace = createMockBookWorkspace();
    const scriptDir = path.dirname(new URL(import.meta.url).pathname);
    const tauriDir = path.resolve(scriptDir, "..");
    const manifestPath = path.resolve(scriptDir, "../src-tauri/Cargo.toml");
    const binaryPath = path.resolve(scriptDir, "../../target/debug/iriebook-tauri-ui");

    await buildTauriBinary(manifestPath, ["e2e-mocks"]);
    viteProc = await startViteServer(tauriDir);
    ctx.viteProc = viteProc;
    driverProc = await startDriver({ workspacePath: workspace });
    ctx.driverProc = driverProc;
    driver = await createWebDriverSession(binaryPath);
    ctx.driver = driver;

    // Wait for app
    await driver.wait(until.elementLocated(By.css("body")), 20_000);
    await new Promise((resolve) => setTimeout(resolve, 3000));
    await slowModeDelay(2000, SLOW_MODE);

    // TEST: Search for a book
    const searchInput = await driver.findElement(By.css("[data-testid='book-search']"));
    await searchInput.sendKeys("Sample");
    await slowModeDelay(1000, SLOW_MODE);

    // Verify search results
    const results = await driver.findElements(By.css("[data-testid='book-item']"));
    if (results.length === 0) {
      throw new Error("No search results found");
    }

    const firstResult = await results[0].getText();
    if (!firstResult.includes("Sample Book")) {
      throw new Error(`Expected "Sample Book" in results, got: ${firstResult}`);
    }

    console.log("[SUCCESS] Book search test passed!");
  }, context);
}

run()
  .then(() => process.exit(0))
  .catch((err) => {
    console.error(err);
    process.exit(1);
  });
```

Save this as `scripts/e2e-search-test.js` and run:

```bash
node scripts/e2e-search-test.js
```

## Next Steps

1. Add more mock workspace templates for different test scenarios
2. Add assertion helper functions
3. Add screenshot capture utilities
4. Add performance timing helpers
5. Add test report generation
