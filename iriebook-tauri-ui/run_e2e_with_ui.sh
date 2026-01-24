#!/usr/bin/env bash
set -euo pipefail

# Launch ALL Selenium E2E tests with visible UI in slow mode.
# SLOW_MODE adds delays so you can see what's happening.
# KEEP_OPEN keeps the window open at the end for manual inspection.

echo "=========================================="
echo "E2E Tests - Visible UI Mode"
echo "=========================================="
echo ""
echo "Options:"
echo "  SLOW_MODE=false   - Run without delays (fast)"
echo "  KEEP_OPEN=true    - Keep window open after test"
echo ""
echo "Examples:"
echo "  ./run_e2e_with_ui.sh                    # Slow with auto-close"
echo "  KEEP_OPEN=true ./run_e2e_with_ui.sh     # Slow and stay open"
echo "  SLOW_MODE=false ./run_e2e_with_ui.sh    # Fast mode"
echo ""
echo "=========================================="
echo ""

# Default to slow mode so you can see what's happening
export SLOW_MODE=${SLOW_MODE:-true}
export KEEP_OPEN=${KEEP_OPEN:-false}

# Directory setup
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# List all e2e test scripts to run (in order)
# Add new e2e tests to this array as they are created
E2E_SCRIPTS=(
  "$SCRIPT_DIR/scripts/e2e-diff-test.js"
  "$SCRIPT_DIR/scripts/e2e-google-auth-test.js"
)

TOTAL_TESTS=${#E2E_SCRIPTS[@]}
PASSED=0
FAILED=0
FAILED_TESTS=()

echo "Found $TOTAL_TESTS e2e test(s) to run"
echo ""

# Run each test
for test_script in "${E2E_SCRIPTS[@]}"; do
  if [[ ! -f "$test_script" ]]; then
    echo "⚠️  Skipping missing test: $(basename "$test_script")"
    continue
  fi

  test_name=$(basename "$test_script" .js)
  echo "=========================================="
  echo "Running: $test_name"
  echo "=========================================="
  echo ""

  # Run in a subshell to isolate any hanging file descriptors or processes
  (
    node "$test_script"
  )
  EXIT_CODE=$?

  if [[ $EXIT_CODE -eq 0 ]]; then
    echo ""
    echo "✅ PASSED: $test_name"
    echo ""
    PASSED=$((PASSED + 1))
  else
    echo ""
    echo "❌ FAILED: $test_name"
    echo ""
    FAILED=$((FAILED + 1))
    FAILED_TESTS+=("$test_name")
  fi

  # Small delay to ensure all cleanup is done
  sleep 1
done

# Print summary
echo "=========================================="
echo "E2E Test Summary"
echo "=========================================="
echo "Total:  $TOTAL_TESTS"
echo "Passed: $PASSED"
echo "Failed: $FAILED"
echo ""

if [[ $FAILED -gt 0 ]]; then
  echo "Failed tests:"
  for test in "${FAILED_TESTS[@]}"; do
    echo "  - $test"
  done
  echo ""
  exit 1
else
  echo "🎉 All tests passed!"
  echo ""
  exit 0
fi
