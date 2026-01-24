# E2E Testing with Selenium + tauri-driver

This document explains the E2E testing setup for iriebook-tauri-ui and how dependencies are isolated from production builds.

## Testing Framework: Selenium Only

**IMPORTANT**: We use **selenium-webdriver + tauri-driver** exclusively for E2E tests. Playwright was removed because it doesn't natively support Tauri's wry webview (they speak different protocols).

### Why Selenium?

- ✅ **Native WebDriver support** - tauri-driver is a WebDriver server, selenium-webdriver is a WebDriver client
- ✅ **No protocol adapters needed** - they already speak the same language
- ✅ **Works out of the box** - no complex configuration required
- ✅ **Headed mode by default** - you can see the app window during tests

### Why Not Playwright?

- ❌ Playwright expects Chrome DevTools Protocol or similar
- ❌ tauri-driver speaks WebDriver protocol
- ❌ Would require custom adapters/bridge to connect them
- ❌ Unnecessary complexity when Selenium works perfectly

## Architecture

The E2E tests use **the production application** with mocked resource access, not a separate test app. This ensures we test the real UI, commands, and workflows.

**Test Stack:**
```
selenium-webdriver (Node.js)
    ↓ WebDriver Protocol
tauri-driver (port 9555)
    ↓ WebDriver Protocol
WebKitWebDriver (port 9556)
    ↓ Native Control
Tauri App (wry webview)
```

### Dependency Separation

**JavaScript/TypeScript:**
- ✅ `selenium-webdriver` is in `devDependencies` (not bundled in production)
- ✅ Test scripts are in `scripts/` directory (not part of the app bundle)

**Rust:**
- ✅ `iriebook-test-support` is marked as `optional = true` in `Cargo.toml`
- ✅ Only included when building with `--features e2e-mocks`
- ✅ Production builds (without the feature) have ZERO test dependencies

```toml
# In src-tauri/Cargo.toml
[features]
e2e-mocks = ["dep:iriebook-test-support"]  # Only included when this feature is enabled

[dependencies]
iriebook-test-support = { path = "../../iriebook-test-support", optional = true }
```

### Verification

```bash
# Production build - NO test dependencies
cargo tree -p iriebook-tauri-ui | grep test-support
# Output: (nothing)

# E2E build - test dependencies included
cargo tree -p iriebook-tauri-ui --features e2e-mocks | grep test-support
# Output: ├── iriebook-test-support v0.1.0
```

## Running E2E Tests

### Quick Start

```bash
# Run with visible UI (recommended for development)
./run_e2e_with_ui.sh

# Or run individual tests
npm run test:e2e        # Basic smoke test
npm run test:e2e:diff   # Mocked state initialization test
```

### What Happens

Both tests will:
1. Kill any stale driver processes
2. Build the app with `cargo build --features e2e-mocks` (always rebuilds)
3. Start `tauri-driver` (port 9555) and `WebKitWebDriver` (port 9556)
4. Launch the app via WebDriver session (Tauri auto-starts Vite dev server)
5. Run the test (verify window loads, UI elements present, etc.)
6. Clean up processes and close the app

**All tests run in headed mode by default** - you'll see the app window open and the tests interacting with it. This is perfect for debugging and development.

**Note**: Tests always rebuild to ensure e2e-mocks is enabled. The Vite dev server starts automatically when the app launches.

### Build Approach

E2E tests use the **debug build with e2e-mocks feature**:

```bash
cargo build --features e2e-mocks
# Builds to: target/debug/iriebook-tauri-ui
```

The `e2e-mocks` feature is defined in `src-tauri/Cargo.toml`:

```toml
[features]
e2e-mocks = ["dep:iriebook-test-support"]
```

This ensures:
- ✅ Same binary location as normal dev builds (`target/debug/`)
- ✅ Tauri auto-detects to use dev server in debug mode
- ✅ Mocked resource access only included when feature is enabled
- ✅ Production builds have ZERO test code (feature not enabled by default)

**Note**: The test scripts always rebuild to ensure the e2e-mocks feature is enabled.

## System Requirements

### Linux (tested)
- `webkit2gtk-driver` package (provides WebKitWebDriver)
- `tauri-driver` (install with `cargo install tauri-driver`)

### macOS / Windows
- Native WebDriver for the platform (Safari Driver / Edge Driver)
- `tauri-driver` (install with `cargo install tauri-driver`)

**Note**: tauri-driver produces no console output on success - it just starts listening on the port. The test scripts poll port 9555 to detect when it's ready.

### Expected "Error" Messages

You may see this harmless error during test runs:

```
[driver] Error serving connection: hyper::Error(User(Service), client error (Connect)
Caused by:
    0: tcp connect error: Connection refused (os error 111)
    1: Connection refused (os error 111))
```

This is **expected and harmless** - it's tauri-driver trying to connect to WebKitWebDriver before it's fully ready. It's a race condition during startup that gets logged but doesn't affect test execution. The test will continue and complete successfully.

## Key Files

- `scripts/run-tauri-driver-e2e.js` - Basic smoke test (window loads)
- `scripts/e2e-diff-test.js` - Mocked state initialization test
- `src-tauri/src/infrastructure.rs` - `init_mock_state` command (only available with `e2e-mocks` feature)
- `src-tauri/src/state.rs` - `#[cfg(feature = "e2e-mocks")]` provides mock resource access

## Debug Output

The test script includes comprehensive debug logging. Look for these markers:
- `[DEBUG]` - Internal script flow
- `[driver]` - tauri-driver output
- `[tauri]` - Tauri app output (when using npm-based launch)

## Future Improvements

### More Comprehensive Tests
- [ ] Test book scanning with mocked file system
- [ ] Test git operations with mocked git access
- [ ] Test diff view navigation and display
- [ ] Test metadata editing workflow
- [ ] Test processing/publication workflow
- [ ] Test Google Docs integration flows

### Test Infrastructure
- [ ] Configure mocks to read from test workspace (currently they return hardcoded data)
- [ ] Add headless mode support for CI (set `MOZ_HEADLESS=1` or similar)
- [ ] Screenshot capture on test failure
- [ ] Video recording of test sessions

### CI/CD
- [ ] GitHub Actions workflow with xvfb for headless testing
- [ ] Automated test runs on PR
- [ ] Test result reporting

## Design Philosophy

### Why Selenium + Mocked Resource Access?

This approach combines the best of both worlds:

**Full-Stack Integration Testing:**
- ✅ Tests the **real** Tauri UI (not just components)
- ✅ Tests the **real** Tauri commands (not just mocks)
- ✅ Tests the **real** state management and data flow
- ✅ Sees the app exactly as users see it

**Deterministic & Fast:**
- ✅ Mocked resource access = no external dependencies (git, file system, etc.)
- ✅ Tests are fast and don't require network access
- ✅ Tests are deterministic - same inputs = same outputs
- ✅ No flaky tests due to external service issues

**Clean Separation:**
- ✅ Mock implementations live in `iriebook-test-support` (optional dependency)
- ✅ Production builds have ZERO test code or dependencies
- ✅ E2E builds use separate profile and binary path

### Alternatives We Rejected

**Playwright**: Doesn't speak WebDriver protocol, would need adapters
**Component testing only**: Wouldn't catch integration issues
**Real resource access**: Tests would be slow, flaky, and hard to set up
**Separate test app**: Wouldn't test the real production code path
