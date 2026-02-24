#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'EOF'
Build git-scanline for multiple Rust targets.

Usage:
  ./scripts/build-targets.sh [--debug] [--no-install-targets] [--matrix] [target...]

Options:
  --debug            Build with debug profile (default is release)
  --no-install-targets
                     Do not install missing Rust targets automatically
  --matrix           Use broad cross-platform default matrix
                     (macOS + Linux + Windows GNU)
  -h, --help         Show this help

Examples:
  ./scripts/build-targets.sh
  ./scripts/build-targets.sh --matrix
  ./scripts/build-targets.sh --no-install-targets
  ./scripts/build-targets.sh x86_64-unknown-linux-gnu aarch64-unknown-linux-gnu
  ./scripts/build-targets.sh --debug aarch64-apple-darwin

Defaults (if no target args are provided):
  Host-safe set (auto-detected), e.g. on Apple Silicon:
  aarch64-apple-darwin
  x86_64-apple-darwin

Matrix defaults (with --matrix):
  x86_64-apple-darwin
  aarch64-apple-darwin
  x86_64-unknown-linux-gnu
  aarch64-unknown-linux-gnu
  x86_64-pc-windows-gnu
EOF
}

if ! command -v cargo >/dev/null 2>&1; then
  echo "Error: cargo is not installed or not on PATH." >&2
  exit 1
fi

if ! command -v rustup >/dev/null 2>&1; then
  echo "Error: rustup is not installed or not on PATH." >&2
  exit 1
fi

profile="release"
install_targets=true
use_matrix_defaults=false

declare -a targets=()

while [[ $# -gt 0 ]]; do
  case "$1" in
    --debug)
      profile="debug"
      shift
      ;;
    --no-install-targets)
      install_targets=false
      shift
      ;;
    --matrix)
      use_matrix_defaults=true
      shift
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    --*)
      echo "Error: unknown option '$1'" >&2
      usage
      exit 1
      ;;
    *)
      targets+=("$1")
      shift
      ;;
  esac
done

host_target="$(rustc -vV | sed -n 's/^host: //p')"

if [[ ${#targets[@]} -eq 0 ]]; then
  if [[ "$use_matrix_defaults" == true ]]; then
    targets=(
      "x86_64-apple-darwin"
      "aarch64-apple-darwin"
      "x86_64-unknown-linux-gnu"
      "aarch64-unknown-linux-gnu"
      "x86_64-pc-windows-gnu"
    )
  else
    case "$host_target" in
      aarch64-apple-darwin)
        targets=("aarch64-apple-darwin" "x86_64-apple-darwin")
        ;;
      x86_64-apple-darwin)
        targets=("x86_64-apple-darwin" "aarch64-apple-darwin")
        ;;
      *)
        targets=("$host_target")
        ;;
    esac
  fi
fi

installed_targets="$(rustup target list --installed)"
declare -a missing_targets=()
for target in "${targets[@]}"; do
  if ! grep -qx "$target" <<< "$installed_targets"; then
    missing_targets+=("$target")
  fi
done

if [[ ${#missing_targets[@]} -gt 0 ]]; then
  if [[ "$install_targets" == true ]]; then
    echo "Installing missing targets: ${missing_targets[*]}"
    rustup target add "${missing_targets[@]}"
  else
    echo "Error: missing Rust target(s): ${missing_targets[*]}" >&2
    echo "Hint: rerun without --no-install-targets or run:" >&2
    echo "  rustup target add ${missing_targets[*]}" >&2
    exit 2
  fi
fi

build_args=()
if [[ "$profile" == "release" ]]; then
  build_args+=(--release)
fi

echo "Building profile: $profile"
echo "Host target: $host_target"
for target in "${targets[@]}"; do
  echo "------------------------------------------------------------"
  echo "Building target: $target"
  cargo build "${build_args[@]}" --target "$target"
  echo "Output: target/$target/$profile/git-scanline"
  if [[ "$target" == *"windows"* ]]; then
    echo "Output (Windows): target/$target/$profile/git-scanline.exe"
  fi
done

echo "------------------------------------------------------------"
echo "Done. Built ${#targets[@]} target(s)."
