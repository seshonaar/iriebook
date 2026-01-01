# IrieBook 🎺

> *"Mon, yuh wife write AMAZING vampire stories about Bucharest! IrieBook be di complete pipeline fi clean up dem quotes, format di markdown, and publish beautiful ebooks. One irie flow from Google Docs to Kindle!"*

## What Dis Do?

Your wife exports beautiful books from Google Docs to markdown, but the quotes are all messed up - mixing straight quotes (`"`) with curly quotes (`"` and `"`), or worse, using them inconsistently!

**IrieBook** is a complete ebook publication pipeline that:

### Text Processing
- ✅ Converts ALL straight double quotes (`"`) to proper curly quotes (`"` and `"`)
- ✅ **Smart apostrophe handling** - converts legitimate apostrophes (`'`) to curly (`'`)
- ✅ Detects contractions, possessives, abbreviated years (`'70s`), and omitted letters (`'cause`)
- ✅ **Strictly validates** - refuses to process if actual dialogue single quotes found
- ✅ Ensures double quotes are balanced (even number)
- ✅ **Whitespace trimming** - cleans up excessive spacing automatically
  - Collapses multiple consecutive spaces to single space
  - Converts tabs to single space
  - Limits consecutive blank lines to max 1
  - Trims leading/trailing whitespace from lines
- ✅ **Markdown transformation** - Restructures chapter headings and scene breaks for EPUB
- ✅ Preserves `*italics*`, `**bold**`, and other markdown formatting
- ✅ Handles Romanian UTF-8 characters perfectly (ă, â, î, ș, ț)
- ✅ **UTF-8 BOM handling** - Strips BOM from input, never writes BOM to output

### Analysis & Publishing
- ✅ **Word frequency analysis** - With Romanian stopword filtering
- ✅ **EPUB generation** - Automatic via pandoc with custom CSS
- ✅ **Kindle conversion** - Automatic AZW3 generation via calibre
- ✅ **Metadata stamping** - Series information applied to ebooks
- ✅ **ZIP archiving** - Complete book package for distribution
- ✅ Lightning fast - processes 800KB in **< 20 milliseconds**

## Installation

```bash
cd /home/andrei/work/ebook_processing/tools/iriebook
cargo build --release
```

The binary will be at `./target/release/iriebook`

## Usage

### Basic (Wife-Approved Simple!)

```bash
iriebook book.md
```

Creates `book-fixed.md` in the same directory with all quotes converted to curly.

### Check Before Converting

```bash
iriebook --dry-run book.md
```

Validates the file without writing output. Perfect for checking if there are any issues.

### See What's Happening

```bash
iriebook -v book.md
```

Verbose mode shows detailed progress.

### Custom Output Location

```bash
iriebook -o final.md book.md
```

Write to a custom location instead of the default `-fixed` suffix.

### Get Help

```bash
iriebook --help
```

## Real Example

**Before IrieBook:**
```markdown
  She  said  "hello"  and  he  replied  "goodbye".



It's  from  the  '70s,  John's  favorite  *vampire*  story.

The  *vampire*  whispered	"bună  seara".
```

**After IrieBook:**
```markdown
She said "hello" and he replied "goodbye".

It's from the '70s, John's favorite *vampire* story.

The *vampire* whispered "bună seara".
```

Note:
- Double quotes are now curly: `"` and `"`
- Apostrophes are now curly: `'` (in it's, '70s, John's)
- Multiple spaces collapsed to single spaces
- Tabs converted to spaces
- Excessive blank lines collapsed to single blank line
- Leading/trailing whitespace removed from lines
- Asterisks preserved: `*vampire*` unchanged
- Romanian characters perfect: `bună seara`

## Error Examples

### Single Quotation Marks (Dialogue) Found

```bash
$ iriebook book.md
❌ Validation failed!

Found 2 single quotation mark(s) used for dialogue

1. Location (line 42, column 10):
  She said 'hello' to me.
           ^

2. Location (line 42, column 16):
  She said 'hello' to me.
                 ^

These appear to be dialogue quotes, not apostrophes.
Consider using double quotes instead for dialogue.

Please fix these issues in the original file and try again.
```

**What to do:** Replace the single quotes with double quotes for dialogue: `'hello'` → `"hello"`. Then run iriebook again.

**Note:** Legitimate apostrophes (contractions, possessives, abbreviated years) are automatically handled - no manual fixes needed!

### Unbalanced Quotes

```bash
$ iriebook book.md
❌ Validation failed!

Unbalanced quotes: found 47 straight double quotes (must be even)

Location (line 1205, column 89):
  ...she said "hello but never...
                                                                                                       ^

Please fix these issues in the original file and try again.
```

**What to do:** You have an odd number of quotes - one is missing its pair. Check around line 1205.

## Smart Apostrophe Detection

IrieBook automatically recognizes and converts these legitimate apostrophes to curly style:

### ✅ Contractions
- `it's` → `it's`
- `can't` → `can't`
- `won't` → `won't`
- `we're` → `we're`

### ✅ Possessives
- `John's book` → `John's book`
- `wife's novel` → `wife's novel`
- `James' car` → `James' car` (possessive of names ending in 's')

### ✅ Abbreviated Years
- `the '70s` → `the '70s`
- `'80s music` → `'80s music`
- `back in '90` → `back in '90`

### ✅ Omitted Letters
- `'cause` → `'cause`
- `'til tomorrow` → `'til tomorrow`
- `'em` → `'em`

### ❌ Dialogue Quotes (ERROR)
- `She said 'hello'` → **ERROR - use double quotes instead**
- `'Goodbye' he whispered` → **ERROR - use double quotes instead**

## Features Wife Will Love

1. **One Command** - That's it! `iriebook book.md` and done - quotes AND whitespace fixed!
2. **Safe** - Never modifies the original file
3. **Output in Same Folder** - Easy to find `book-fixed.md`
4. **Smart Apostrophes** - Automatically handles contractions, possessives, and years (`'70s`)
5. **Clean Whitespace** - No more double spaces, tabs, or excessive blank lines
6. **Clear Errors** - Shows EXACTLY where problems are (line & column)
7. **Preserves Formatting** - Your `*italics*` and `**bold**` stay perfect
8. **Fast** - Processes 800KB in milliseconds

## Features You'll Love (Technical Stuff)

- 🦀 **Written in Rust** - Fast, safe, reliable
- ✨ **Test-Driven Development** - 118 tests, 100% passing
- 🏗️ **Righting Software Architecture** - Volatility-based decomposition for maintainability
- 🤖 **Smart Classification** - Context-aware apostrophe detection
- 🧹 **Whitespace Cleaning** - Four-rule trimming system (spaces, tabs, blank lines, line edges)
- 📊 **Word Analysis** - Frequency analysis with Romanian stopword filtering
- 📚 **EPUB/Kindle Generation** - Automatic ebook creation via pandoc & calibre
- 🔧 **Zero Clippy Warnings** - Clean, idiomatic Rust code
- 🎯 **Strict Validation** - Fail fast with helpful errors
- 💾 **Atomic Writes** - Uses temp files to prevent data loss
- 🌍 **UTF-8 Safe** - Perfect handling of Romanian and multi-byte characters
- 📝 **BOM-Free Output** - Strips BOM from input, always produces clean UTF-8 without byte order mark
- 📊 **NewType Pattern** - Can't mix up line numbers with columns
- 🚀 **Optimized** - Single-pass algorithms, minimal allocations

## Technical Details

### Algorithm

Uses a simple state machine:
- **Outside** - not in quotes
- **InsideDouble** - inside double quotes

When encountering a straight quote (`"`):
- If `Outside` → convert to opening curly quote (`"`) and enter `InsideDouble`
- If `InsideDouble` → convert to closing curly quote (`"`) and return to `Outside`

### Validation

Two-pass approach:
1. **Validation Pass** - Scan for errors (single quotes, unbalanced)
2. **Conversion Pass** - Only runs if validation passed

This ensures we never partially process a file with problems.

### Performance

Tested on real book (with both quote conversion and whitespace trimming):
- **File Size:** 823,832 bytes (804KB)
- **Processing Time:** < 20 milliseconds
- **Lines:** 1,809
- **Quotes Converted:** Depends on the file
- **Whitespace Trimmed:** Depends on the file

Well under the 100ms target! Whitespace trimming adds minimal overhead.

## Architecture

IrieBook follows **Juval Löwy's "Righting Software" Method**, organizing code by **volatility** (what changes together) rather than by function. This architectural approach ensures changes are isolated within single components.

### Five-Layer Architecture

```
┌─────────────────────────────────────────┐
│           Client (CLI)                  │  ← Presentation layer
├─────────────────────────────────────────┤
│           Manager                       │  ← Orchestration layer
├─────────────────────────────────────────┤
│           Engines                       │  ← Business logic
│  • text_processing (high volatility)   │     (grouped by volatility)
│  • analysis (different domain)         │
│  • validation (separate concern)       │
├─────────────────────────────────────────┤
│       Resource Access                   │  ← External resources
│  • file, config, pandoc, calibre       │
├─────────────────────────────────────────┤
│         Utilities                       │  ← Cross-cutting concerns
│  • types, errors                        │
└─────────────────────────────────────────┘
```

### Project Structure

```
iriebook/
├── src/
│   ├── main.rs                           # CLI entry point
│   ├── lib.rs                            # Library root
│   │
│   ├── client/                           # Presentation Layer
│   │   └── cli.rs                        # CLI display & formatting
│   │
│   ├── managers/                         # Orchestration Layer
│   │   └── ebook_publication.rs          # Workflow coordinator
│   │
│   ├── engines/                          # Business Logic (by volatility)
│   │   ├── text_processing/              # High volatility (change together)
│   │   │   ├── quote_fixer.rs            # Curly quote conversion
│   │   │   ├── whitespace_trimmer.rs     # Whitespace cleaning
│   │   │   └── markdown_transform.rs     # Markdown restructuring
│   │   ├── analysis/                     # Different volatility domain
│   │   │   └── word_analyzer.rs          # Word frequency analysis
│   │   ├── validation/                   # Separate concern
│   │   │   └── validator.rs              # Quote validation
│   │   └── traits.rs                     # Engine interfaces
│   │
│   ├── resource_access/                  # External Resources
│   │   ├── file.rs                       # File I/O operations
│   │   ├── config.rs                     # Configuration loading
│   │   ├── pandoc.rs                     # EPUB generation
│   │   ├── calibre.rs                    # Kindle conversion
│   │   ├── archive.rs                    # ZIP creation
│   │   └── traits.rs                     # Resource Access interfaces
│   │
│   └── utilities/                        # Cross-cutting Concerns
│       ├── types.rs                      # NewType wrappers
│       └── error.rs                      # Error types
│
├── tests/
│   ├── integration_tests.rs              # End-to-end CLI tests (33 tests)
│   └── fixtures/                         # Test data
├── docs/
│   └── righting-software-analysis.md     # Architecture documentation
├── Cargo.toml
└── README.md
```

### Why This Organization?

**Volatility-based decomposition** means changes are isolated:

- **UI change?** → Only `client/` changes
- **Workflow change?** → Only `managers/` changes
- **Quote algorithm change?** → Only `engines/text_processing/quote_fixer.rs` changes
- **New analysis feature?** → Add to `engines/analysis/`
- **New output format?** → Add to `resource_access/`

**No cascading changes across the system!** Changes to text processing stay in `text_processing/`, changes to analysis stay in `analysis/`, etc.

## Development

Built following Rust best practices:
- **Rust 2024 Edition**
- **Simplified module system** (no `mod.rs` files)
- **NewType pattern** for type safety
- **Prefer `match` over `if let`**
- **Zero `unwrap()`** - all errors use `?`
- **`impl Trait`** for return types
- **`anyhow` + `thiserror`** for error handling

### Running Tests

```bash
# All tests (118 unit + 33 integration = 151 total)
cargo test

# Just unit tests (118 tests)
cargo test --lib

# Just integration tests (33 tests)
cargo test --test integration_tests

# With clippy
cargo clippy -- -D warnings
```

**Test Coverage by Layer:**
- **Engines** (text_processing, analysis, validation): 74 tests
- **Resource Access** (file, config, pandoc, calibre, archive): 28 tests
- **Utilities** (types, errors): 10 tests
- **Managers**: 6 tests
- **Integration** (end-to-end CLI): 33 tests

All tests passing! ✅

### Dependencies

- **anyhow** - Ergonomic error handling
- **thiserror** - Custom error types
- **clap** - CLI argument parsing
- **tempfile** (dev) - Testing
- **assert_cmd** (dev) - CLI testing
- **predicates** (dev) - Test assertions
- **proptest** (dev) - Property-based testing

## Why "IrieBook"?

Because everyting about your wife's books gonna be IRIE after dis pipeline run! From messy Google Docs to professional Kindle ebooks - one irie flow.

Plus, wife can remember it: "Run IrieBook on my book!" And she'll love di vibe.

Irie = feeling great, peaceful, and in harmony (Jamaican Patois). Just like how her books feel after processing!

## Limitations

- **Smart but not perfect apostrophe detection** - Handles most common cases (contractions, possessives, years) but may miss edge cases
- **No nested quote handling** - Assumes standard dialogue with simple alternation
- **Markdown specific** - Designed for markdown exported from Google Docs
- **No code block awareness** - Assumes straight quotes in the file are all dialogue (not code)

If you need more advanced features, this tool might not be for you. But for cleaning up Google Docs exports? Perfect!

## Troubleshooting

### "Found single quotation marks" error

**Problem:** Your file contains single quotes used for dialogue (`'hello'`).

**Solution:** IrieBook distinguishes between legitimate apostrophes (which it converts automatically) and dialogue single quotes (which are errors). Replace dialogue single quotes with double quotes: `'hello'` → `"hello"`. Then run iriebook again.

The error message shows you EXACTLY where they are!

**Note:** Apostrophes in contractions (`it's`), possessives (`John's`), years (`'70s`), and omitted letters (`'cause`) are handled automatically - no action needed.

### "Unbalanced quotes" error

**Problem:** You have an odd number of straight quotes.

**Solution:** Every opening quote needs a closing quote. Check around the line number shown in the error message.

### File not found

**Problem:** IrieBook can't find your input file.

**Solution:** Use the full path or make sure you're in the right directory:
```bash
fixit /full/path/to/book.md
```

## About the Architecture

IrieBook implements **Juval Löwy's "Righting Software" Method**, a revolutionary approach to software design that organizes code by **volatility** (likelihood of change) rather than by function.

### Key Principles

**Volatility-Based Decomposition**: Instead of organizing by "what the system does" (functional decomposition), organize by "what is likely to change together" (volatility decomposition).

**The Five Component Types**:
1. **Client** - Presentation layer (UI, CLI, API endpoints)
2. **Manager** - Orchestrates workflows, encapsulates sequence volatility
3. **Engine** - Implements business logic, encapsulates activity volatility
4. **Resource Access** - Abstracts external resources (databases, files, APIs)
5. **Utility** - Cross-cutting concerns (logging, types, errors)

**Benefits**:
- **Isolated Changes** - Changes affect single components, not cascading changes
- **Clear Dependencies** - Strict top-to-bottom flow (Client → Manager → Engine → Resource Access)
- **Maintainability** - Easy to understand impact of changes
- **Testability** - Each component independently testable
- **Reusability** - Engines shared across Managers, Managers shared across Clients

### Learn More

- [Righting Software Official Site](https://rightingsoftware.org/)
- [Book: Righting Software by Juval Löwy](https://www.amazon.com/Righting-Software-Juval-L%C3%B6wy/dp/0136524036)
- See `docs/righting-software-analysis.md` for detailed architectural analysis of IrieBook

## License

MIT - Use it, fix your books, be happy!

## Credits

Built with 🎺 by Andrei for his wife's vampire novels.

Architecture refactored to the Righting Software Method through 6 phases of systematic transformation.

Generated with help from [Claude Code](https://claude.com/claude-code).

---

*"No woman, no cry... but dis woman need proper quotes!"* 🎵
