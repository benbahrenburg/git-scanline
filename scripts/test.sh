#!/usr/bin/env bash
# Run the full git-scanline test suite.
#
# Usage:
#   ./scripts/test.sh [options]
#
# Options:
#   --unit-only        Skip tests that require TEST_REPO_PATH
#   --verbose          Show stdout/stderr from each test (--nocapture)
#   --no-fmt           Skip cargo fmt --check
#   --no-clippy        Skip cargo clippy
#   -h, --help         Show this message
#
# Integration tests (test_parse_log_real_repo, test_full_pipeline_scores_in_range)
# run automatically when TEST_REPO_PATH is set in the environment or in a .env
# file at the workspace root. They are skipped gracefully when it is not set.
#
# Example — run everything with a real repo:
#
#   TEST_REPO_PATH=/path/to/any-git-repo ./scripts/test.sh
#
# Example — unit tests only, no linting:
#
#   ./scripts/test.sh --unit-only --no-fmt --no-clippy

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"

# ── Defaults ──────────────────────────────────────────────────────────────────

UNIT_ONLY=false
VERBOSE=false
RUN_FMT=true
RUN_CLIPPY=true

# ── Argument parsing ──────────────────────────────────────────────────────────

while [[ $# -gt 0 ]]; do
  case "$1" in
    --unit-only)   UNIT_ONLY=true ;;
    --verbose)     VERBOSE=true ;;
    --no-fmt)      RUN_FMT=false ;;
    --no-clippy)   RUN_CLIPPY=false ;;
    -h|--help)
      sed -n '2,/^[^#]/{ /^#/!q; s/^# \{0,1\}//; p }' "$0"
      exit 0
      ;;
    *)
      echo "Unknown option: $1" >&2
      echo "Run '$0 --help' for usage." >&2
      exit 1
      ;;
  esac
  shift
done

# ── Helpers ───────────────────────────────────────────────────────────────────

PASS=0
FAIL=0
SKIP=0

step() { echo; echo "━━  $*"; }
ok()   { echo "  ✓  $*"; PASS=$((PASS + 1)); }
fail() { echo "  ✗  $*"; FAIL=$((FAIL + 1)); }
skip() { echo "  –  $*"; SKIP=$((SKIP + 1)); }

# ── Change to workspace root ──────────────────────────────────────────────────

cd "$ROOT"

# ── Load .env if present (sets TEST_REPO_PATH for integration tests) ──────────

if [[ -f .env ]]; then
  # Export only lines that look like KEY=VALUE (ignore comments and blanks)
  set -a
  # shellcheck disable=SC1091
  source <(grep -E '^[A-Za-z_][A-Za-z0-9_]*=' .env)
  set +a
fi

# ── fmt check ─────────────────────────────────────────────────────────────────

if $RUN_FMT; then
  step "cargo fmt --check"
  if cargo fmt --check 2>&1; then
    ok "Formatting"
  else
    fail "Formatting — run 'cargo fmt' to fix"
  fi
else
  skip "fmt check (--no-fmt)"
fi

# ── clippy ────────────────────────────────────────────────────────────────────

if $RUN_CLIPPY; then
  step "cargo clippy -- -D warnings"
  if cargo clippy -- -D warnings 2>&1; then
    ok "Clippy"
  else
    fail "Clippy"
  fi
else
  skip "clippy (--no-clippy)"
fi

# ── Unit + integration tests ──────────────────────────────────────────────────

step "cargo test"

if [[ -n "${TEST_REPO_PATH:-}" ]]; then
  echo "  Using TEST_REPO_PATH=${TEST_REPO_PATH}"
else
  echo "  TEST_REPO_PATH not set — integration tests will be skipped"
fi

CARGO_TEST_ARGS=""
if $UNIT_ONLY; then
  CARGO_TEST_ARGS="--lib"
fi
CARGO_TEST_EXTRA=""
if $VERBOSE; then
  CARGO_TEST_EXTRA="-- --nocapture"
fi

# shellcheck disable=SC2086
if cargo test $CARGO_TEST_ARGS $CARGO_TEST_EXTRA 2>&1; then
  ok "Tests"
else
  fail "Tests"
fi

# ── Summary ───────────────────────────────────────────────────────────────────

echo
echo "━━  Results: ${PASS} passed · ${FAIL} failed · ${SKIP} skipped"
echo

if [[ $FAIL -gt 0 ]]; then
  exit 1
fi
