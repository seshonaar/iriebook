# Nom Parser Combinator Guide

## Table of Contents
1. [What is Nom?](#what-is-nom)
2. [Core Concepts](#core-concepts)
3. [Common Combinators](#common-combinators)
4. [Application in markdown_transform.rs](#application-in-markdown_transformrs)
5. [Annotated Code Examples](#annotated-code-examples)
6. [Benefits](#benefits)
7. [Learning Resources](#learning-resources)

---

## What is Nom?

Nom is a **parser combinator library** for Rust. It allows you to build complex parsers by composing simple, reusable parsing functions (called "combinators"). Instead of writing a monolithic parser with complex state management, you combine small parsers to handle specific patterns.

### Key Philosophy

- **Composability**: Small parsers are combined to build larger ones
- **Type Safety**: Rust's type system ensures parser correctness at compile time
- **Zero-Copy**: Nom parsers work with string slices, avoiding unnecessary allocations
- **Error Handling**: Rich error reporting with context

---

## Core Concepts

### IResult

The foundation of nom parsers is the `IResult` type:

```rust
type IResult<I, O> = Result<(I, O), nom::Err<E>>;
```

- **`I`** (Input): The remaining input after parsing (e.g., `&str`)
- **`O`** (Output): The parsed value
- **Success**: Returns `Ok((remaining_input, parsed_output))`
- **Failure**: Returns `Err(nom::Err)` with error details

**Example:**
```rust
fn parse_number(input: &str) -> IResult<&str, u32> {
    // If input = "42 apples"
    // Returns: Ok((" apples", 42))
    //           ^remaining  ^parsed
}
```

### Parser Combinators

A **combinator** is a function that:
1. Takes one or more parsers as input
2. Returns a new parser that combines their behavior

**Example:**
```rust
// `tag` matches a specific string
let parse_hello = tag("hello");

// `preceded` runs two parsers, returns the second result
let parse_name = preceded(tag("Name: "), take_while(char::is_alphabetic));

// Input: "Name: Alice"
// Output: "Alice"
```

---

## Common Combinators

### Bytes Combinators (`bytes::complete`)

#### `tag(pattern)`
Matches an exact string.

```rust
let (remaining, matched) = tag("Chapter")("Chapter 5").unwrap();
// remaining = " 5"
// matched = "Chapter"
```

#### `take_until(pattern)`
Consumes input until a pattern is found (does not consume the pattern).

```rust
let (remaining, matched) = take_until(" - ")("Title - Subtitle").unwrap();
// remaining = " - Subtitle"
// matched = "Title"
```

### Sequence Combinators (`sequence`)

#### `tuple((parser1, parser2, ...))`
Runs multiple parsers in sequence, returns all results as a tuple.

```rust
let parser = tuple((tag("Chapter"), tag(" "), digit1));
let (remaining, (ch, space, num)) = parser("Chapter 5").unwrap();
// ch = "Chapter", space = " ", num = "5"
```

#### `preceded(prefix_parser, value_parser)`
Runs two parsers, returns only the second result.

```rust
let parser = preceded(tag("Title: "), rest);
let (_, title) = parser("Title: My Book").unwrap();
// title = "My Book"
```

### Branch Combinators (`branch`)

#### `alt((parser1, parser2, ...))`
Tries parsers in order, returns the first successful match.

```rust
let parser = alt((
    tag(" – "),  // Unicode en-dash
    tag(" — "),  // Unicode em-dash
    tag(" - "),  // ASCII hyphen
));

let (remaining, matched) = parser(" – subtitle").unwrap();
// matched = " – "
```

### Combinator (`combinator`)

#### `rest`
Consumes and returns all remaining input.

```rust
let (_, subtitle) = preceded(tag(" - "), rest)("Title - Subtitle").unwrap();
// subtitle = "Subtitle"
```

---

## Application in markdown_transform.rs

### Problem: Parsing Chapter Headings

We need to parse chapter headings with various dash styles:

1. **Unicode en-dash**: `## Chapter 1 – The Beginning`
2. **Unicode em-dash**: `## Chapter 1 — The Beginning`
3. **Hyphen with spaces**: `## Chapter 1 - The Beginning`
4. **Hyphen no spaces**: `## Chapter1-The Beginning`

### Solution: Nom Combinators

We create separate parsers for each pattern and combine them with `alt`:

```rust
use nom::{
    IResult,
    branch::alt,
    bytes::complete::{tag, take_until},
    combinator::rest,
    sequence::{preceded, tuple},
};

/// Parse chapter heading with Unicode en-dash (–) separator
fn parse_unicode_endash(input: &str) -> IResult<&str, (&str, &str)> {
    let (remaining, (prefix, subtitle)) = tuple((
        take_until(" – "),      // Match everything before " – "
        preceded(tag(" – "), rest),  // Match and skip " – ", return rest
    ))(input)?;

    Ok((remaining, (prefix, subtitle)))
}

/// Try all dash patterns in order
fn parse_chapter_with_dash(input: &str) -> IResult<&str, (&str, &str)> {
    alt((
        parse_unicode_endash,
        parse_unicode_emdash,
        parse_hyphen_with_spaces,
        parse_hyphen_no_spaces,
    ))(input)
}
```

---

## Annotated Code Examples

### Example 1: Simple Tag Matching

```rust
use nom::bytes::complete::tag;

fn parse_chapter_prefix(input: &str) -> IResult<&str, &str> {
    tag("Chapter")(input)
}

// Usage:
let (remaining, matched) = parse_chapter_prefix("Chapter 5").unwrap();
assert_eq!(matched, "Chapter");
assert_eq!(remaining, " 5");
```

**Breakdown:**
- `tag("Chapter")` creates a parser
- The parser is called with `(input)`
- Returns `Ok((remaining, matched))` on success

---

### Example 2: Tuple + Preceded

```rust
use nom::{
    bytes::complete::{tag, take_until},
    combinator::rest,
    sequence::{preceded, tuple},
};

fn parse_title_subtitle(input: &str) -> IResult<&str, (&str, &str)> {
    tuple((
        take_until(" - "),           // 1. Match title before dash
        preceded(tag(" - "), rest),  // 2. Skip dash, return subtitle
    ))(input)
}

// Usage:
let (_, (title, subtitle)) = parse_title_subtitle("MyBook - TheStory").unwrap();
assert_eq!(title, "MyBook");
assert_eq!(subtitle, "TheStory");
```

**Breakdown:**
1. `take_until(" - ")` matches "MyBook", stops at " - "
2. `preceded(tag(" - "), rest)`:
   - `tag(" - ")` matches and skips the dash
   - `rest` returns everything after: "TheStory"
3. `tuple` combines results: `(title, subtitle)`

---

### Example 3: Alt (Try Multiple Patterns)

```rust
use nom::{
    branch::alt,
    bytes::complete::tag,
};

fn parse_dash(input: &str) -> IResult<&str, &str> {
    alt((
        tag(" – "),  // Try en-dash first
        tag(" — "),  // Then em-dash
        tag(" - "),  // Finally ASCII hyphen
    ))(input)
}

// Usage examples:
let (_, dash1) = parse_dash(" – subtitle").unwrap();
assert_eq!(dash1, " – ");

let (_, dash2) = parse_dash(" - subtitle").unwrap();
assert_eq!(dash2, " - ");
```

**Breakdown:**
- `alt` tries each parser in order
- Returns the first successful match
- If all fail, returns an error

---

### Example 4: Full Chapter Heading Parser (From Our Code)

```rust
/// Parse chapter heading content and extract title and optional subtitle
fn parse_chapter_content(rest: &str) -> (String, Option<String>) {
    // Try to parse with dash patterns using nom
    if let Ok((_, (prefix, subtitle))) = parse_chapter_with_dash(rest) {
        // Extract prefix with number (e.g., "Chapter 1")
        if let Some(title) = extract_prefix_with_number(prefix) {
            let processed_subtitle = clean_subtitle(subtitle);

            // Only keep subtitle if it doesn't look like a paragraph
            if should_keep_as_subtitle(&processed_subtitle) {
                return (title, Some(processed_subtitle));
            }

            // Subtitle rejected, check if empty (e.g., "Chapter 1 -")
            if processed_subtitle.is_empty() {
                return (title, None);
            }
        }
    }

    // Try space-separated pattern: "Chapter 1 The Beginning"
    let words: Vec<&str> = rest.split_whitespace().collect();
    if words.len() >= 3 && words[1].parse::<u32>().is_ok() {
        let prefix = format!("{} {}", words[0], words[1]);
        let subtitle_clean = clean_subtitle(&words[2..].join(" "));

        if should_keep_as_subtitle(&subtitle_clean) {
            return (prefix, Some(subtitle_clean));
        }
    }

    // No valid pattern found, return the whole thing as title
    (rest.to_string(), None)
}
```

**Breakdown:**
1. **Line 4**: Use nom's `parse_chapter_with_dash` to try all dash patterns
2. **Line 5**: If successful, extract `(prefix, subtitle)` tuple
3. **Line 7**: Apply business logic (extract number, clean subtitle)
4. **Line 17**: Fallback to space-separated pattern if nom parsing fails
5. **Line 28**: Final fallback: return entire input as title

---

## Benefits

### 1. Composability
Build complex parsers from simple building blocks:

```rust
// Simple parsers
let parse_prefix = take_until(" - ");
let parse_dash = tag(" - ");
let parse_subtitle = rest;

// Composed parser
let parse_chapter = tuple((parse_prefix, parse_dash, parse_subtitle));
```

### 2. Readability
Parser intent is clear from combinator names:

```rust
// Manual parsing (unclear intent):
if let Some(pos) = input.find(" - ") {
    let prefix = &input[..pos];
    let subtitle = &input[pos + 3..];
}

// Nom combinator (clear intent):
tuple((take_until(" - "), preceded(tag(" - "), rest)))
```

### 3. Reusability
Parsers are first-class functions, easy to reuse:

```rust
let parse_unicode_dash = alt((tag(" – "), tag(" — ")));

// Reuse in different contexts:
let parse_chapter_dash = tuple((take_until_dash, preceded(parse_unicode_dash, rest)));
let parse_section_dash = preceded(parse_unicode_dash, alphanumeric1);
```

### 4. Type Safety
Compiler enforces correct parser composition:

```rust
// Compiler error: type mismatch
let bad_parser = tuple((
    tag("hello"),  // Returns &str
    digit1,        // Returns &str
    value(42),     // Returns u32 <- incompatible!
));
```

### 5. Testing
Parsers are pure functions, easy to test:

```rust
#[test]
fn test_parse_unicode_endash() {
    let (_, (prefix, subtitle)) = parse_unicode_endash("Title – Sub").unwrap();
    assert_eq!(prefix, "Title");
    assert_eq!(subtitle, "Sub");
}
```

---

## Learning Resources

### Official Documentation
- [Nom GitHub](https://github.com/rust-bakery/nom)
- [Nom Docs (docs.rs)](https://docs.rs/nom)
- [Nom Book (WIP)](https://github.com/rust-bakery/nom/tree/main/doc)

### Tutorials
- [Nom Tutorial by Geal](https://blog.logrocket.com/parsing-in-rust-with-nom/)
- [Making a Calculator with Nom](https://github.com/Geal/nom/blob/main/doc/making_a_new_parser_from_scratch.md)
- [Nom Recipes](https://github.com/rust-bakery/nom/blob/main/doc/nom_recipes.md)

### Examples
- [Nom Examples Directory](https://github.com/rust-bakery/nom/tree/main/examples)
- [JSON Parser Example](https://github.com/rust-bakery/nom/blob/main/examples/json.rs)
- [INI Parser Example](https://github.com/rust-bakery/nom/blob/main/examples/ini.rs)

### Videos
- [Crust of Rust: Parsing with Nom (Jon Gjengset)](https://www.youtube.com/watch?v=BWm9sAuMkx0)

---

## Summary

Nom provides a powerful, composable approach to parsing:

1. **Build small parsers** for specific patterns
2. **Compose them** using combinators like `tuple`, `alt`, `preceded`
3. **Get type-safe** parsing with rich error handling
4. **Test easily** with pure functions

In `markdown_transform.rs`, nom handles the complex chapter heading patterns elegantly, making the code more maintainable and extensible than manual string manipulation.

**Key Takeaway**: Instead of writing imperative parsing logic with loops and conditionals, nom lets you *declare* what you want to parse, and the library handles the mechanics.
