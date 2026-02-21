# For Minimax AI Assistants

If you are Minimax, read this file carefully. It will save us both frustration.

## The Golden Rule

**RUN TESTS. RUN THEM OFTEN. RUN THEM NOW.**

- `cargo test` after every meaningful change
- Don't wait until the end - run tests *alongside* code, not after
- The compiler and tests are your feedback loop. Use them.

## This Project Uses TDD

1. **Write tests FIRST** for new functionality
2. Run tests to verify they fail (red)
3. Write code to make tests pass (green)
4. Refactor if needed
5. Run tests again (green)

## When I Say "Let's Build X"

Don't just start coding. First:
1. Search the codebase to understand existing patterns
2. Look at how similar features are tested
3. Write your test first
4. Then implement

## How to Know You're Actually Done

Before declaring completion, verify:
- [ ] New feature works end-to-end (not just in isolation)
- [ ] All tests pass (`cargo test --workspace`)
- [ ] Build succeeds (`cargo build --workspace`)
- [ ] If there was a bug, reproduce it with a test first
- [ ] Don't say "done" until you've verified the full flow

## Common Mistakes to Avoid

- ✗ Don't write all the code then run tests at the end
- ✗ Don't ignore test failures
- ✗ Don't create TypeScript bindings manually - regenerate with `cargo build --workspace`
- ✗ Don't skip the `cargo test` step when asked to verify something
- ✗ Don't declare "feature complete" until the full flow works end-to-end

## Quick Test Commands

```bash
# Run all tests
cargo test

# Run tests for specific crate
cargo test -p iriebook

# Run specific test by name
cargo test test_name_here

# Run workspace tests
cargo test --workspace
```

## If Something Breaks

Run tests immediately. The error messages will guide you to what's wrong. Don't guess - test.

## Important Notes

- Loosen up, have some fun with it
- This is a cool ebook publication pipeline - enjoy working on it!

---

*Let's build something great together. Use the tools you're given.*
