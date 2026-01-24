# Tauri + Playwright E2E Plan (Mocked Resources)

- [x] Phase 1: Rename workflow tests for clarity
  - [x] Move `iriebook/tests/e2e/` → `iriebook/tests/workflows/`
  - [x] Rename `e2e_tests.rs` → `workflow_tests.rs`; update `#[path]`, imports, and docs to “workflow integration (mocked externals, no UI/Tauri)”

- [x] Phase 2: Extract shared test support
  - [x] Create workspace crate `iriebook-test-support` (feature `e2e-mocks`)
  - [x] Export fixtures (`TestWorkspace`, `TestBook`) and mocks (`MockGitAccess`, `MockGoogleDocsAccess`, `MockPandocAccess`, `MockCalibreAccess`, `MockArchiveAccess`, call trackers)
  - [x] Update workflow tests to consume the new crate

- [x] Phase 3: Enable mocked AppState for Tauri
  - [x] Add env/feature gate (`E2E_MOCKS=1` or Cargo feature) to build `AppState` via `AppStateBuilder` with mocks from `iriebook-test-support`
  - [x] Expose a minimal Tauri command to initialize with mocks and supplied workspace path (no business logic in command)

- [x] Phase 4: Playwright infrastructure in `iriebook-tauri-ui`
  - [x] Add dev dep `@playwright/test` and `test:e2e` script
  - [x] Create Playwright config (current: webserver smoke; TODO: reintroduce Tauri window + mocks)

- [ ] Phase 5: Playwright specs (initial set)
  - [ ] Init mocks smoke: load app and invoke mock initialization (pending driver-based wiring)
  - [ ] Git: status/sync/save UI flow against mocks
  - [ ] Google Docs: list/link/unlink UI against mocks
  - [ ] Publication: trigger flow, assert completion/toast/state using mocked Pandoc/Calibre
  - [ ] Diff: open diff view, verify mocked diff content

- [ ] Phase 6: Verification
  - [ ] Run Rust workflow suite: `cargo test -p iriebook --test workflow_tests`
  - [ ] Run UI E2E: `npm run test:e2e`

## Notes
- Keep headless Rust workflow tests as fast safety net; Playwright adds UI coverage.
- Mock injection gated by env/feature to keep production untouched.
- Reuse fixture data for both Rust and Playwright to avoid divergence.
