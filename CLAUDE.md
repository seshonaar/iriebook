# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

IrieBook is an ebook publication pipeline for converting manuscripts into professional ebooks. It handles quote curling, whitespace cleanup, word analysis, and generates ebooks via Calibre and Pandoc. The project consists of multiple crates with different UI implementations.

## Crate Architecture

This is a **Cargo workspace** with the following crates:

- **iriebook**: Core library implementing the publication pipeline
  - Organized by volatility using the Righting Software Method:
    - `client/`: CLI presentation layer
    - `managers/`: Workflow orchestration for multi-step operations
      - `ebook_publication.rs`: Ebook generation workflow (quote fixing, validation, Pandoc/Calibre)
      - `repository_manager.rs`: Git workflow orchestration (clone, sync, save, status)
      - `google_docs_sync.rs`: Google Docs sync workflow (link, sync, unlink)
    - `engines/`: Business logic organized by volatility domain:
      - `text_processing/`: Quote fixing, whitespace cleanup, markdown
      - `analysis/`: Word frequency analysis
      - `validation/`: Quote validation
    - `resource_access/`: External resource abstractions (files, config, Calibre, Pandoc, archives, Git, Google Docs)
    - `utilities/`: Cross-cutting concerns (types, errors)

- **iriebook-ui-common**: Framework-agnostic UI utilities shared across all UI implementations
  - `book_scanner`: Scanning directories for book files
  - `session`: Session persistence
  - `ui_state`: Application state management
  - `processing`: Async book processing orchestration
  - `image_loading`: Cover image loading and thumbnails

- **iriebook-tauri-ui**: Cross-platform desktop GUI (Tauri + React + TypeScript)
  - `src/`: React/TypeScript frontend with i18n support
  - `src-tauri/src/`: Thin Rust backend layer (Tauri commands only)
  - Uses specta for type-safe TypeScript bindings
  - REPHRAIN FROM ADDING BUSINESS LOGIC HERE THIS IS AN UI IMPLEMENTATION THAT IS HIGHLY VOLATILE. BUSINESS LOGIC BELONGS TO THE core or ui-common crates.

## Common Commands

### Building
```bash
# Build entire workspace
cargo build

# Build specific crate
cargo build -p iriebook
cargo build -p iriebook-cosmic-ui
cargo build -p iriebook-ui-common

# Release build with optimizations
cargo build --release
```

### Testing
```bash
# Run all tests in workspace
cargo test

# Run tests for specific crate
cargo test -p iriebook
cargo test -p iriebook-ui-common

# Run a specific test
cargo test -p iriebook test_name

# Run integration tests
cargo test --test integration_tests
```

### Linting and Validation
```bash
# Run clippy on all workspace members
cargo clippy --workspace

# Run clippy on specific crate
cargo clippy -p iriebook

# Check without building
cargo check --workspace
```

### Running
```bash
# Run CLI tool
cargo run -p iriebook -- <args>

# Run COSMIC GUI
cargo run -p iriebook-cosmic-ui

# Run COSMIC GUI using justfile (in iriebook-cosmic-ui directory)
just run
```

### Tauri UI Specific Commands (using just)
```bash
# Navigate to tauri-ui directory first
cd iriebook-tauri-ui

# Run in development mode (auto-reloads on changes)
just dev

# Build release version
just build

# Generate TypeScript bindings from Rust
just gen-bindings

# Clean build artifacts
just clean
```

## Testing Philosophy

- **TDD approach**: Write tests first for new functionality
- Unit tests are inline with `#[cfg(test)]` modules in most source files
- Integration tests are in the `tests/` directory
- Property-based testing uses `proptest` for validation logic
- Use `tempfile` for tests requiring filesystem operations

## Key Dependencies

### Rust Crates (Workspace)
- **anyhow**: Error handling (workspace dependency)
- **thiserror**: Custom error types (workspace dependency)
- **serde**: Serialization (workspace dependency)
- **tokio**: Async runtime (workspace dependency, "full" features)

### External Tools (Required for ebook generation)
- **Calibre**: Command-line tool for Kindle (AZW3) conversion
- **Pandoc**: Command-line tool for EPUB generation with custom CSS

## Architecture Patterns

### Module Organization
- Uses simplified module system (no `mod.rs` files, Rust 2024 edition)
- Module files declared in parent with `pub mod foo;`, implementation in `foo.rs`

### Resource Access Layer
All external dependencies (filesystem, CLI tools, config) are abstracted in `resource_access/`:
- **Trait-based abstraction** (`traits.rs`) for testing and flexibility
- `calibre.rs`: Calibre CLI interaction
- `pandoc.rs`: Pandoc CLI interaction
- `file.rs`: File I/O operations
- `archive.rs`: ZIP file handling
- `config.rs`: Configuration management

This allows mocking external dependencies in tests.

### UI Common Layer
`iriebook-ui-common` provides framework-agnostic utilities so UI implementations can share:
- Book discovery logic (`book_scanner.rs`)
- Session state persistence (`session.rs`)
- Processing orchestration (`processing.rs`)
- Image/cover handling (`image_loading.rs`)
- UI state management (`ui_state.rs`)

## Architectural Guidelines

**Strict Separation of Concerns:**
- **iriebook-tauri-ui**:
  - **Frontend (src/)**: ONLY React/TypeScript UI components and i18n. No business logic.
  - **Backend (src-tauri/src/)**: ONLY thin Tauri command wrappers. All business logic delegated to `iriebook` managers or `iriebook-ui-common`.
  - **Commands should be ~5-10 lines**: Create manager/client, call method, return result.
- **iriebook-ui-common**: Any state management, file scanning, image processing, or business logic that could be reused in another UI (e.g., a web app or TUI).
- **iriebook**: Core domain logic, file I/O, external tool wrappers (Pandoc/Calibre), and functionality required for the CLI.
  - **managers/**: Workflow orchestration (multi-step operations like sync, publication, repository management)
- **Rule of Thumb**: If you can write a test for it without mocking a UI widget or Tauri runtime, it belongs in `iriebook-ui-common` or `iriebook`.

## Enforcement: Preventing Volatility Violations

**CRITICAL**: The Tauri layer must be thin and replaceable. Follow these rules strictly to maintain architectural integrity.

### Tauri Command Rules (MANDATORY)

**ALL Tauri commands MUST be thin wrappers (~5-10 lines).** Before adding or modifying a command:

#### Mandatory Pattern
```rust
#[tauri::command]
#[specta::specta]
async fn my_command(
    state: State<'_, AppState>,
    param: String,
) -> Result<ReturnType, String> {
    let manager = state.some_manager();
    ui_common::module::function(param, &manager).await
}
```

#### Pre-Commit Checklist
- [ ] No `Arc::new(...)` in command
- [ ] No `Manager::new(...)` construction
- [ ] No loops or orchestration logic
- [ ] No file filtering or business rules
- [ ] Command is < 15 lines
- [ ] Logic extracted to `iriebook-ui-common` or `iriebook`

### AppState Pattern

**Single Source of Managers:**

All managers are initialized once in `iriebook-ui-common/src/app_state.rs` and cached:

```rust
pub struct AppState {
    repository_manager: Arc<RepositoryManager>,
    google_docs_manager: Arc<GoogleDocsSyncManager>,
    diff_manager: Arc<DiffManager>,
    github_authenticator: Arc<GitHubAuthenticator>,
    google_authenticator: Arc<GoogleAuthenticator>,
    google_docs_client: Arc<GoogleDocsClient>,
}
```

**Usage in Tauri Commands:**
```rust
state: State<'_, AppState>
let manager = state.repository_manager(); // Returns Arc<RepositoryManager>
```

**NEVER instantiate managers in Tauri commands.** All manager construction happens in `AppState::new()`.

### Workflow Orchestration Rules

**Orchestration belongs in `iriebook-ui-common` or `iriebook` managers:**

- ✅ **Correct**: `BatchProcessor::process_books()` in `iriebook-ui-common/src/batch_processing.rs`
- ❌ **Wrong**: Processing loop in Tauri `start_processing` command

- ✅ **Correct**: `DiffManager::get_revision_changes()` in `iriebook/src/managers/diff_manager.rs`
- ❌ **Wrong**: File filtering and diff loop in Tauri `git_get_revision_diffs` command

**If you find yourself writing a loop or conditional workflow in a Tauri command, STOP and move it to ui-common or core.**

### Code Review Red Flags

❌ **REJECT PRs that:**
- Add `Arc::new()` in `lib.rs`
- Implement loops in Tauri commands
- Duplicate existing manager methods
- Add business logic to Tauri layer
- Have commands > 20 lines
- Filter data (e.g., `.filter(|f| f.ends_with(".md"))`) in commands

✅ **REQUIRE:**
- Extract to `iriebook-ui-common` module
- Use existing manager methods
- Delegate via AppState
- Keep commands as thin translation layers

### Why This Matters

**The Tauri layer is disposable.** If we switch to 'bouri', 'iced', or a web framework:
- Only `lib.rs` should need rewriting
- All business logic survives by living in `iriebook-ui-common` or `iriebook` core
- AppState, BatchProcessor, and all managers remain unchanged

**Example violations and fixes:**

| Violation | Fix |
|-----------|-----|
| `let repo_manager = RepositoryManager::new(Arc::new(GitClient));` | Use `state.repository_manager()` |
| Batch processing loop in command | Move to `BatchProcessor::process_books()` |
| `.filter(\|f\| f.ends_with(".md"))` in command | Use `DiffManager::get_revision_changes(_, Some(".md"))` |

See `design_compliance_report.md` for detailed analysis of historical violations.

## TypeScript Bindings (Tauri UI)

The Tauri UI uses **specta + tauri-specta** to automatically generate type-safe TypeScript bindings from Rust code. This provides complete type safety between the Rust backend and TypeScript frontend, including compile-time validation of command names, parameters, return types, and events.

### How Bindings Work

1. **Define Types in Rust** (`iriebook-ui-common/src/`)
   - Add `#[derive(Serialize, Type)]` to your struct/enum
   - Example:
     ```rust
     use serde::Serialize;
     use specta::Type;

     #[derive(Serialize, Type)]
     pub struct CoverImageData {
         pub data_url: String,
         pub width: u32,
         pub height: u32,
     }
     ```

2. **Define Commands** (`iriebook-tauri-ui/src-tauri/src/lib.rs`)
   - Add `#[specta::specta]` alongside `#[tauri::command]`
   - Example:
     ```rust
     #[tauri::command]
     #[specta::specta]
     fn load_cover_image(cover_path: Option<String>) -> Result<CoverImageData, String> {
         // implementation
     }
     ```

3. **Define Events** (`iriebook-ui-common/src/processing.rs`)
   - Use `#[derive(Event)]` for type-safe events
   - Example:
     ```rust
     use tauri_specta::Event;

     #[derive(Serialize, Type, Event)]
     pub struct ProcessingUpdateEvent(pub ProcessingEvent);

     // Emit events type-safely
     ProcessingUpdateEvent(event).emit(&app)?;
     ```

4. **Bindings Auto-Generate**
   - Run `npm run tauri dev` or `npm run tauri build`
   - Bindings are automatically generated to `iriebook-tauri-ui/src/bindings.ts`
   - Only generated in debug builds (production uses pre-generated file)

5. **Use in TypeScript** (`iriebook-tauri-ui/src/`)
   ```typescript
   import { commands, events, type CoverImageData } from "../bindings";

   // Type-safe command calls with Result pattern
   const result = await commands.loadCoverImage(coverPath);
   if (result.status === "error") {
     throw new Error(result.error);
   }
   const coverData = result.data;

   // Type-safe event listeners
   events.processingUpdateEvent.listen((event) => {
     console.log("Processing update:", event.payload);
   });
   ```

### Key Features

- **Command Type Safety**: Command names, parameter types, and return types are validated at compile time
- **Result Pattern**: All commands return `Result<T, E>` for explicit error handling
- **Event Type Safety**: Event names and payload types are validated at compile time
- **Single Source of Truth**: Types defined once in Rust, automatically available in TypeScript
- **IDE Support**: Full autocomplete for commands, events, and types

### Important Notes

- **Result Handling**: Commands return `{ status: "ok", data: T }` or `{ status: "error", error: E }`. Always check `.status` before accessing `.data`
- **Event Names**: Auto-converted to camelCase (e.g., `ProcessingUpdateEvent` → `events.processingUpdateEvent`)
- **Type Overrides**: Use `#[specta(type = ...)]` to override type inference (e.g., `usize` → `u32`, `PathBuf` → `String`)
- **NewType Pattern**: Use `#[specta(transparent)]` for NewType wrappers to map to inner type
- **Type Location**: Put all shared types in `iriebook-ui-common` so they're available to both Rust and TypeScript
- **Regenerating Bindings**: Automatically regenerated when running `npm run tauri dev` or `npm run tauri build`

## Internationalization (i18n) - Tauri UI

The Tauri UI uses **react-i18next** for internationalization. All user-facing strings must be externalized to translation files.

### Translation File Structure

Translation files are organized by feature in `src/i18n/locales/en/`:

```
src/i18n/locales/en/
├── common.json          # Common UI elements (buttons, labels)
├── books.json           # Book list, book operations
├── metadata.json        # Book metadata fields
├── processing.json      # Processing/publication workflow
├── git.json            # Git/GitHub integration
├── google.json         # Google Docs integration
├── dialogs.json        # Dialog messages
├── toasts.json         # Toast notifications
├── errors.json         # Error messages
├── menu.json           # Menu items
└── log.json            # Log viewer
```

### Adding New Features with i18n

When adding a new feature to the Tauri UI:

1. **Create Translation File** (if new feature domain)
   - Create `src/i18n/locales/en/feature-name.json`
   - Organize translations hierarchically by component/action

2. **Translation File Structure**
   ```json
   {
     "componentName": {
       "action": "Button Text",
       "label": "Field Label",
       "messages": {
         "success": "Operation succeeded",
         "error": "Operation failed: {{details}}"
       }
     }
   }
   ```

3. **Use Translations in Components**
   ```typescript
   import { useTranslation } from "react-i18next";

   export function MyComponent() {
     const { t } = useTranslation();

     return (
       <div>
         <h1>{t("featureName.componentName.label")}</h1>
         <button>{t("featureName.componentName.action")}</button>
         {/* With variables */}
         <p>{t("featureName.componentName.messages.error", { details: error })}</p>
       </div>
     );
   }
   ```

4. **Translation Key Format**
   - Use dot notation: `"namespace.path.to.key"`
   - Examples:
     - `"google.auth.title"` → Google Docs authentication title
     - `"git.sync.messages.syncSuccess"` → Git sync success message
     - `"books.list.selectAll"` → Select all books checkbox

### Best Practices

- **Never hardcode user-facing strings** in JSX/TSX components
- **Group related translations** under common parent keys
- **Use descriptive key names** that indicate context (e.g., `button.submit` vs just `submit`)
- **Support pluralization** using i18next plural forms when needed
- **Include context in keys** (e.g., `dialog.title`, `button.confirm`, `messages.success`)
- **Use interpolation** for dynamic values: `"Welcome {{name}}"` accessed as `t("key", { name })`

### Example: Google Docs Integration

```json
{
  "auth": {
    "title": "Google Docs",
    "connected": "Connected to Google Docs",
    "notConnected": "Not connected to Google Docs",
    "connect": "Connect Google Docs",
    "disconnect": "Disconnect",
    "deviceFlow": {
      "enterCode": "Enter this code in your browser:",
      "waiting": "Waiting for authorization..."
    }
  },
  "sync": {
    "button": {
      "link": "Link to Google Doc",
      "sync": "Sync from Google Docs",
      "lastSynced": "Last synced: {{time}}"
    },
    "messages": {
      "syncSuccess": "Synced successfully",
      "linkFailed": "Link failed"
    }
  }
}
```

### Future Language Support

To add additional languages:
1. Create `src/i18n/locales/[lang-code]/` directory
2. Copy all JSON files from `en/` directory
3. Translate strings while preserving keys and interpolation variables
4. Update `src/i18n/config.ts` to include new language
